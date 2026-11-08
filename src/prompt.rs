//! 同梱プロンプトの出力。prompt/rewrite.md の本文の末尾に、規則の一覧を付ける。

use crate::rules::Rule;

/// 改善プロンプトの本文
pub const BODY: &str = include_str!("../prompt/rewrite.md");

/// プロンプトとヒットの行を分ける見出し。呼び出し側はこの行より後をヒットの行として読める。
pub const HITS_HEADING: &str = "## 検出結果";

/// 規則の一覧の各行。語句の規則の後に、混在、否定構文、漢字 1 字の語の検査を 1 行ずつ並べる。`--rules` の出力と
/// 改善プロンプトの規則の一覧が、この 1 か所で組んだ行を使う。
pub fn rule_lines(rules: &[Rule]) -> Vec<String> {
    let mut lines: Vec<String> = rules.iter().map(Rule::render_line).collect();
    lines.push(crate::mixed::render_line());
    lines.push(crate::negative::render_line());
    lines.push(crate::single::render_line());
    lines
}

/// 検査結果の前に出す本文と規則の一覧。
pub fn render(rules: &[Rule]) -> String {
    let mut out = String::from(BODY);
    if !out.ends_with('\n') {
        out.push('\n');
    }
    out.push_str("\n## 規則の一覧\n\n");
    out.push_str("形は `avoid -> write | keep: ... | note: ...` で、avoid が避ける書き方、write が案、keep は同じ字を含むが別の語、note が理由である。\n\n");
    for line in rule_lines(rules) {
        out.push_str("- ");
        out.push_str(&line);
        out.push('\n');
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rules::RULES;

    #[test]
    fn prompt_carries_the_body_and_every_rule() {
        let out = render(RULES);
        assert!(out.starts_with(BODY));
        for r in RULES {
            assert!(out.contains(&format!("- {}\n", r.render_line())));
        }
        assert!(out.contains(&format!("- {}\n", crate::mixed::render_line())));
        assert!(out.contains(&format!("- {}\n", crate::negative::render_line())));
        assert!(out.ends_with(&format!("- {}\n", crate::single::render_line())));
    }

    #[test]
    fn body_names_the_role_the_constraints_and_the_check() {
        for needle in [
            "役割",
            "守ること",
            "確認",
            "意味と事実を変えない",
            "コードフェンス",
            "インラインコード",
            "sumi",
            "規則",
        ] {
            assert!(BODY.contains(needle), "{needle:?} が本文に無い");
        }
    }
}
