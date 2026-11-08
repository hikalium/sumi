//! 漢字 1 字の語の検査。英語の術語を漢字 1 字に置き換えた語 (section を「節」、page を「頁」、group を
//! 「束」) を、語の表を足していく前に形で拾う。前後に漢字もカタカナも無く、助詞か読点か句点か括弧が
//! 続く漢字 1 字を語とみなし、許可リスト `ALLOWED` に無ければヒットにする。

use crate::mixed::Cell;

/// 検出結果の行の `->` の右に出す、修正方法の案。
pub const WRITE: &str = "国語辞典に載る意味で読んで文の意味と合わない語は、カタカナの術語か、何が起きるかを書く動詞の文に替える。合う語は sumi の src/single.rs の ALLOWED に足す";

/// 1 字の語の後に続くとみなす助詞と記号。先頭一致で見る。1 字の「か」「ま」「だ」を足すと、「書かない」
/// 「止まる」のような送り仮名も拾うので、この表から外す。
const FOLLOWERS: &[&str] = &[
    "が",
    "の",
    "を",
    "に",
    "へ",
    "と",
    "で",
    "は",
    "も",
    "や",
    "から",
    "まで",
    "だけ",
    "など",
    "ごと",
    "ずつ",
    "こそ",
    "より",
    "です",
    "である",
    "、",
    "。",
    "(",
];

/// 1 字の語の後に助詞の字が続いても、動詞の送り仮名と読む形 (「転がる」「冷やす」「秀でる」)。
const OKURIGANA: &[&str] = &[
    "がる", "がり", "がっ", "がれ", "がろ", "やす", "やし", "やさ", "やせ", "でる", "でた", "でて",
];

/// 許す漢字 1 字の語。文書にあった語に、数字と、普通の文章で単独で使う語 (念、事、冊、朝) を
/// 足して作った。語を足す前に国語辞典に載る意味で読み、文の意味と合うかを試す。
pub const ALLOWED: &str = concat!(
    "一七万三上下両中丸主乗九事二五人今他仮件位低体何例係個倍値側偽像億元兆先全八六共",
    "内円冊冬処分列別前剥割力勢化十千半単印又口句可台右各名周和問嘘器四回図国地型埋基",
    "場塊塞境増壁声夏外夜大奥姿子字孫宙宛実害家宿対小尾層左差帯常幅年幹幻床度式弧弾当",
    "形影役後従心念急意所手択指掛損故数敵文新方族旗既日旧旨春昼時晩暗暦更最月有朝期木",
    "末本札条来枚枝核桁案棒標権横橙機欄次正歩死殻毎比気法波流済減源炭点無熱版物特率",
    "玉現生産用略番癖発百的皿目直相真石砦社票秋秒稀税種積穏穴空章端第等筋答策算管箱糸",
    "系約紙級素組継網線縦罠罪群者肝脇脳腕色芯落葉行術表裏裸要見親角解言註話語読誰論谷",
    "負質赤超足跡跨身軸輪轍辺逃逆通週進道選部重量金錨鍵鎖長門間陰陽隙際隣面音頃項順頭",
    "題額顔類風高",
);

/// `--rules` と改善プロンプトの規則の一覧に出す 1 行。形は src/rules.rs の `Rule::render_line` に揃える。
pub fn render_line() -> String {
    format!(
        "(前後に漢字もカタカナも無く、助詞か読点か句点か括弧が続く漢字 1 字の語のうち、許可リストに無いもの) -> {WRITE} | \
         keep: 許可リストの {} 語 / 「」の中の引用 / インラインコード / フェンス | \
         note: 英語の術語 (section、page、group) を漢字 1 字 (節、頁、束) に置き換えた語は、\
         日本語の読み手に意味が通じにくい。",
        ALLOWED.chars().count()
    )
}

/// 1 字の語のヒット 1 件。`line` と `col` は 1 始まりで、`col` は文字 (char) 単位。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Hit {
    pub line: usize,
    pub col: usize,
    pub word: char,
}

impl Hit {
    /// `LINE:COL: 「語」-> 直し方` の形。規則のヒットと同じ形にする。
    pub fn render(&self) -> String {
        format!("{}:{}: 「{}」-> {WRITE}", self.line, self.col, self.word)
    }
}

/// 漢字の範囲。CJK 統合漢字、拡張 A、互換漢字と「々」「〆」を見る。拡張 B 以降は今の文書で使われた例が 0 件なので外す。
fn is_kanji(c: char) -> bool {
    matches!(c, '\u{4E00}'..='\u{9FFF}' | '\u{3400}'..='\u{4DBF}' | '\u{F900}'..='\u{FAFF}' | '々' | '〆')
}

/// 全角のカタカナ。半角のカタカナは今の文書で使われた例が 0 件なので外す。
fn is_katakana(c: char) -> bool {
    matches!(c, '\u{30A1}'..='\u{30FF}' | '\u{31F0}'..='\u{31FF}')
}

/// 語の境目を探すときに飛ばす字。空白 (行の境目の除外の空白を含む)、引用の `>`、強調の `*` である。
fn skipped(k: &Cell) -> bool {
    matches!(k.c, ' ' | '\t') || (!k.hidden && matches!(k.c, '>' | '*'))
}

/// `i` の前で、飛ばす字を除いた最初の字。
fn before(unit: &[Cell], i: usize) -> Option<&Cell> {
    unit[..i].iter().rev().find(|k| !skipped(k))
}

/// `i` の後で、飛ばす字を除いた最初の字から始まる、除外の外の 3 字まで。
fn after(unit: &[Cell], i: usize) -> String {
    unit[i + 1..]
        .iter()
        .skip_while(|k| skipped(k))
        .take_while(|k| !k.hidden)
        .take(3)
        .map(|k| k.c)
        .collect()
}

/// 本文の単位 (`mixed::prose_units` が作る) を検査する。位置は元の本文で返す。
pub fn scan_units(units: &[Vec<Cell>]) -> Vec<Hit> {
    let mut hits = Vec::new();
    for unit in units {
        for (i, cell) in unit.iter().enumerate() {
            if cell.hidden || !is_kanji(cell.c) || matches!(cell.c, '々' | '〆') {
                continue;
            }
            if before(unit, i).is_some_and(|p| !p.hidden && (is_kanji(p.c) || is_katakana(p.c))) {
                continue;
            }
            let next = after(unit, i);
            if OKURIGANA.iter().any(|o| next.starts_with(o))
                || !FOLLOWERS.iter().any(|f| next.starts_with(f))
            {
                continue;
            }
            if ALLOWED.contains(cell.c) {
                continue;
            }
            hits.push(Hit {
                line: cell.line + 1,
                col: cell.col + 1,
                word: cell.c,
            });
        }
    }
    hits
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mixed::{Layout, prose_units};
    use crate::scan::Document;

    fn words(text: &str) -> Vec<char> {
        let units = prose_units(&Document::parse(text), &Layout::plain());
        scan_units(&units).iter().map(|h| h.word).collect()
    }

    #[test]
    fn a_single_kanji_word_before_a_particle_is_a_hit() {
        assert_eq!(words("この節の手順を読む。\n"), vec!['節']);
        assert_eq!(words("前の頁を見る。\n"), vec!['頁']);
    }

    #[test]
    fn neighbors_that_make_a_longer_word_are_not_hits() {
        // 前後の漢字、前のカタカナ、助詞でない後続
        assert_eq!(
            words("この章節の手順と、ノード群を見て、書く。\n"),
            Vec::<char>::new()
        );
    }

    #[test]
    fn quotes_and_code_are_not_hits() {
        assert_eq!(
            words("「節」は使わない語で、`節` もコードである。\n"),
            Vec::<char>::new()
        );
    }

    #[test]
    fn a_word_wrapped_across_lines_is_a_hit() {
        assert_eq!(words("この手順は次の節\nの最後に書く。\n"), vec!['節']);
        assert_eq!(words("- この手順は次の節\n  の最後に書く。\n"), vec!['節']);
        assert_eq!(words("> この手順は次の節\n> の最後に書く。\n"), vec!['節']);
    }

    #[test]
    fn longer_particles_punctuation_and_brackets_follow_a_word() {
        assert_eq!(words("この筐から次の筐までを読む。\n"), vec!['筐', '筐']);
        assert_eq!(words("筐、匣。\n"), vec!['筐', '匣']);
        assert_eq!(words("この筐 (case) と **匣**を読む。\n"), vec!['筐', '匣']);
    }

    #[test]
    fn okurigana_that_looks_like_a_particle_is_not_a_hit() {
        assert_eq!(
            words("石が転がり、GPU を冷やし、腕に秀でる。\n"),
            Vec::<char>::new()
        );
    }

    #[test]
    fn allowed_words_are_not_hits() {
        for c in ALLOWED.chars() {
            assert_eq!(
                words(&format!("この{c}が要る。\n")),
                Vec::<char>::new(),
                "{c}"
            );
        }
    }

    #[test]
    fn allowed_words_are_single_kanji_without_duplicates() {
        let mut seen = std::collections::HashSet::new();
        for c in ALLOWED.chars() {
            assert!(is_kanji(c), "{c}");
            assert!(seen.insert(c), "{c}");
        }
    }
}
