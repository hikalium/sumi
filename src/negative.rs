//! 操作者が禁止した否定構文を、本文の単位と文の境界に沿って検査する。

use crate::mixed::Cell;

pub const FORMS: &str =
    "〜や〜はしていない / 〜には〜ない / 〜であり〜ではない / 〜という〜ではない / 〜は〜ない";
pub const WRITE: &str = crate::rules::with_purpose_check!(
    "事実・現在の状態・対象・次の行動を直接述べる。未実施や制約が判断に必要なら、その状態と必要な手順を具体的に書く"
);

pub fn render_line() -> String {
    format!(
        "{FORMS} -> {WRITE} | keep: 引用 / インラインコード / フェンス | note: 操作者が指定した5構文を禁止する。丁寧形も同じ規則で扱う。"
    )
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

fn boundary(unit: &[Cell], i: usize) -> bool {
    let c = unit[i].c;
    if c == '.' {
        return !(i > 0
            && unit[i - 1].c.is_ascii_alphanumeric()
            && unit.get(i + 1).is_some_and(|k| k.c.is_ascii_alphanumeric()));
    }
    matches!(c, '。' | '！' | '？' | '!' | '?' | ';' | '；')
}

/// 本文の単位 (`mixed::prose_units` が作る) を検査する。空白を挟む表記にも同じ判定を適用し、位置は
/// 元の本文で返す。
pub fn scan_units(units: &[Vec<Cell>]) -> Vec<Hit> {
    let mut hits = Vec::new();
    for unit in units {
        let mut topic = None;
        let mut sentence_start = 0;
        for (i, cell) in unit.iter().enumerate() {
            if cell.hidden {
                continue;
            }
            if boundary(unit, i) {
                topic = None;
                sentence_start = i + 1;
                continue;
            }
            if cell.c == 'は' && i > sentence_start && !inside_word(unit, i) {
                topic = Some(i);
            }
            let Some(start) = topic else { continue };
            if ['な', '無', 'ま', 'あ'].contains(&cell.c)
                && let Some(end) = negative_end(unit, i)
            {
                let excerpt = unit[start..end]
                    .iter()
                    .map(|k| k.c)
                    .collect::<String>()
                    .split_whitespace()
                    .collect::<Vec<_>>()
                    .join(" ");
                hits.push(Hit {
                    line: unit[start].line + 1,
                    col: unit[start].col + 1,
                    excerpt,
                });
                topic = None;
            }
        }
    }
    hits
}

fn negative_end(unit: &[Cell], start: usize) -> Option<usize> {
    if inside_word(unit, start) {
        return None;
    }
    // 名詞や数値を並べる選択接続詞を、否定形に続く接続助詞から分ける。
    if unit
        .get(start..start + 3)
        .is_some_and(|part| part.iter().map(|k| k.c).eq("ないし".chars()))
        && unit[..start]
            .iter()
            .rev()
            .find(|k| !k.c.is_whitespace())
            .is_some_and(|k| {
                k.hidden || k.c.is_ascii_alphanumeric() || ('\u{4e00}'..='\u{9fff}').contains(&k.c)
            })
    {
        return None;
    }
    for ending in ["ありません", "ません", "ない", "無い"] {
        let mut end = start;
        let mut matched = true;
        for expected in ending.chars() {
            while end < unit.len() && unit[end].c.is_whitespace() {
                end += 1;
            }
            if unit.get(end).is_none_or(|k| k.hidden || k.c != expected) {
                matched = false;
                break;
            }
            end += 1;
        }
        if matched && predicate_end(unit, end) {
            return Some(end);
        }
    }
    None
}

/// 基本形容詞と語中の文字を、否定の述語・助詞から分ける。
fn inside_word(unit: &[Cell], at: usize) -> bool {
    const WORDS: &[&str] = &[
        "少ない",
        "すくない",
        "危ない",
        "あぶない",
        "切ない",
        "せつない",
        "きたない",
        "つたない",
        "はかない",
        "もったいない",
        "こないだ",
        "はじめ",
        "はっきり",
        "はるか",
        "はしゃ",
        "はたら",
        "はなれ",
        "はしる",
        "はしっ",
    ];
    WORDS.iter().any(|word| {
        let chars: Vec<char> = word.chars().collect();
        (at.saturating_sub(chars.len() - 1)..=at).any(|start| {
            unit.get(start..start + chars.len())
                .is_some_and(|part| part.iter().zip(&chars).all(|(k, c)| !k.hidden && k.c == *c))
        })
    })
}

/// 否定形の直後を確認し、活用語の途中での一致を除く。
fn predicate_end(unit: &[Cell], end: usize) -> bool {
    let tail: String = unit[end..].iter().map(|k| k.c).collect();
    let tail = tail.trim_start();
    let Some(next) = tail.chars().next() else {
        return true;
    };
    matches!(
        next,
        '。' | '、'
            | ','
            | '.'
            | '!'
            | '?'
            | '！'
            | '？'
            | ';'
            | '；'
            | ')'
            | ']'
            | '」'
            | '"'
            | '*'
            | '_'
            | ':'
    ) || ('\u{4e00}'..='\u{9fff}').contains(&next)
        || [
            "です",
            "でした",
            "でしょう",
            "だろう",
            "と",
            "ので",
            "の",
            "ため",
            "が",
            "けれど",
            "けど",
            "から",
            "もの",
            "こと",
            "よう",
            "わけ",
            "はず",
            "まま",
            "で",
            "か",
            "なら",
            "し",
        ]
        .iter()
        .any(|suffix| tail.starts_with(suffix))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mixed::{Layout, prose_units};
    use crate::scan::Document;

    fn scan(body: &str, layout: &Layout) -> Vec<Hit> {
        scan_units(&prose_units(&Document::parse(body), layout))
    }

    #[test]
    fn rejects_the_five_forms_and_polite_endings_once_each() {
        for body in [
            "実装や実験はしていない。",
            "この結果には根拠がない。",
            "調査であり実装ではない。",
            "完了という報告ではない。",
            "検証は終わっていない。",
            "製品コードの修正・比較実験は実施していません。",
            "この結果には根拠がありません。",
            "調査であり実装ではありません。",
            "完了という報告ではありません。",
            "設定は無い。",
            "結果は\n確認していない。",
            "結果は確認していな\nい。",
            "対象は `main` ではない。",
            "仕様は v1.2 に準拠していない。",
            "数は少ないとはいえない。",
            "検証はしていないし、実装も未着手である。",
            "検証はしないし、実装も未着手である。",
        ] {
            assert_eq!(scan(body, &Layout::plain()).len(), 1, "{body}");
        }
    }

    #[test]
    fn keeps_direct_statements_quotes_code_and_independent_units() {
        for body in [
            "実装と実験は承認後に進める。",
            "設定は未登録である。",
            "対象は調査である。",
            "「検証は終わっていない」と引用する。",
            "\"検証は終わっていない\"と引用する。",
            "`検証は終わっていない`という例。",
            "```\n検証は終わっていない。\n```",
            "結果は確認した。追加しない。",
            "結果は確認した\n\n追加しない。",
            "- 結果は確認した\n- 追加しない。",
            "| 結果は確認した | 追加しない |",
            "機器はケーブルでつないだ。",
            "準備をはじめ、変更しない。",
            "数は少ない。",
            "道は危ない。",
            "結果は甲ないし乙である。",
        ] {
            assert!(scan(body, &Layout::plain()).is_empty(), "{body}");
        }
    }

    #[test]
    fn preserves_character_positions_after_wrapping() {
        let hits = scan("見出し\n\n検証は\n終わっていない。", &Layout::plain());
        assert_eq!((hits[0].line, hits[0].col), (3, 3));
        assert_eq!(hits[0].excerpt, "は 終わっていない");
    }
}
