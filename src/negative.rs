//! 作業の免責 (書き手が、したことや今の状態の代わりに、していないことや範囲の外を書いて自分の作業の
//! 範囲を先回りで打ち消す文) を、字面のリテラルの照合で検出する。形態素解析と正規表現を使わない。
//!
//! 1 文 (句点などで区切る) ごとに、引用とコードを除いた可視の文字から空白を除いた列を作り、次の型を
//! 先頭から照合して、最初に当たった 1 か所を出す。
//!   A. 作業の名詞 + 助詞 + 未完了の否定の述語 (例: 「修正を実施していない」「テストはまだ実施していない」)
//!   B. 根拠の名詞 + 助詞 + 存在の否定 (例: 「根拠がない」)
//!   C. 範囲の打ち消し (例: 「調査であり実装ではない」「全てを調べたわけではない」)
//!   D. 作業の名詞 + 助詞 + 範囲外 (例: 「性能の評価は今回の範囲外である」)
//!   E. 作業の名詞 + 助詞 + 省略の過去形 (例: 「異常系の動作確認は省略した」)
//! 型 A、D、E は書き手の作業の報告なので、Markdown と HTML にだけ適用する。Rust のコメントではプログラムが
//! 主語の説明 (「起動を検証できなかった。」のような終了コードの説明) に当たるので、型 B と C だけを適用する。
//! 平叙の現在形 (「Python は使わない」) は指示や禁止の形なので、述語に置かない。述語の直後が条件 (と、場合、
//! なら)、名詞化 (こと、もの)、名詞の修飾 (直後が漢字、カタカナ、英数字、コード) なら除く。

use crate::mixed::Cell;
use std::sync::OnceLock;

pub const FORMS: &str =
    "作業の名詞 + していない / 根拠がない / 〜であり〜ではない / 〜わけではない / 〜は範囲外";
pub const WRITE: &str = crate::rules::with_purpose_check!(
    "事実・現在の状態・対象・次の行動を直接述べる。未実施や制約が判断に必要なら、その状態と必要な手順を具体的に書く"
);

pub fn render_line() -> String {
    format!(
        "{FORMS} -> {WRITE} | keep: 引用 / インラインコード / フェンス / 条件 (場合、なら) / 名詞の修飾 / 指示の現在形 (しない) | note: 作業の免責を検出する。作業の名詞の型と範囲外の型は Markdown と HTML にだけ適用する。"
    )
}

/// 検査する本文の種類。作業の報告の型 (A、D、E) を適用するかを決める。
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Prose {
    /// 文書 (Markdown、HTML)。全ての型を適用する
    Document,
    /// プログラムのコメント (Rust)。根拠と範囲の打ち消しの型だけを適用する
    Comment,
}

#[derive(Debug)]
pub struct Hit {
    pub line: usize,
    pub col: usize,
    pub excerpt: String,
}

impl Hit {
    pub fn render(&self) -> String {
        format!("{}:{}: 「{}」-> {WRITE}", self.line, self.col, self.excerpt)
    }
}

/// 書き手の仕事の段階を指す名詞。
const WORK: &[&str] = &[
    "動作確認",
    "ベンチマーク",
    "ベンチ",
    "レビュー",
    "テストケース",
    "テスト",
    "洗い出し",
    "特定",
    "証明",
    "最適化",
    "実施",
    "実装",
    "実行",
    "検証",
    "確認",
    "計測",
    "測定",
    "実験",
    "試験",
    "再現",
    "調査",
    "着手",
    "評価",
    "比較",
    "監視",
    "修正",
    "対応",
    "反映",
    "検討",
    "分析",
    "変更",
    "操作",
    "転記",
];

/// 読む作業の目的語。「ログを読んでいない」のように、読む動詞とだけ組んで作業を表す。
const READ_OBJECT: &[&str] = &["ログ", "コード", "差分", "ソース"];
const READ_VERB: &[&str] = &["読んで", "見て"];

/// 目的語を問わず、それだけで書き手の作業を表す動詞の「て」の形。直前の助詞 (を、は、も) と組む。
const WORK_VERB: &[&str] = &["調べて", "測って", "確かめて", "試して", "触って"];

/// 作業の名詞の後ろに置ける助詞。空の文字列は、名詞の直後に述語が続く形。長いものを先に置く。
const PARTICLES: &[&str] = &[
    "については",
    "までは",
    "などは",
    "すら",
    "さえ",
    "など",
    "まで",
    "には",
    "では",
    "は",
    "も",
    "を",
    "が",
    "",
];

/// 助詞と述語の間に置ける副詞。長いものを先に置く。
const ADVERBS: &[&str] = &[
    "現時点では",
    "現時点で",
    "今回は",
    "十分に",
    "実際に",
    "今回",
    "まだ",
    "未だ",
    "特に",
    "一切",
    "全く",
    "直接",
    "",
];

/// 未完了の否定を作る動詞の「て」の形。
const TE: &[&str] = &[
    "して",
    "行って",
    "行えて",
    "できて",
    "終わって",
    "済んで",
    "終えて",
    "完了して",
    "受けて",
    "通して",
    "試して",
    "試みて",
    "回して",
    "走らせて",
    "流して",
    "書いて",
    "やって",
    "至って",
    "確かめて",
];

/// 受け身の「て」の形。「確認されていない」のような結果の報告と分けるため、助詞を置いた形だけを拾う。
const PASSIVE_TE: &str = "されて";

/// 「て」の形に続く否定。「ている」の否定と、その丁寧形と過去形。
const TE_NEG: &[&str] = &[
    "いませんでした",
    "いなかった",
    "いません",
    "いない",
    "おりません",
    "おらず",
];

/// 過去形の否定と、「て」の形を取らない未完了の述語。
const PAST_NEG: &[&str] = &[
    "しませんでした",
    "しなかった",
    "行いませんでした",
    "行わなかった",
    "やりませんでした",
    "やらなかった",
    "試さなかった",
    "回さなかった",
    "確かめなかった",
    "に至っていません",
    "に至っていない",
];

/// 打ち消しの連用中止形 (「計測は行わず、既定値を使った」)。「計画を実行せず、」のような指示と分けるため、
/// 対比の助詞「は」「も」の後ろだけで拾う。
const ZU: &[&str] = &[
    "行わず、",
    "試さず、",
    "調べず、",
    "測らず、",
    "確かめず、",
    "せず、",
];

fn zu() -> &'static [String] {
    static LIST: OnceLock<Vec<String>> = OnceLock::new();
    LIST.get_or_init(|| ZU.iter().map(|z| z.to_string()).collect())
}

/// 作業を飛ばしたことの報告 (型 E)。指示と分けるため、過去形と「ている」の形だけを置く。
const SKIPPED: &[&str] = &[
    "省略した",
    "省略し、",
    "省略している",
    "省略しており",
    "後回しにした",
    "後回しにしている",
    "スキップした",
    "スキップしている",
    "見送った",
    "見送っている",
    "割愛した",
    "割愛している",
    "見送り、",
    "後回しにし、",
    "スキップし、",
    "割愛し、",
];

/// 「て」の形と否定を組んだ述語の一覧。長いものを先に照合する。
fn with_neg(te: &[&str], past: &[&str]) -> Vec<String> {
    let mut v: Vec<String> = te
        .iter()
        .flat_map(|t| TE_NEG.iter().map(move |n| format!("{t}{n}")))
        .chain(past.iter().map(|s| s.to_string()))
        .collect();
    v.sort_by_key(|s| std::cmp::Reverse(s.chars().count()));
    v
}

fn undone() -> &'static [String] {
    static LIST: OnceLock<Vec<String>> = OnceLock::new();
    LIST.get_or_init(|| with_neg(TE, PAST_NEG))
}

fn passive() -> &'static [String] {
    static LIST: OnceLock<Vec<String>> = OnceLock::new();
    LIST.get_or_init(|| with_neg(&[PASSIVE_TE], &["されなかった"]))
}

fn read_undone() -> &'static [String] {
    static LIST: OnceLock<Vec<String>> = OnceLock::new();
    LIST.get_or_init(|| with_neg(READ_VERB, &["読まなかった", "見なかった"]))
}

fn verb_undone() -> &'static [String] {
    static LIST: OnceLock<Vec<String>> = OnceLock::new();
    LIST.get_or_init(|| {
        with_neg(
            WORK_VERB,
            &[
                "調べなかった",
                "測らなかった",
                "確かめなかった",
                "試さなかった",
                "触らなかった",
            ],
        )
    })
}

const GROUND: &[&str] = &["根拠", "証拠", "裏付け", "確証"];
const ABSENT: &[&str] = &[
    "得られていません",
    "得られていない",
    "得られなかった",
    "ありません",
    "なかった",
    "ない",
    "無い",
];

const NOT_COPULA: &[&str] = &["ではありません", "ではない", "じゃない"];
/// 「わけではない」。直前が過去の「た」「だ」のとき (「全てを調べたわけではない」) だけ拾い、
/// 「常に失敗するわけではない」のような性質の説明を除く。
const WAKE: &[&str] = &["わけではありません", "わけではない", "訳ではない"];
/// 範囲の打ち消しの前半。この後ろの、次の読点までの「ではない」を拾う。
const SCOPE_HEAD: &[&str] = &["であって", "であり", "にすぎず", "に過ぎず", "という"];

const OUT_OF_SCOPE: &[&str] = &["範囲外", "対象外", "スコープ外"];
/// 範囲外の語の前に置ける、文書や作業を指す語。空の文字列は助詞の直後に範囲外が続く形。
const SCOPE_OWNER: &[&str] = &[
    "今回の",
    "本書の",
    "本稿の",
    "本計画の",
    "この計画の",
    "この文書の",
    "本作業の",
    "この作業の",
    "本変更の",
    "この変更の",
    "本PRの",
    "このPRの",
    "今回は",
    "",
];

/// 範囲の打ち消しで打ち消す、作業の成果や保証を指す名詞。この語で終わる名詞の「ではない」だけを拾う。
const ASSURANCE: &[&str] = &[
    "保証",
    "証明",
    "実装",
    "実測",
    "確定",
    "決定",
    "結論",
    "対策",
    "最終版",
    "完成版",
    "特定",
    "報告",
    "検証",
    "確認",
    "方針",
    "計測",
    "測定",
    "評価",
    "修正",
    "解決",
    "合格",
    "適用",
    "仕様",
];

/// 述語の直後に来ると、条件か名詞化か修飾として除く語。
const NOT_PREDICATE: &[&str] = &[
    "と",
    "場合",
    "なら",
    "ければ",
    "とき",
    "時",
    "限り",
    "間",
    "うち",
    "こと",
    "もの",
    "ように",
    "ら",
    "まま",
    "ほう",
    "ところ",
    "やり方",
    "か",
    "り",
];

fn starts_with(s: &[char], at: usize, lit: &str) -> bool {
    (at..).zip(lit.chars()).all(|(i, c)| s.get(i) == Some(&c))
}

fn len(lit: &str) -> usize {
    lit.chars().count()
}

fn is_noun_start(c: char) -> bool {
    ('\u{4e00}'..='\u{9fff}').contains(&c)
        || ('\u{30a0}'..='\u{30ff}').contains(&c)
        || c.is_ascii_alphanumeric()
        || c == '\u{0}'
}

/// 述語の後ろが、文の述語として終わっているか。
fn predicate_end(s: &[char], end: usize) -> bool {
    let Some(&next) = s.get(end) else { return true };
    if NOT_PREDICATE.iter().any(|w| starts_with(s, end, w)) {
        return false;
    }
    !is_noun_start(next)
}

/// `at` から、副詞と述語の一覧のどれかを照合し、文の述語として終わる位置を返す。
fn predicate_at(s: &[char], at: usize, list: &[String]) -> Option<usize> {
    for adv in ADVERBS {
        if !starts_with(s, at, adv) {
            continue;
        }
        let from = at + len(adv);
        for u in list {
            if starts_with(s, from, u) && (u.ends_with('、') || predicate_end(s, from + len(u))) {
                return Some(from + len(u));
            }
        }
    }
    None
}

/// 文末が過去か「ている」の形 (した、している、しました) か。打ち消しの連用中止形 (「〜は行わず、」) は時制を
/// 持たないので、文末の形で、設計や手順の文 (「再試行はせず、stalled にする」) と作業の報告を分ける。
fn ends_as_report(s: &[char]) -> bool {
    let end = s
        .iter()
        .rposition(|c| !matches!(c, ')' | '\u{ff09}' | '」' | '\u{0}' | '*' | '_'))
        .map_or(0, |k| k + 1);
    let body = &s[..end];
    ["た", "ている", "ていた", "ています", "ました", "でした"]
        .iter()
        .any(|w| body.len() >= len(w) && starts_with(body, body.len() - len(w), w))
}
/// 型 A、D、E: 作業の名詞か読む目的語から始まる、未完了、範囲外、省略の報告。
fn work_at(s: &[char], i: usize) -> Option<usize> {
    // 目的語を問わない作業の動詞 (「影響の有無は調べていない」)
    if i > 0
        && ['を', 'は', 'も'].contains(&s[i - 1])
        && let Some(end) = predicate_at(s, i, verb_undone())
    {
        return Some(end);
    }
    if i > 0
        && ['は', 'も'].contains(&s[i - 1])
        && let Some(end) = predicate_at(s, i, zu()).filter(|_| ends_as_report(s))
    {
        return Some(end);
    }
    for w in READ_OBJECT {
        if !starts_with(s, i, w) {
            continue;
        }
        for p in ["までは", "は", "も", "を", ""] {
            let at = i + len(w);
            if starts_with(s, at, p)
                && let Some(end) = predicate_at(s, at + len(p), read_undone())
            {
                return Some(end);
            }
        }
    }
    for w in WORK {
        if !starts_with(s, i, w) {
            continue;
        }
        let after = i + len(w);
        for p in PARTICLES {
            if !starts_with(s, after, p) {
                continue;
            }
            let at = after + len(p);
            if let Some(end) = predicate_at(s, at, undone()) {
                return Some(end);
            }
            if ["は", "も"].contains(p)
                && let Some(end) = predicate_at(s, at, passive())
                    .or_else(|| predicate_at(s, at, zu()).filter(|_| ends_as_report(s)))
            {
                return Some(end);
            }
            if ["は", "も", "を"].contains(p) {
                for adv in ADVERBS {
                    let from = at + len(adv);
                    if !starts_with(s, at, adv) {
                        continue;
                    }
                    for x in SKIPPED {
                        let end = from + len(x);
                        if starts_with(s, from, x) && (x.ends_with('、') || predicate_end(s, end))
                        {
                            return Some(end);
                        }
                    }
                }
            }
            // D: 助詞の直後、または文書や作業を指す語の直後の「範囲外」。「実行」は仕組みの動作も指すので除く
            if !["実行", "変更"].contains(w) && ["は", "も", "を", "については"].contains(p)
            {
                for owner in SCOPE_OWNER {
                    if !starts_with(s, at, owner) {
                        continue;
                    }
                    let o_at = at + len(owner);
                    if let Some(o) = OUT_OF_SCOPE.iter().find(|o| starts_with(s, o_at, o))
                        && !starts_with(s, o_at + len(o), "の")
                        && !starts_with(s, o_at + len(o), "なら")
                        && !s.get(o_at + len(o)).is_some_and(|c| is_noun_start(*c))
                    {
                        return Some(o_at + len(o));
                    }
                }
            }
        }
    }
    None
}

/// 型 B: 根拠の欠如。
fn ground_at(s: &[char], i: usize) -> Option<usize> {
    for g in GROUND {
        if !starts_with(s, i, g) {
            continue;
        }
        let after = i + len(g);
        for p in ["が", "は", "も"] {
            if !starts_with(s, after, p) {
                continue;
            }
            for adv in ["特に", "全く", "一切", "まだ", ""] {
                let from = after + 1 + len(adv);
                if !starts_with(s, after + 1, adv) {
                    continue;
                }
                for a in ABSENT {
                    if starts_with(s, from, a) && predicate_end(s, from + len(a)) {
                        return Some(from + len(a));
                    }
                }
            }
        }
    }
    None
}

/// 「わけではない」の前が、作業の報告でなく理由の否定や状態の変化の説明か。「下げたからといって、減ったわけではない」
/// 「非推奨になっただけで、削除されたわけではない」のような文を除く。
fn wake_states_a_change(s: &[char], i: usize) -> bool {
    let before = &s[..i];
    let has = |w: &str| (0..before.len()).any(|k| starts_with(before, k, w));
    has("からといって")
        || has("だけで")
        || ["れた", "になった"]
            .iter()
            .any(|w| i >= len(w) && starts_with(s, i - len(w), w))
}

/// 型 C の一部: 保証と証明の打ち消し。前半の「であり」が無くても、文書では範囲の打ち消しである
/// (「実モデルが従う保証ではない」)。プログラムのコメントでは「権限の保証ではない」のような説明に使うので外す。
fn assurance_at(s: &[char], i: usize) -> Option<usize> {
    for a in ["保証", "証明"] {
        for n in NOT_COPULA {
            if starts_with(s, i, a)
                && starts_with(s, i + len(a), n)
                && predicate_end(s, i + len(a) + len(n))
            {
                return Some(i + len(a) + len(n));
            }
        }
    }
    None
}
/// 型 C: 範囲の打ち消し。
fn scope_at(s: &[char], i: usize) -> Option<usize> {
    for w in WAKE {
        if starts_with(s, i, w)
            && i > 0
            && ['た', 'だ'].contains(&s[i - 1])
            && !wake_states_a_change(s, i)
            && predicate_end(s, i + len(w))
        {
            return Some(i + len(w));
        }
    }
    for head in SCOPE_HEAD {
        if !starts_with(s, i, head) {
            continue;
        }
        let limit = if *head == "という" { 15 } else { 40 };
        let mut from = i + len(head);
        // 前半の直後の読点は読み飛ばす
        if *head != "という" && s.get(from) == Some(&'、') {
            from += 1;
        }
        for j in from..s.len().min(from + limit) {
            if matches!(s[j], '、' | ',') {
                break;
            }
            for n in NOT_COPULA {
                // 打ち消す語が、作業の成果や保証を指す名詞のときだけ拾う。「相対パスであって絶対パスではない」
                // のような定義の対比を除く
                if starts_with(s, j, n)
                    && ASSURANCE
                        .iter()
                        .any(|a| j >= len(a) && starts_with(s, j - len(a), a))
                    && predicate_end(s, j + len(n))
                {
                    return Some(j + len(n));
                }
            }
        }
    }
    None
}

/// 1 文の中で最初に当たった位置と終わりを返す。
fn find(s: &[char], prose: Prose) -> Option<(usize, usize)> {
    for i in 0..s.len() {
        if prose == Prose::Document
            && let Some(end) = work_at(s, i).or_else(|| assurance_at(s, i))
        {
            return Some((i, end));
        }
        if let Some(end) = ground_at(s, i).or_else(|| scope_at(s, i)) {
            return Some((i, end));
        }
    }
    None
}

fn boundary(chars: &[char], k: usize) -> bool {
    let c = chars[k];
    if c == '.' {
        return !(k > 0
            && chars[k - 1].is_ascii_alphanumeric()
            && chars.get(k + 1).is_some_and(|n| n.is_ascii_alphanumeric()));
    }
    matches!(c, '。' | '！' | '？' | '!' | '?' | ';' | '；')
}

/// 本文の単位 (`mixed::prose_units` が作る) を検査する。空白を挟む表記にも同じ判定を適用し、位置は
/// 元の本文で返す。
pub fn scan_units(units: &[Vec<Cell>], prose: Prose) -> Vec<Hit> {
    let mut hits = Vec::new();
    for unit in units {
        // 可視の文字だけを、元の位置を保って並べる。引用とコードは区切りの印 '\u{0}' に替える
        let mut chars = Vec::new();
        let mut pos = Vec::new();
        for cell in unit {
            if cell.c.is_whitespace() {
                continue;
            }
            chars.push(if cell.hidden { '\u{0}' } else { cell.c });
            pos.push((cell.line, cell.col));
        }
        let mut start = 0;
        for k in 0..=chars.len() {
            if k < chars.len() && !boundary(&chars, k) {
                continue;
            }
            let s = &chars[start..k];
            if let Some((a, b)) = find(s, prose) {
                let (line, col) = pos[start + a];
                hits.push(Hit {
                    line: line + 1,
                    col: col + 1,
                    excerpt: s[a..b].iter().collect(),
                });
            }
            start = k + 1;
        }
    }
    hits
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mixed::{Layout, prose_units};
    use crate::scan::Document;

    fn scan(body: &str) -> Vec<Hit> {
        scan_units(
            &prose_units(&Document::parse(body), &Layout::plain()),
            Prose::Document,
        )
    }

    #[test]
    fn preserves_character_positions_after_wrapping() {
        let hits = scan("見出し\n\n検証は\n終わっていない。");
        assert_eq!((hits[0].line, hits[0].col), (3, 1));
        assert_eq!(hits[0].excerpt, "検証は終わっていない");
    }

    #[test]
    fn reports_one_hit_per_sentence() {
        assert_eq!(scan("検証はしておらず、再現も試していない。").len(), 1);
        assert_eq!(scan("検証はしていない。再現も試していない。").len(), 2);
    }

    #[test]
    fn comments_skip_the_work_report_forms() {
        let units = prose_units(&Document::parse("起動を検証していない。"), &Layout::plain());
        assert!(scan_units(&units, Prose::Comment).is_empty());
        assert_eq!(scan_units(&units, Prose::Document).len(), 1);
        let units = prose_units(
            &Document::parse("この上限には根拠がない。"),
            &Layout::plain(),
        );
        assert_eq!(scan_units(&units, Prose::Comment).len(), 1);
    }
}
