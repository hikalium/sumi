//! 作業の免責の検出のベンチマーク。tests/disclaimer/ の例文を 1 文ずつバイナリにかけ、
//! 免責の検出 (否定構文の規則と留保の規則) が当たったかを数える。
//! rejected.txt は全て検出し、allowed.txt は全て通し、limits.txt は書いた期待どおりであることを求める。

use std::io::Write;
use std::process::{Command, Stdio};

/// 免責の検出の修正案の書き出し。この字面を含む検出結果の行を、免責の検出として数える。
const MARKERS: &[&str] = &[
    "事実・現在の状態・対象・次の行動を直接述べる",
    "確認した事実と対象範囲を直接書く",
    "出典や確認した範囲を直接書く",
];

struct Example {
    rust: bool,
    text: String,
}

fn parse(line: &str) -> Option<Example> {
    let (rust, text) = if let Some(t) = line.strip_prefix("md: ") {
        (false, t)
    } else {
        (true, line.strip_prefix("rs: ")?)
    };
    Some(Example {
        rust,
        text: text.replace("\\n", "\n"),
    })
}

fn examples(data: &str) -> Vec<(String, Example)> {
    data.lines()
        .filter_map(|l| parse(l).map(|e| (l.to_string(), e)))
        .collect()
}

/// 例文を検査し、免責の検出の行を返す。
fn disclaimer_hits(e: &Example) -> Vec<String> {
    let (args, input): (&[&str], String) = if e.rust {
        let body: String = e.text.lines().map(|l| format!("// {l}\n")).collect();
        (
            &["--check-only", "--lang", "rust"],
            format!("{body}fn f() {{}}\n"),
        )
    } else {
        (&["--check-only"], format!("{}\n", e.text))
    };
    let mut child = Command::new(env!("CARGO_BIN_EXE_sumi"))
        .args(args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("sumi を起動できること");
    child
        .stdin
        .take()
        .expect("stdin")
        .write_all(input.as_bytes())
        .expect("stdin へ書けること");
    let out = child.wait_with_output().expect("終了を待てること");
    String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter(|l| MARKERS.iter().any(|m| l.contains(m)))
        .map(str::to_string)
        .collect()
}

#[test]
fn disclaimer_benchmark() {
    let rejected = examples(include_str!("disclaimer/rejected.txt"));
    let allowed = examples(include_str!("disclaimer/allowed.txt"));
    let mut wrong = Vec::new();
    let mut caught = 0;
    for (line, e) in &rejected {
        if disclaimer_hits(e).is_empty() {
            wrong.push(format!("見逃し: {line}"));
        } else {
            caught += 1;
        }
    }
    let mut passed = 0;
    for (line, e) in &allowed {
        let hits = disclaimer_hits(e);
        if hits.is_empty() {
            passed += 1;
        } else {
            wrong.push(format!("誤検知: {line} -> {}", hits.join(" / ")));
        }
    }
    for line in include_str!("disclaimer/limits.txt").lines() {
        let (expect_hit, rest) = if let Some(r) = line.strip_prefix("hit ") {
            (true, r)
        } else if let Some(r) = line.strip_prefix("pass ") {
            (false, r)
        } else {
            continue;
        };
        let e = parse(rest).expect("limits.txt の例の書式");
        if disclaimer_hits(&e).is_empty() == expect_hit {
            wrong.push(format!("限界の例の判定が変わった: {line}"));
        }
    }
    println!(
        "検出 {caught}/{} 、通過 {passed}/{}",
        rejected.len(),
        allowed.len()
    );
    assert!(
        wrong.is_empty(),
        "{} 件:\n  {}",
        wrong.len(),
        wrong.join("\n  ")
    );
}
