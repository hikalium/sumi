//! マスクとヒットの生成。
//!
//! 走査で常に除く箇所は 3 つある。コードフェンスとインラインコード (識別子を散文から分けるため)、
//! 避ける語を「」で囲んで引用しているスパン、`keep` に挙げた語の中に収まるヒット、である。
//!
//! `str::lines` で行に分け、フェンスは文書全体で、code span は行ごとに判定する。

use crate::rules::Rule;

/// インラインコードの区切りとして受け付けるバッククォートの連なりの長さの上限。3 つ以上が
/// 行の途中にあればフェンス記号への言及として扱う。
const MAX_SPAN_TICKS: usize = 2;

/// インラインコード (`` `...` ``) の中にある文字の位置。N <= `MAX_SPAN_TICKS` 個のバッククォートの
/// 連なりで開き、次にちょうど N 個の連なりが来たら閉じる。同じ行の中でだけ対を作り、
/// 対を持たない連なりを通常の文字として扱う。
pub fn code_span_mask(chars: &[char]) -> Vec<bool> {
    let n = chars.len();
    let mut mask = vec![false; n];
    let run_len = |start: usize| {
        let mut k = start;
        while k < n && chars[k] == '`' {
            k += 1;
        }
        k - start
    };
    let mut i = 0;
    while i < n {
        if chars[i] != '`' {
            i += 1;
            continue;
        }
        let open = run_len(i);
        if open > MAX_SPAN_TICKS {
            i += open; // フェンス記号への言及として扱い、周りの散文を検査対象に残す
            continue;
        }
        // 同じ長さの閉じの連なりを行末までに探す
        let mut j = i + open;
        let mut close = None;
        while j < n && chars[j] != '\n' {
            if chars[j] == '`' {
                let r = run_len(j);
                if r == open {
                    close = Some(j + r);
                    break;
                }
                j += r;
            } else {
                j += 1;
            }
        }
        match close {
            Some(end) => {
                mask[i..end].fill(true);
                i = end;
            }
            None => i += open, // 対を持たない連なりを通常の文字として扱う
        }
    }
    mask
}

/// 各行がフェンスの中 (``` の行自身を含む) かどうか。
pub fn fenced_lines(lines: &[&str]) -> Vec<bool> {
    let mut fenced = vec![false; lines.len()];
    let mut f = false;
    for (i, l) in lines.iter().enumerate() {
        if l.trim_start().starts_with("```") {
            f = !f;
            fenced[i] = true;
        } else {
            fenced[i] = f;
        }
    }
    fenced
}

/// 「」で引用された文字の位置 (行ごと)。括弧の内側だけを除外にする。引用は同じ段落の中なら
/// 行をまたぐ。段落は空行 (空白だけの行) とフェンスの行で切れ、フェンスの中を段落から除く。
/// 入れ子は深さで数え、段落の終わりまでに閉じた対だけを除外に使う (閉じ忘れた場合にも本文を
/// 検査対象に残すため)。対の無い」は無視する。
pub fn quote_masks(lines: &[Vec<char>], fenced: &[bool]) -> Vec<Vec<bool>> {
    let mut masks: Vec<Vec<bool>> = lines.iter().map(|l| vec![false; l.len()]).collect();
    let boundary = |i: usize| fenced[i] || lines[i].iter().all(|c| c.is_whitespace());
    let mut open: Vec<(usize, usize)> = Vec::new();
    for i in 0..lines.len() {
        if boundary(i) {
            open.clear(); // 段落の終わり: 閉じていない「は捨てる
            continue;
        }
        for (k, &c) in lines[i].iter().enumerate() {
            if c == '「' {
                open.push((i, k));
            } else if c == '」'
                && let Some((sl, sk)) = open.pop()
            {
                if sl == i {
                    masks[i][sk + 1..k].fill(true);
                } else {
                    masks[sl][sk + 1..].fill(true);
                    for m in masks.iter_mut().take(i).skip(sl + 1) {
                        m.fill(true);
                    }
                    masks[i][..k].fill(true);
                }
            }
        }
    }
    masks
}

/// `needle` が `hay` に現れる位置 [start, end) の全て (重なりも含む)。空の needle なら空の結果を返す。
pub fn find_all(hay: &[char], needle: &[char]) -> Vec<(usize, usize)> {
    let mut out = Vec::new();
    if needle.is_empty() || hay.len() < needle.len() {
        return out;
    }
    for start in 0..=hay.len() - needle.len() {
        if hay[start..start + needle.len()] == *needle {
            out.push((start, start + needle.len()));
        }
    }
    out
}

/// 1 文書を行に割り、除外のマスクを付けたもの。
pub struct Document {
    /// 各行の文字
    pub lines: Vec<Vec<char>>,
    /// 各行がフェンスの中か
    pub fenced: Vec<bool>,
    /// 各行の各文字が code span の中か
    pub code: Vec<Vec<bool>>,
    /// 各行の各文字が「」引用の中か
    pub quoted: Vec<Vec<bool>>,
}

impl Document {
    pub fn parse(body: &str) -> Document {
        let raw: Vec<&str> = body.lines().collect();
        let fenced = fenced_lines(&raw);
        let lines: Vec<Vec<char>> = raw.iter().map(|l| l.chars().collect()).collect();
        let code = lines.iter().map(|c| code_span_mask(c)).collect();
        let quoted = quote_masks(&lines, &fenced);
        Document {
            lines,
            fenced,
            code,
            quoted,
        }
    }

    /// 行 `line` の [start, end) が検査の対象か (フェンス外で、code span も「」引用も含まない)。
    pub fn visible(&self, line: usize, start: usize, end: usize) -> bool {
        !self.fenced[line] && !(start..end).any(|i| self.code[line][i] || self.quoted[line][i])
    }

    /// `--mask` の出力。各行の各文字について、フェンス内の行か code span なら '1'、
    /// それ以外なら '0'。「」引用の内側も、この条件に従う。
    pub fn mask_lines(&self) -> Vec<String> {
        self.lines
            .iter()
            .enumerate()
            .map(|(i, chars)| {
                chars
                    .iter()
                    .enumerate()
                    .map(|(k, _)| {
                        if self.fenced[i] || self.code[i][k] {
                            '1'
                        } else {
                            '0'
                        }
                    })
                    .collect()
            })
            .collect()
    }
}

/// 規則のヒット 1 件。`line` と `col` は 1 始まりで、`col` は文字 (char) 単位。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Hit<'a> {
    pub line: usize,
    pub col: usize,
    pub matched: String,
    pub rule: &'a Rule,
}

impl Hit<'_> {
    /// `LINE:COL: 「avoid」-> write1 / write2` の形。
    pub fn render(&self) -> String {
        format!(
            "{}:{}: 「{}」-> {}",
            self.line,
            self.col,
            self.matched,
            self.rule.write.join(" / ")
        )
    }
}

/// 1 つの規則を 1 行に当てる。avoid の複数の文字列が重なって当たるときは長い方を 1 件にまとめ、
/// keep の語の出現に収まるヒットを除く。返す位置は start の順。
fn rule_hits_in_line(doc: &Document, line: usize, rule: &Rule) -> Vec<(usize, usize)> {
    let chars = &doc.lines[line];
    let mut candidates: Vec<(usize, usize)> = Vec::new();
    for avoid in rule.avoid {
        let needle: Vec<char> = avoid.chars().collect();
        for (start, end) in find_all(chars, &needle) {
            if doc.visible(line, start, end) {
                candidates.push((start, end));
            }
        }
    }
    let keeps: Vec<(usize, usize)> = rule
        .keep
        .iter()
        .flat_map(|k| {
            let needle: Vec<char> = k.chars().collect();
            find_all(chars, &needle)
        })
        .collect();
    candidates.retain(|&(s, e)| !keeps.iter().any(|&(ks, ke)| ks <= s && e <= ke));
    // 長い順 (同じ長さなら前の方) に取り、既に取ったヒットと重なるものは落とす
    candidates.sort_by_key(|&(s, e)| (std::cmp::Reverse(e - s), s));
    let mut accepted: Vec<(usize, usize)> = Vec::new();
    for (s, e) in candidates {
        if !accepted.iter().any(|&(a, b)| s < b && a < e) {
            accepted.push((s, e));
        }
    }
    accepted.sort_unstable();
    accepted
}

/// 文書に規則を当て、ヒットを行、列、規則の順で返す。
#[cfg(test)]
pub fn scan<'a>(body: &str, rules: &'a [Rule]) -> Vec<Hit<'a>> {
    scan_doc(&Document::parse(body), rules)
}

/// 解析した文書に規則を当て、ヒットを行、列、規則の順で返す。
pub fn scan_doc<'a>(doc: &Document, rules: &'a [Rule]) -> Vec<Hit<'a>> {
    let mut hits: Vec<(usize, usize, usize, Hit<'a>)> = Vec::new();
    for line in 0..doc.lines.len() {
        for (index, rule) in rules.iter().enumerate() {
            for (start, end) in rule_hits_in_line(doc, line, rule) {
                let matched: String = doc.lines[line][start..end].iter().collect();
                hits.push((
                    line,
                    start,
                    index,
                    Hit {
                        line: line + 1,
                        col: start + 1,
                        matched,
                        rule,
                    },
                ));
            }
        }
    }
    // 別の規則のヒットに収まるヒットを除く (`停止窓` の中の `窓` のように、ある規則の avoid が別の規則の
    // avoid を含むとき、長い方の規則だけを出す)。範囲が同じヒットは avoid の一意性により 1 つに限られる
    let ranges: Vec<(usize, usize, usize)> = hits
        .iter()
        .map(|(line, start, _, h)| (*line, *start, start + h.matched.chars().count()))
        .collect();
    let contained = |k: usize| {
        let (line, s, e) = ranges[k];
        ranges
            .iter()
            .enumerate()
            .any(|(j, &(l, a, b))| j != k && l == line && a <= s && e <= b && (b - a) > (e - s))
    };
    let mut kept: Vec<(usize, usize, usize, Hit<'a>)> = Vec::new();
    for (k, hit) in hits.into_iter().enumerate() {
        if !contained(k) {
            kept.push(hit);
        }
    }
    kept.sort_by_key(|&(line, start, index, _)| (line, start, index));
    kept.into_iter().map(|(_, _, _, h)| h).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn mask_str(s: &str) -> String {
        let chars: Vec<char> = s.chars().collect();
        code_span_mask(&chars)
            .iter()
            .map(|&b| if b { '1' } else { '0' })
            .collect()
    }

    /// docgate.rs のテストにある例と同じ判定になること。
    #[test]
    fn code_span_mask_matches_docgate_examples() {
        // 1 つのバッククォートの対
        assert_eq!(
            mask_str("run `read_events` and `**x**`"),
            "00001111111111111000001111111"
        );
        // 対にならない ``` は文字そのもので、後ろの散文は見えたまま
        assert_eq!(
            mask_str("note ``` fence then **bold** here"),
            "0".repeat(33)
        );
        // 2 つのバッククォートの対は同じ距離を隠す
        assert_eq!(
            mask_str("a ``x **y** z`` b **c** d"),
            "0011111111111110000000000"
        );
        // 行の途中で ``` を 2 度言及しても、その間の散文を検査対象に残す (docs/design/TOOLS.md の edit 行)
        let row = "| `edit` | ``` ブロックで置換。旧テキストは**再現させない**。**SEARCH 指定**: a fenced ``` block of markers |";
        let m = mask_str(row);
        let chars: Vec<char> = row.chars().collect();
        assert!(m.starts_with("0011111100"));
        for (k, c) in chars.iter().enumerate() {
            if *c == '再' || *c == 'S' || *c == 'b' {
                assert_eq!(m.as_bytes()[k], b'0', "{row}[{k}]");
            }
        }
        // 対の無い 1 つのバッククォートは文字そのもの
        assert_eq!(mask_str("a ` b"), "00000");
        // 1 つの ` に対応する閉じ記号も 1 つの ` に限る
        assert_eq!(mask_str("a `x`` y` b"), "00111111100");
    }

    #[test]
    fn fenced_lines_matches_docgate_examples() {
        let doc = "# H\n**Goal** text\n```\n**not touched**\n```\nplain _x_\n";
        let lines: Vec<&str> = doc.lines().collect();
        assert_eq!(
            fenced_lines(&lines),
            vec![false, false, true, true, true, false]
        );
        // 字下げしたフェンスも、閉じないフェンスも docgate と同じ
        let lines: Vec<&str> = "a\n  ```rust\nb\n".lines().collect();
        assert_eq!(fenced_lines(&lines), vec![false, true, true]);
        assert_eq!(
            Document::parse("t\n```\n**kept**\n```\nu\n").mask_lines(),
            vec!["0", "111", "11111111", "111", "0"]
        );
    }

    #[test]
    fn quote_mask_excludes_only_the_inside_of_closed_pairs() {
        let q = |s: &str| -> Vec<String> {
            let doc = Document::parse(s);
            doc.quoted
                .iter()
                .map(|m| m.iter().map(|&b| if b { '1' } else { '0' }).collect())
                .collect()
        };
        assert_eq!(q("a「bc」d"), vec!["001100"]);
        // 入れ子は深さで数える
        assert_eq!(q("「a「b」c」"), vec!["0111110"]);
        // 閉じた対の内側だけを除外し、片方だけの括弧を通常の文字として扱う
        assert_eq!(q("「ab"), vec!["000"]);
        assert_eq!(q("ab」「c」"), vec!["000010"]);
        // 同じ段落なら行をまたぐ (2 行目全体と 3 行目の」の手前まで)
        assert_eq!(q("a「b\ncd\ne」f\n"), vec!["001", "11", "100"]);
        // 空行は段落を切るので、またぐ引用も検査対象に残す
        assert_eq!(q("a「b\n\ne」f\n"), vec!["000", "", "000"]);
        assert_eq!(q("a「b\n  \ne」f\n"), vec!["000", "00", "000"]);
        // フェンスの行も段落を切り、フェンスの中を段落から除く
        assert_eq!(
            q("a「b\n```\ne」f\n```\ng」h\n"),
            vec!["000", "000", "000", "000", "000"]
        );
        // 段落の終わりまでに閉じた内側の対だけを除外する
        assert_eq!(q("「a\n「b」c\n\nd」\n"), vec!["00", "0100", "", "00"]);
    }

    fn rule(avoid: &'static [&'static str], keep: &'static [&'static str]) -> Rule {
        Rule {
            avoid,
            write: &["w"],
            keep,
            note: Some("n"),
            bad: &[],
            good: &[],
        }
    }

    #[test]
    fn hits_carry_one_based_line_and_char_column() {
        let rules = [rule(&["交点"], &[])];
        let hits = scan("x\nあい交点う\n", &rules);
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].line, 2);
        assert_eq!(hits[0].col, 3);
        assert_eq!(hits[0].render(), "2:3: 「交点」-> w");
    }

    #[test]
    fn fences_code_spans_and_quotes_hide_hits() {
        let rules = [rule(&["交点"], &[])];
        assert!(scan("```\n交点\n```\n", &rules).is_empty());
        assert!(scan("`交点` は識別子\n", &rules).is_empty());
        assert!(scan("「交点」は避ける\n", &rules).is_empty());
        // 一部だけが隠れている語もヒットから除く
        assert!(scan("交「点」\n", &rules).is_empty());
        // 閉じ忘れた引用の本文を検査対象に残す
        assert_eq!(scan("「交点 は\n", &rules).len(), 1);
        // 同じ行の引用の外は当たる
        assert_eq!(scan("「交点」と交点\n", &rules).len(), 1);
        assert_eq!(scan("「交点」と交点\n", &rules)[0].col, 6);
    }

    #[test]
    fn a_quote_spanning_lines_in_one_paragraph_hides_hits() {
        let rules = [rule(&["交点"], &[])];
        // 行をまたぐ「」の内側を検査対象から除く
        assert!(scan("操作者は「この\n交点は\n避ける」と言った\n", &rules).is_empty());
        // 空行をまたぐ「」の内側も検査対象に残す
        let hits = scan("操作者は「この\n\n交点は避ける」と言った\n", &rules);
        assert_eq!(hits.len(), 1);
        assert_eq!((hits[0].line, hits[0].col), (3, 1));
        // フェンスの行で段落が切れる
        assert_eq!(scan("「a\n```\nb\n```\n交点」\n", &rules).len(), 1);
        // 段落の終わりまでに閉じ忘れた引用の本文を検査対象に残す
        assert_eq!(scan("「この\n交点は\n", &rules).len(), 1);
        // 引用が閉じた後のヒットは出る
        let hits = scan("「この\n交点」の外の交点\n", &rules);
        assert_eq!(hits.len(), 1);
        assert_eq!((hits[0].line, hits[0].col), (2, 7));
    }

    #[test]
    fn keep_suppresses_a_hit_inside_the_kept_word() {
        let rules = [rule(&["道具が言"], &["道具が言語"])];
        assert!(scan("道具が言語に依存する\n", &rules).is_empty());
        assert_eq!(scan("道具が言う\n", &rules).len(), 1);
        // keep が同じ行の別の位置にあっても、そこに収まらないヒットは出す
        assert_eq!(scan("道具が言語と道具が言う\n", &rules).len(), 1);
    }

    #[test]
    fn overlapping_avoids_in_one_rule_collapse_to_the_longest() {
        let rules = [rule(&["独自算術", "算術を持"], &[])];
        let hits = scan("独自算術を持つ\n", &rules);
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].matched, "独自算術");
        let rules = [rule(&["ab", "abcd"], &[])];
        let hits = scan("xabcd\n", &rules);
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].matched, "abcd");
        // 重ならなければ両方出す
        let rules = [rule(&["独自算術", "算術を持"], &[])];
        assert_eq!(scan("独自算術。算術を持つ\n", &rules).len(), 2);
    }

    #[test]
    fn hits_are_ordered_by_line_column_then_rule() {
        let rules = [rule(&["b"], &[]), rule(&["a"], &[]), rule(&["cd"], &[])];
        let hits = scan("acd b\na\n", &rules);
        let seen: Vec<(usize, usize, &str)> = hits
            .iter()
            .map(|h| (h.line, h.col, h.matched.as_str()))
            .collect();
        assert_eq!(
            seen,
            vec![(1, 1, "a"), (1, 2, "cd"), (1, 5, "b"), (2, 1, "a")]
        );
    }

    #[test]
    fn a_hit_inside_a_longer_hit_of_another_rule_is_not_reported() {
        let rules = [rule(&["窓"], &["窓口"]), rule(&["停止窓"], &[])];
        let hits = scan("停止窓と窓\n", &rules);
        let seen: Vec<(usize, &str)> = hits.iter().map(|h| (h.col, h.matched.as_str())).collect();
        assert_eq!(seen, vec![(1, "停止窓"), (5, "窓")]);
        // 一部だけの重なりは両方出す
        let rules = [rule(&["ab"], &[]), rule(&["bc"], &[])];
        assert_eq!(scan("abc\n", &rules).len(), 2);
    }
}
