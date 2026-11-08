//! 原文と改善例、および留保のシグナルの CLI 検査。

use std::path::Path;
use std::process::{Command, Output};

const PURPOSE_CHECK: &str = "そもそもこの文が文章の目的に照らして必須かどうかを判断し、そうでないなら削除することも検討しなさい";

fn check(path: &Path, args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_sumi"))
        .args(args)
        .arg("--file")
        .arg(path)
        .output()
        .expect("sumi を起動できること")
}

#[test]
fn every_anti_case_hits_its_signal_and_every_rewrite_is_clean() {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("anti-cases");
    let cases = [
        ("01-link-cause", vec!["は未実施", "は未確定"]),
        ("02-module-spec", vec!["は未実施"]),
        ("03-diagnostics", vec!["は未実施"]),
        ("04-link-status", vec!["は未実施"]),
        ("05-serdes-mode", vec!["は未確定"]),
        ("06-procedure-scope", vec!["は未検証"]),
        ("07-gpio-correction", vec!["は未確認"]),
        ("08-i2c-arguments", vec!["別途検証する必要がある"]),
        ("09-redaction", vec!["が必要になる。"]),
        ("10-published-dump", vec!["に限る。"]),
        ("11-sdk-source", vec!["は未確認"]),
        ("12-i2c-interface", vec!["と決めつけない"]),
        ("13-diagnostic-device-id", vec!["取り違えに注意"]),
    ];
    for suffix in [".before.md", ".after.md"] {
        let count = std::fs::read_dir(&dir)
            .unwrap()
            .filter(|entry| {
                entry
                    .as_ref()
                    .unwrap()
                    .file_name()
                    .to_str()
                    .unwrap()
                    .ends_with(suffix)
            })
            .count();
        assert_eq!(count, cases.len(), "{suffix}: すべての対を検査する");
    }
    let rule_output = Command::new(env!("CARGO_BIN_EXE_sumi"))
        .arg("--rules")
        .output()
        .unwrap();
    assert!(rule_output.status.success());
    let rules = String::from_utf8(rule_output.stdout).unwrap();
    assert_eq!(rules.matches(PURPOSE_CHECK).count(), 8);
    assert!(
        rules
            .lines()
            .find(|line| line.starts_with("〜や〜はしていない / "))
            .unwrap()
            .contains(PURPOSE_CHECK)
    );
    for (name, signals) in cases {
        let before = check(&dir.join(format!("{name}.before.md")), &["--check-only"]);
        let hits = String::from_utf8(before.stdout).unwrap();
        assert_eq!(before.status.code(), Some(1), "{name}: {hits}");
        assert!(before.stderr.is_empty(), "{name}");
        assert_eq!(hits.lines().count(), signals.len(), "{name}: {hits}");
        for line in hits.lines() {
            let (_, write) = line.split_once("-> ").unwrap();
            assert_eq!(write.matches(PURPOSE_CHECK).count(), 1, "{name}: {line}");
        }
        for signal in signals {
            assert!(hits.contains(&format!("「{signal}」")), "{name}: {hits}");
            let rule = rules
                .lines()
                .find(|line| {
                    line.split_once(" -> ")
                        .unwrap()
                        .0
                        .split(" / ")
                        .any(|avoid| avoid == signal)
                })
                .unwrap();
            let (_, proposal) = rule.split_once(" -> ").unwrap();
            assert!(
                proposal
                    .split_once(" | keep: ")
                    .unwrap()
                    .0
                    .contains(PURPOSE_CHECK),
                "{rule}"
            );
        }
        let after = check(&dir.join(format!("{name}.after.md")), &["--check-only"]);
        assert_eq!(after.status.code(), Some(0), "{name}: {after:?}");
        assert!(after.stdout.is_empty(), "{name}");
        assert!(after.stderr.is_empty(), "{name}");
    }
}

#[test]
fn signals_reach_all_output_modes_with_contextual_rewrite_instructions() {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("anti-cases/09-redaction.before.md");
    let normal = check(&path, &[]);
    assert_eq!(normal.status.code(), Some(1));
    let stdout = String::from_utf8(normal.stdout).unwrap();
    assert!(stdout.contains("必要な不確かさ、安全上の注意、公開範囲、検証条件は保つ"));
    assert!(stdout.contains("「が必要になる。」"));
    let (prompt, hits) = stdout.split_once("## 検出結果\n\n").unwrap();
    assert!(prompt.contains(PURPOSE_CHECK));
    assert_eq!(hits.matches(PURPOSE_CHECK).count(), 1);
    let quiet = check(&path, &["--silent"]);
    assert_eq!(quiet.status.code(), Some(1));
    assert!(quiet.stdout.is_empty());
    assert!(quiet.stderr.is_empty());
}

#[test]
fn signals_only_scan_prose_in_each_language_and_keep_original_positions() {
    use std::io::Write;
    use std::process::Stdio;

    for (lang, body, position) in [
        (
            "markdown",
            "`監視は未実施。`\n監視は未実施。\n「監視は未実施。」\n```\n監視は未実施。\n```",
            "2:3:",
        ),
        (
            "rust",
            "let s = \"監視は未実施。\";\n// 監視は未実施。",
            "2:6:",
        ),
        (
            "html",
            "<code>監視は未実施。</code>\n<p>監視は未実施。</p>",
            "2:6:",
        ),
    ] {
        let mut child = Command::new(env!("CARGO_BIN_EXE_sumi"))
            .args(["--lang", lang, "--check-only"])
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap();
        child
            .stdin
            .take()
            .unwrap()
            .write_all(body.as_bytes())
            .unwrap();
        let output = child.wait_with_output().unwrap();
        assert_eq!(output.status.code(), Some(1), "{lang}");
        assert!(output.stderr.is_empty(), "{lang}");
        let hits = String::from_utf8(output.stdout).unwrap();
        assert_eq!(hits.lines().count(), 1, "{lang}: {hits}");
        assert!(hits.starts_with(position), "{lang}: {hits}");
        assert!(hits.contains("「は未実施」"), "{lang}: {hits}");
    }
}

#[test]
fn purpose_check_reaches_negative_hits_without_changing_unrelated_hits() {
    use std::io::Write;
    use std::process::Stdio;

    for (args, body, code, guided) in [
        (vec!["--check-only"], "実装はしていない。", 1, true),
        (vec![], "実装はしていない。", 1, true),
        (vec!["--silent"], "実装はしていない。", 1, false),
        (vec!["--check-only"], "独自算術", 1, false),
        (vec!["--check-only"], "自然な日本語の文章。", 0, false),
    ] {
        let mut child = Command::new(env!("CARGO_BIN_EXE_sumi"))
            .args(&args)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap();
        child
            .stdin
            .take()
            .unwrap()
            .write_all(body.as_bytes())
            .unwrap();
        let result = child.wait_with_output().unwrap();
        assert_eq!(result.status.code(), Some(code), "{args:?}: {body}");
        assert!(result.stderr.is_empty());
        let stdout = String::from_utf8(result.stdout).unwrap();
        let hits = stdout
            .split_once("## 検出結果\n\n")
            .map_or(stdout.as_str(), |(_, hits)| hits);
        assert_eq!(hits.contains(PURPOSE_CHECK), guided, "{args:?}: {stdout}");
        if guided {
            assert_eq!(hits.matches(PURPOSE_CHECK).count(), 1);
            assert!(hits.contains("事実・現在の状態・対象・次の行動を直接述べる"));
        } else if body == "独自算術" {
            assert_eq!(stdout, "1:1: 「独自算術」-> 独自の計算ロジック\n");
        } else {
            assert!(stdout.is_empty());
        }
    }
}
