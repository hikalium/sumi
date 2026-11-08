//! 日本語と英語の混在の検査。語の置き換えの表 (src/rules.rs) とは別の、文書の構造を見る検査である。
//!
//! 1 つの単位の中に、日本語の文と英語の文が並ぶ書き方を当てる。単位は、段落、箇条書きの 1 項目、
//! 表のセル、見出しの 1 行である。Rust のソースでは、空行やコードで切れずに続くコメント (1 つの
//! doc comment など) が段落にあたり、空のコメント行 (`///` だけの行) を挟んでも同じ段落に含める。
//! コードの後ろに置いた行末のコメントは、その 1 行で 1 つの単位である。英語の文書と日本語の文書は
//! どちらも認めており、当てるのは 1 つの単位の中での混在だけである。
//!
//! 単位に日本語の文があるとは、除外の外にひらがなが `MIN_HIRAGANA` 字以上あることをいう。英語の
//! 文があるとは、英語の語が `MIN_WORDS` 語以上続き、その中に機能語 (`FUNCTION_WORDS`) が 1 つ以上
//! あることをいう。英語の語は、ASCII の英字で始まり英字とアポストロフィとハイフンだけからなる
//! トークンである。数字だけのトークン、パスや識別子のように記号を含むトークン、除外した範囲を
//! 語数から除き、前後の語を同じ並びに含める。並びを切るのは、除外の外にある ASCII 以外の文字 (日本語の文字と
//! 全角の記号) と、`/` や `->` のような記号だけのトークン (列挙の区切り) である。
//!
//! 次の範囲にある英語を混在の判定から除く。フェンス、インラインコード (行をまたぐもの
//! を含む)、「」の中の引用 (scan.rs の判定)、ASCII の二重引用符の対の中 (行をまたぐものを含む)、
//! Markdown のリンクの行き先 `](...)`、見出しの anchor `{#...}`、URL。短い対訳の括弧書き
//! (`目的 (standing mission)`) と、識別子や名前を並べただけの英語も、語数の下限か機能語の条件によって
//! 検出対象から外れる。
//!
//! 下限の値は、文書、Rust のコメント、HTML で語数と機能語の条件ごとの検出件数を数え、
//! 誤検出の例を読んで選んだ。

use crate::scan::{Document, code_span_mask};

/// 英語の文があるとみなす、続いた英語の語の数の下限。
pub const MIN_WORDS: usize = 5;
/// 日本語の文があるとみなす、ひらがなの字数の下限。
pub const MIN_HIRAGANA: usize = 2;

/// 検出結果の行の `->` の右に出す、修正方法の案。
pub const WRITE: &str = "単位 (段落、箇条書きの 1 項目、表のセル、見出し、ひと続きのコメント) を周りの文書の言語 1 つで書き直す";

/// `--rules` と改善プロンプトの規則の一覧に出す 1 行。形は src/rules.rs の `Rule::render_line` に揃える。
pub fn render_line() -> String {
    format!(
        "(日本語の文と、機能語を含み {MIN_WORDS} 語以上続く英語の並びが、1 つの単位に同居する書き方) -> {WRITE} | \
         keep: インラインコード / フェンス / 「」や \"\" の中の引用 / リンクの行き先 / URL / \
         {MIN_WORDS} 語未満の英語 (短い対訳の括弧書き) / 機能語の無い英語 (識別子や名前の列挙) | \
         note: 英語の文書と日本語の文書はどちらも認めるが、1 つの単位の中に 2 つの言語の文を並べると、\
         どちらの言語の読み手にも読みにくい。コードから出る説明 (clap の help など) は行を空白で繋ぐので、\
         日本語の行の折り返しに余計な空白も入る。"
    )
}

/// 混在のヒット 1 件。`line` と `col` は 1 始まりで、`col` は文字 (char) 単位。英語の並びの先頭を指す。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Hit {
    pub line: usize,
    pub col: usize,
    /// 英語の並びの抜粋
    pub excerpt: String,
}

impl Hit {
    /// `LINE:COL: 「英語の並び」-> 直し方` の形。規則のヒットと同じ形にする。
    pub fn render(&self) -> String {
        format!("{}:{}: 「{}」-> {WRITE}", self.line, self.col, self.excerpt)
    }
}

/// 単位の中の 1 文字。
#[derive(Clone, Copy)]
pub(crate) struct Cell {
    pub line: usize,
    pub col: usize,
    pub c: char,
    /// 除外の中か
    pub hidden: bool,
}

/// 単位の割り方から見た、行の種類。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LineKind {
    /// 文書の行。空行は単位を切る
    Normal,
    /// その 1 行で 1 つの単位にする行 (Rust の行末のコメント)
    Standalone,
    /// 中身の無いコメントの行 (`///` だけの行)。単位を切らない
    Continues,
}

/// 文書を単位に割るための、Markdown の構造の外にある手がかり。
pub struct Layout {
    /// 各行の種類。足りない行は `LineKind::Normal` とみなす
    pub kinds: Vec<LineKind>,
    /// 各行の各文字が単位の境目か (HTML のブロック要素のタグ)。未指定の行と文字を現在の単位に含める。
    pub breaks: Vec<Vec<bool>>,
}

impl Layout {
    /// 手がかりの無い文書 (Markdown)。
    pub fn plain() -> Layout {
        Layout {
            kinds: Vec::new(),
            breaks: Vec::new(),
        }
    }

    /// Rust のソースの割り方。`comments` は `source::rust_comments` がソース `src` から取り出した
    /// コメントだけの本文である。1 つの doc comment のように、空行やコードで切れずに続くコメントの行は、
    /// 中身の無いコメントの行 (`///` だけの行) を挟んでも 1 つの単位である。コードの後ろに置いた行末の
    /// コメントは、その 1 行で 1 つの単位にする (隣の行のコメントとは別のことを述べている)。
    pub fn rust(src: &str, comments: &str) -> Layout {
        Layout {
            kinds: rust_line_kinds(src, comments),
            breaks: Vec::new(),
        }
    }

    /// HTML の割り方。`text` は `source::html_text` が `src` から取り出した本文である。ブロック要素の
    /// タグ (`INLINE_ELEMENTS` に無い要素のタグ) は単位を切るので、要素ごとの本文と、そのタグの中の
    /// 属性値 (`aria-label` など) は、それぞれ別の単位になる。
    pub fn html(src: &str, text: &str) -> Layout {
        Layout {
            kinds: Vec::new(),
            breaks: html_breaks(src, text),
        }
    }

    fn kind(&self, line: usize) -> LineKind {
        self.kinds.get(line).copied().unwrap_or(LineKind::Normal)
    }

    fn is_break(&self, line: usize, col: usize) -> bool {
        self.breaks
            .get(line)
            .and_then(|l| l.get(col))
            .copied()
            .unwrap_or(false)
    }
}

/// 単位ごとに混在を判定し、単位ごとに高々 1 件のヒットを行と列の順で返す。単位は `prose_units` が作る。
pub fn scan_units(units: &[Vec<Cell>]) -> Vec<Hit> {
    let mut hits: Vec<Hit> = units.iter().filter_map(|unit| judge(unit)).collect();
    hits.sort_by_key(|h| (h.line, h.col));
    hits
}

/// 文の途中に置かれ、単位を切らない HTML の要素。
const INLINE_ELEMENTS: &[&str] = &[
    "a", "abbr", "b", "bdi", "bdo", "cite", "code", "data", "dfn", "em", "i", "kbd", "mark", "q",
    "s", "samp", "small", "span", "strong", "sub", "sup", "time", "u", "var", "wbr",
];

/// HTML の各文字が単位の境目か。インライン要素でないタグの文字のうち、本文として取り出されなかった
/// もの (属性値以外) を境目にする。
fn html_breaks(src: &str, text: &str) -> Vec<Vec<bool>> {
    let chars: Vec<char> = src.chars().collect();
    let kept: Vec<char> = text.chars().collect();
    let mut flat = vec![false; chars.len()];
    let mut i = 0;
    while i < chars.len() {
        let opens_tag = chars[i] == '<'
            && chars
                .get(i + 1)
                .is_some_and(|&n| n.is_ascii_alphabetic() || n == '/');
        if !opens_tag {
            i += 1;
            continue;
        }
        let name_start = if chars[i + 1] == '/' { i + 2 } else { i + 1 };
        let name: String = chars[name_start..]
            .iter()
            .take_while(|c| c.is_ascii_alphanumeric())
            .collect::<String>()
            .to_ascii_lowercase();
        // タグの終わりを、引用符の外の > で判定する。
        let mut end = i + 1;
        let mut quote: Option<char> = None;
        while end < chars.len() {
            match (quote, chars[end]) {
                (None, '"' | '\'') => quote = Some(chars[end]),
                (Some(q), c) if c == q => quote = None,
                (None, '>') => break,
                _ => {}
            }
            end += 1;
        }
        if !INLINE_ELEMENTS.contains(&name.as_str()) {
            for (k, b) in flat
                .iter_mut()
                .enumerate()
                .take((end + 1).min(chars.len()))
                .skip(i)
            {
                *b = kept.get(k).is_none_or(|c| c.is_whitespace());
            }
        }
        i = end + 1;
    }
    // 行ごとに割る (改行は境目の対象外で、行の区切りとして落とす)
    let mut out = vec![Vec::new()];
    for (c, b) in chars.iter().zip(flat) {
        if *c == '\n' {
            out.push(Vec::new());
        } else if let Some(line) = out.last_mut() {
            line.push(b);
        }
    }
    out
}

/// Rust のソースの各行の種類。
fn rust_line_kinds(src: &str, comments: &str) -> Vec<LineKind> {
    src.lines()
        .zip(comments.lines())
        .map(
            |(s, c)| match c.chars().position(|ch| !ch.is_whitespace()) {
                // 同じ行のコメントの前にコードがあれば、行末のコメントである
                Some(p)
                    if s.chars()
                        .take(p)
                        .any(|ch| !ch.is_whitespace() && !matches!(ch, '/' | '*' | '!')) =>
                {
                    LineKind::Standalone
                }
                Some(_) => LineKind::Normal,
                None if s.trim_start().starts_with("//") => LineKind::Continues,
                None => LineKind::Normal,
            },
        )
        .collect()
}

/// 構造の検査 (混在と否定構文) で共用する、位置と引用・コードの除外を保持した本文の単位。
pub(crate) fn prose_units(doc: &Document, layout: &Layout) -> Vec<Vec<Cell>> {
    let mut out = units(doc, layout);
    for unit in &mut out {
        hide_code_spans_across_lines(unit);
        hide_double_quotes(unit);
    }
    out
}

/// 行をまたぐインラインコードも除外にする。scan.rs の `code_span_mask` は docgate.rs と同じ判定を保つ
/// ために行ごとに判定するので、段落の中で行を折り返したインラインコードはここで単位ごとに拾い直す。
/// 単位の行の境目は空白 1 文字になっているので、同じ関数をそのまま単位に当てられる。
fn hide_code_spans_across_lines(unit: &mut [Cell]) {
    let chars: Vec<char> = unit.iter().map(|k| k.c).collect();
    for (k, code) in unit.iter_mut().zip(code_span_mask(&chars)) {
        k.hidden |= code;
    }
}

/// ASCII の二重引用符の対の中を、「」と同じく引用として除外にする。除外の外にある `"` を前から
/// 2 つずつ対にし、行をまたぐ対も拾う。最後に余った 1 つと後続の本文を検査対象に保つ。
fn hide_double_quotes(unit: &mut [Cell]) {
    let quotes: Vec<usize> = (0..unit.len())
        .filter(|&i| !unit[i].hidden && unit[i].c == '"')
        .collect();
    for &[open, close] in quotes.as_chunks::<2>().0 {
        for k in &mut unit[open..=close] {
            k.hidden = true;
        }
    }
}

/// 1 つの単位を判定する。日本語の文があり、英語の並びが条件 (`MIN_WORDS` 語以上で、機能語を含む) を
/// 満たすなら、最初のその並びを返す。
fn judge(unit: &[Cell]) -> Option<Hit> {
    let hiragana = unit
        .iter()
        .filter(|k| !k.hidden && ('\u{3041}'..='\u{309F}').contains(&k.c))
        .count();
    if hiragana < MIN_HIRAGANA {
        return None;
    }
    english_runs(unit)
        .into_iter()
        .find(|r| r.words >= MIN_WORDS && r.function_words > 0)
        .map(|r| {
            let excerpt: String = unit[r.start..r.end]
                .iter()
                .map(|k| k.c)
                .collect::<String>()
                .split_whitespace()
                .collect::<Vec<_>>()
                .join(" ");
            Hit {
                line: unit[r.start].line + 1,
                col: unit[r.start].col + 1,
                excerpt: shorten(&excerpt),
            }
        })
}

/// 抜粋を 60 字までに詰める。
fn shorten(s: &str) -> String {
    const MAX: usize = 60;
    if s.chars().count() <= MAX {
        s.to_string()
    } else {
        let head: String = s.chars().take(MAX).collect();
        format!("{head} ...")
    }
}

/// 英語の文にほぼ必ず現れる機能語。識別子や名前だけの列挙 (`server, backend, provider, model`) を
/// 文の判定から除くために、機能語の出現を条件とする。
pub const FUNCTION_WORDS: &[&str] = &[
    "a", "an", "the", "of", "to", "in", "on", "at", "by", "for", "from", "with", "into", "as",
    "is", "are", "was", "were", "be", "been", "not", "no", "and", "or", "but", "if", "that",
    "this", "it", "its", "which", "when", "than", "then", "so", "do", "does", "did", "has", "have",
    "can", "will", "would", "should", "must", "may", "before", "after", "only", "every", "any",
    "each",
];

/// 単位の中の英語の並び 1 つ。
struct Run {
    /// 最初の語の先頭 (単位の中の位置)
    start: usize,
    /// 最後の語の末尾
    end: usize,
    /// 語の数
    words: usize,
    /// そのうちの機能語の数
    function_words: usize,
}

/// 単位の中の英語の並び。
fn english_runs(unit: &[Cell]) -> Vec<Run> {
    let mut runs = Vec::new();
    let mut run: Option<Run> = None;
    let mut i = 0;
    while i < unit.len() {
        let k = unit[i];
        if k.hidden || k.c.is_whitespace() {
            i += 1;
            continue;
        }
        if !k.c.is_ascii() {
            // 日本語の文字や全角の記号が並びを切る
            runs.extend(run.take());
            i += 1;
            continue;
        }
        // 空白、除外、ASCII 以外の文字までを 1 つのトークンにする
        let start = i;
        let mut end = i;
        while end < unit.len() {
            let t = unit[end];
            if t.hidden || t.c.is_whitespace() || !t.c.is_ascii() {
                break;
            }
            end += 1;
        }
        let token: String = unit[start..end].iter().map(|t| t.c).collect();
        if !token.chars().any(|c| c.is_ascii_alphanumeric()) && token != "--" {
            // `/`、`->`、`|`、`+` のような記号だけのトークンを列挙の区切りとして扱う。
            runs.extend(run.take());
        } else if let Some((a, b)) = word_bounds(&token) {
            let word = token[a..b].to_ascii_lowercase();
            let function = usize::from(FUNCTION_WORDS.contains(&word.as_str()));
            let r = run.get_or_insert(Run {
                start: start + a,
                end: start + b,
                words: 0,
                function_words: 0,
            });
            r.end = start + b;
            r.words += 1;
            r.function_words += function;
        }
        i = end;
    }
    runs.extend(run);
    runs
}

/// トークンが英語の語なら、前後の句読点を除いた語の範囲 (トークンの中のバイト位置) を返す。
/// トークンは ASCII だけなので、バイト位置と文字の位置は一致する。
fn word_bounds(token: &str) -> Option<(usize, usize)> {
    let bytes = token.as_bytes();
    let punct = |c: u8| b"()[]{}\"'`,.;:!?*".contains(&c);
    let a = bytes.iter().position(|&c| !punct(c))?;
    let b = bytes.len() - bytes.iter().rev().position(|&c| !punct(c))?;
    let word = &bytes[a..b];
    let is_word = word[0].is_ascii_alphabetic()
        && word
            .iter()
            .all(|&c| c.is_ascii_alphabetic() || c == b'\'' || c == b'-');
    is_word.then_some((a, b))
}

/// 文書を単位に割る。見出しの行と `LineKind::Standalone` の行は、その 1 行で 1 つの単位にする。
/// `LineKind::Continues` の行を現在の単位に含める。
fn units(doc: &Document, layout: &Layout) -> Vec<Vec<Cell>> {
    let mut units: Vec<Vec<Cell>> = Vec::new();
    let mut current: Vec<Cell> = Vec::new();
    let flush = |current: &mut Vec<Cell>, units: &mut Vec<Vec<Cell>>| {
        if !current.is_empty() {
            units.push(std::mem::take(current));
        }
    };
    for (li, chars) in doc.lines.iter().enumerate() {
        let hidden = hidden_mask(doc, li);
        let cells: Vec<Cell> = chars
            .iter()
            .enumerate()
            .map(|(col, &c)| Cell {
                line: li,
                col,
                c,
                hidden: hidden[col],
            })
            .collect();
        let text: String = chars.iter().collect();
        let body = text.trim_start().trim_start_matches('>').trim_start();
        let kind = layout.kind(li);
        if kind == LineKind::Continues {
            continue;
        }
        if doc.fenced[li] || body.is_empty() {
            flush(&mut current, &mut units);
            continue;
        }
        if body.starts_with('#') || kind == LineKind::Standalone {
            flush(&mut current, &mut units);
            units.push(cells);
            continue;
        }
        if body.starts_with('|') {
            flush(&mut current, &mut units);
            // 表の行をセルごとの単位にする。区切りにはインラインコードの外の | を使う。
            let mut cell = Vec::new();
            for k in cells {
                if k.c == '|' && !doc.code[li][k.col] {
                    flush(&mut cell, &mut units);
                } else {
                    cell.push(k);
                }
            }
            flush(&mut cell, &mut units);
            continue;
        }
        if list_item(body) {
            flush(&mut current, &mut units);
        }
        if !current.is_empty() {
            // 行の境目は空白として扱う
            current.push(Cell {
                line: li,
                col: 0,
                c: ' ',
                hidden: true,
            });
        }
        // HTML のブロック要素のタグは、行の途中でも単位を切る
        for k in cells {
            if layout.is_break(li, k.col) {
                flush(&mut current, &mut units);
            } else {
                current.push(k);
            }
        }
    }
    flush(&mut current, &mut units);
    units
}

/// 箇条書きの項目の始まりか (`- `、`* `、`+ `、`1. `、`1) `)。
fn list_item(body: &str) -> bool {
    let mut chars = body.chars();
    match chars.next() {
        Some('-' | '*' | '+') => chars.next() == Some(' '),
        Some(c) if c.is_ascii_digit() => {
            let rest = body.trim_start_matches(|c: char| c.is_ascii_digit());
            rest.starts_with(". ") || rest.starts_with(") ")
        }
        _ => false,
    }
}

/// 行の各文字が除外の中か。フェンス、インラインコード、「」の引用に、リンクの行き先、見出しの
/// anchor、URL を足す。
fn hidden_mask(doc: &Document, li: usize) -> Vec<bool> {
    let chars = &doc.lines[li];
    let mut mask: Vec<bool> = (0..chars.len())
        .map(|k| doc.fenced[li] || doc.code[li][k] || doc.quoted[li][k])
        .collect();
    let n = chars.len();
    let mut i = 0;
    while i < n {
        let rest: String = chars[i..].iter().take(8).collect();
        let close = |open: usize, close_char: char| {
            (open..n)
                .find(|&j| chars[j] == close_char)
                .map_or(n, |j| j + 1)
        };
        if chars[i] == ']' && chars.get(i + 1) == Some(&'(') {
            let end = close(i + 1, ')');
            mask[i + 1..end].fill(true);
            i = end;
        } else if chars[i] == '{' && chars.get(i + 1) == Some(&'#') {
            let end = close(i, '}');
            mask[i..end].fill(true);
            i = end;
        } else if rest.starts_with("http://") || rest.starts_with("https://") {
            let end = (i..n)
                .find(|&j| chars[j].is_whitespace() || !chars[j].is_ascii())
                .unwrap_or(n);
            mask[i..end].fill(true);
            i = end;
        } else {
            i += 1;
        }
    }
    mask
}

#[cfg(test)]
mod tests {
    use super::*;

    fn scan_layout(body: &str, layout: &Layout) -> Vec<Hit> {
        scan_units(&prose_units(&Document::parse(body), layout))
    }

    fn scan(body: &str) -> Vec<Hit> {
        scan_layout(body, &Layout::plain())
    }

    fn scan_rust(src: &str, comments: &str) -> Vec<Hit> {
        scan_layout(comments, &Layout::rust(src, comments))
    }

    fn scan_html(src: &str, text: &str) -> Vec<Hit> {
        scan_layout(text, &Layout::html(src, text))
    }

    fn hits(s: &str) -> Vec<String> {
        scan(s).iter().map(Hit::render).collect()
    }

    #[test]
    fn a_japanese_sentence_and_an_english_clause_in_one_paragraph_hit() {
        let doc = "path to a JSON array of real-input steps played before the test.\n\
                   合成した DOM イベントでは通らない振る舞いを測るための道である。\n";
        let h = scan(doc);
        assert_eq!(h.len(), 1, "{h:?}");
        assert_eq!((h[0].line, h[0].col), (1, 1));
        assert!(h[0].excerpt.starts_with("path to a JSON array"));
        // 1 行の中の混在も当たる
        assert_eq!(
            hits("この値は、the number of rounds the task may still run を表す。\n").len(),
            1
        );
        // 単位ごとに高々 1 件
        assert_eq!(
            hits("これは the first clause of this unit で、次は the second clause of this unit である。\n")
                .len(),
            1
        );
    }

    #[test]
    fn the_doc_comment_the_operator_pointed_at_is_caught() {
        // 2026-09-28 に操作者が指摘した clap の help の元の doc comment (Rust のコメントとして読む)
        let src = "/// path to a JSON array of real-input steps (CDP `Input.*`) played before `--test-js`.\n\
                   /// 合成した DOM イベントでは通らない振る舞い (キーの押しっぱなし ・焦点 ・既定の動作) を\n\
                   /// 測るための道で、手の表記は [`crate::cdp::input_step`] が持つ。\n\
                   #[arg(long)]\n\
                   input: Option<String>,\n";
        let body = crate::source::rust_comments(src);
        let h = scan_rust(src, &body);
        assert_eq!(h.len(), 1, "{h:?}");
        assert_eq!((h[0].line, h[0].col), (1, 5));
    }

    #[test]
    fn single_language_units_do_not_hit() {
        // 英語だけ、日本語だけの段落
        assert!(hits("This paragraph is written in English from start to end.\n").is_empty());
        assert!(hits("この段落は日本語だけで書いてある。\n").is_empty());
        // 隣り合う段落も、それぞれを別の単位として検査する。
        assert!(
            hits("This paragraph is written in English from start to end.\n\nこの段落は日本語である。\n")
                .is_empty()
        );
        // 箇条書きの項目が違えば、別の単位である
        assert!(
            hits(
                "- This item is written in English from start to end.\n- この項目は日本語である。\n"
            )
            .is_empty()
        );
        // 表のセルが違えば、別の単位である
        assert!(
            hits("| This cell is written in English from start to end | 日本語のセルである |\n")
                .is_empty()
        );
        // 見出しは次の段落と別の単位である
        assert!(
            hits("# An English heading with the words here\n本文は日本語で書いてある。\n")
                .is_empty()
        );
        // フェンスは段落を切る
        assert!(
            hits("本文は日本語である。\n```\nx\n```\nThis paragraph is written in English from start to end.\n")
                .is_empty()
        );
    }

    #[test]
    fn excluded_spans_do_not_hit() {
        // インラインコード。行をまたぐものも除く
        assert!(
            hits("`cargo test -- --nocapture for the whole of the suite` を回す。\n").is_empty()
        );
        assert!(
            hits(
                "`host live run --owner SESSION --purpose\nDESCRIPTION -- COMMAND` の中で行う。\n"
            )
            .is_empty()
        );
        // 「」の引用と、同じ行で閉じる ASCII の二重引用符の引用
        assert!(hits("操作者は「this is not what I asked for at all」と書いた。\n").is_empty());
        assert!(hits("拒否文は \"The todo was not added.\" である。\n").is_empty());
        // リンクの行き先、見出しの anchor、URL
        assert!(
            hits("[規則](#is-not-the-one-to-use-for-this) と https://example.com/is-a-page-of-the-site を見る。\n")
                .is_empty()
        );
        assert!(hits("# 見出しである {#is-not-a-sentence-of-the-text}\n").is_empty());
        // フェンスの中
        assert!(
            hits("次を打つ。\n\n```\nrun the whole test suite again and again\n```\n").is_empty()
        );
    }

    #[test]
    fn short_english_and_lists_of_names_do_not_hit() {
        // 短い対訳の括弧書き
        assert!(
            hits("目的 (standing mission) を決めてから持ち時間 (quantum) を渡す。\n").is_empty()
        );
        // 識別子とパスだけの英語
        assert!(
            hits("container/src/cli.rs の UitestCdp と crate::cdp::input_step を読む。\n")
                .is_empty()
        );
        // 機能語の無い名前の列挙
        assert!(hits("列は server, backend, provider, model, digest である。\n").is_empty());
        // 記号だけのトークンは列挙の区切りで、並びを切る
        assert!(hits("状態は good / bad / very bad / unavailable の 4 つである。\n").is_empty());
        assert!(hits("流れは spawn -> scheduler -> agent -> the finish で進む。\n").is_empty());
        // 4 語までの英語の並びは当たらず、機能語を含む 5 語から当たる
        assert!(hits("これは one of the four である。\n").is_empty());
        assert_eq!(hits("これは one of the five words である。\n").len(), 1);
        // 機能語が無ければ 5 語以上でも当たらない
        assert!(hits("これは alpha beta gamma delta epsilon zeta である。\n").is_empty());
    }

    #[test]
    fn a_japanese_word_in_an_english_paragraph_is_not_a_japanese_sentence() {
        // ひらがなが 1 字以下の語や短い引用を、日本語の文の判定から除く。
        assert!(
            hits("The operator ruled that the term 持ち時間 replaces the older word.\n").is_empty()
        );
    }

    #[test]
    fn a_hit_points_at_the_english_run() {
        let h = scan("日本語の文である。\nここから the rest of this line is English text\n");
        assert_eq!(h.len(), 1);
        assert_eq!((h[0].line, h[0].col), (2, 6));
        assert_eq!(h[0].excerpt, "the rest of this line is English text");
        assert_eq!(
            h[0].render(),
            format!("2:6: 「the rest of this line is English text」-> {WRITE}")
        );
    }

    #[test]
    fn a_long_run_is_shortened_in_the_excerpt() {
        let long = "the word ".repeat(20);
        let h = scan(&format!("日本語の文である。{long}\n"));
        assert_eq!(h.len(), 1);
        assert!(h[0].excerpt.ends_with(" ..."));
        assert_eq!(h[0].excerpt.chars().count(), 64);
    }

    #[test]
    fn a_trailing_comment_is_a_unit_of_its_own() {
        let rust = |src: &str| scan_rust(src, &crate::source::rust_comments(src));
        // 行末のコメントは、次の行の doc comment と別の単位である
        let src = "    seed: &[u8], // files written into work/ before the trial\n\
                   /// この課題が続けて出す操作者の発話である。\n\
                   next: u8,\n";
        assert!(rust(src).is_empty(), "{:?}", rust(src));
        // 行末のコメントの中の混在は当たる
        let src = "let x = 1; // the value of the counter と、その意味である。\n";
        assert_eq!(rust(src).len(), 1);
        // 行頭から始まるコメントの続きは、これまでどおり 1 つの単位である
        let src =
            "// the value of the counter is kept here\n// 次の行は日本語である。\nfn f() {}\n";
        assert_eq!(rust(src).len(), 1);
        assert_eq!(
            rust_line_kinds(
                "a(); // x\n  // y\n///\n/* z */\n",
                "      x\n     y\n   \n   z   \n"
            ),
            vec![
                LineKind::Standalone,
                LineKind::Normal,
                LineKind::Continues,
                LineKind::Normal
            ]
        );
    }

    #[test]
    fn one_doc_comment_is_one_unit_across_empty_comment_lines() {
        let rust = |src: &str| scan_rust(src, &crate::source::rust_comments(src));
        let src = "/// Parse the line spec against a file of the given length.\n\
                   ///\n\
                   /// 表記を 2 つの数へ解く、唯一の読み手である。\n\
                   fn f() {}\n";
        assert_eq!(rust(src).len(), 1);
        // コードの行が挟まれば別のコメントである
        let src = "/// Parse the line spec against a file of the given length.\n\
                   fn f() {}\n\
                   /// 表記を 2 つの数へ解く、唯一の読み手である。\n\
                   fn g() {}\n";
        assert!(rust(src).is_empty());
    }

    #[test]
    fn html_block_elements_and_attribute_values_are_units_of_their_own() {
        let html = |src: &str| scan_html(src, &crate::source::html_text(src));
        // 英語の画面の部品と、日本語のメッセージの本文は、要素が違えば別の単位である
        let src = "<div aria-label=\"Screenshot preview of the revised tab\"><div>この修正は既存の契約を変えません。</div></div>\n\
                   <header><span>sample</span><button>Show the rest of the list</button><p>日本語の段落である。</p></header>\n";
        assert!(html(src).is_empty(), "{:?}", html(src));
        // 1 つの要素の中の混在を検出する。インライン要素を同じ単位に含める。
        let src = "<p>この段落は <b>the rest of this sentence is English</b> である。</p>\n";
        assert_eq!(html(src).len(), 1);
        // 境目の計算は行と文字の位置を保つ
        assert_eq!(
            html_breaks("<p>a</p>\n<b>c</b>\n", "   a    \n   c    \n")[..2],
            vec![
                vec![true, true, true, false, true, true, true, true],
                vec![false; 8]
            ]
        );
    }

    #[test]
    fn a_double_quoted_quote_may_span_lines() {
        let doc = "拒否の見出し (\"The task was not\nmarked finished.\") は形容詞である。\n";
        assert!(hits(doc).is_empty(), "{:?}", hits(doc));
        // 二重引用符の対を探す範囲から、インラインコードを除く。
        let doc = "`\"` の後の the rest of this line is English である。\n";
        assert_eq!(hits(doc).len(), 1);
    }
}
