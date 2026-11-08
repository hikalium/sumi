//! バイナリを起動する統合テスト。

use std::io::Write;
use std::process::{Command, Stdio};

struct Run {
    code: i32,
    stdout: String,
    stderr: String,
}

fn sumi(args: &[&str], stdin: &[u8]) -> Run {
    run(Command::new(env!("CARGO_BIN_EXE_sumi")).args(args), stdin)
}

fn run(command: &mut Command, stdin: &[u8]) -> Run {
    let mut child = command
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("sumi を起動できること");
    child
        .stdin
        .take()
        .expect("stdin")
        .write_all(stdin)
        .expect("stdin へ書けること");
    let out = child.wait_with_output().expect("終了を待てること");
    Run {
        code: out.status.code().expect("終了コード"),
        stdout: String::from_utf8_lossy(&out.stdout).into_owned(),
        stderr: String::from_utf8_lossy(&out.stderr).into_owned(),
    }
}

#[test]
fn clean_input_exits_zero_with_no_output() {
    let r = sumi(&["--check-only"], "自然な日本語の文書。\n".as_bytes());
    assert_eq!(r.code, 0);
    assert_eq!(r.stdout, "");
    assert_eq!(r.stderr, "");
    let r = sumi(&["--check-only"], b"");
    assert_eq!(r.code, 0);
    assert_eq!(r.stdout, "");
}

#[test]
fn a_hit_exits_one_and_prints_line_col_avoid_and_write() {
    let r = sumi(
        &["--check-only"],
        "見出し\nこの層は独自算術を持つ。\n".as_bytes(),
    );
    assert_eq!(r.code, 1);
    assert_eq!(r.stdout, "2:5: 「独自算術」-> 独自の計算ロジック\n");
    // 複数の案は / で並ぶ
    let r = sumi(&["--check-only"], "実物の欠落が原因だった。\n".as_bytes());
    assert_eq!(r.code, 1);
    assert_eq!(r.stdout, "1:1: 「実物の欠落」-> 実装上の不備 / 考慮漏れ\n");
}

#[test]
fn invalid_utf8_exits_two_with_a_reason_on_stderr() {
    let r = sumi(&["--check-only"], b"\xff\xfe abc\n");
    assert_eq!(r.code, 2);
    assert_eq!(r.stdout, "");
    assert!(r.stderr.contains("UTF-8"), "{}", r.stderr);
}

#[test]
fn bad_arguments_exit_two() {
    let r = sumi(&["--nope"], b"");
    assert_eq!(r.code, 2);
    assert!(r.stderr.contains("usage"), "{}", r.stderr);
    let r = sumi(&["--help"], b"");
    assert_eq!(r.code, 0);
    assert!(
        r.stdout
            .starts_with("usage: sumi [--file PATH] [--lang markdown|rust|html] [--check-only"),
        "{}",
        r.stdout
    );
}

#[test]
fn rules_lists_one_line_per_rule() {
    let r = sumi(&["--rules"], b"");
    assert_eq!(r.code, 0);
    let lines: Vec<&str> = r.stdout.lines().collect();
    // 語の規則の後に、混在、否定構文、漢字 1 字の語の検査が 1 行ずつ続く。規則の件数を比べる
    // 検査を置かず、規則を足すブランチどうしのコンフリクトを避ける。
    let n = lines.len();
    assert!(n > 3, "{n}");
    for l in &lines {
        assert!(l.contains(" -> "), "{l}");
        assert!(l.contains("| keep: "), "{l}");
        assert!(l.contains("| note: "), "{l}");
    }
    assert!(lines[0].starts_with("道具が言 / 道具は言 -> "));
    assert!(
        lines[n - 3].starts_with("(日本語の文と、"),
        "{}",
        lines[n - 3]
    );
    assert!(lines[n - 2].starts_with("〜や〜はしていない / "));
    assert!(lines[n - 1].starts_with("(前後に漢字もカタカナも無く、"));
}

#[test]
fn negative_constructions_reach_each_output_mode_and_language() {
    let body = "実装や実験はしていない。";
    let r = sumi(&["--check-only"], body.as_bytes());
    assert_eq!(r.code, 1);
    assert!(r.stdout.starts_with("1:6: 「はしていない」-> "));
    assert!(r.stdout.contains("事実・現在の状態"));
    let r = sumi(&[], body.as_bytes());
    assert_eq!(r.code, 1);
    assert!(r.stdout.contains("〜であり〜ではない"));
    let r = sumi(&["--silent"], body.as_bytes());
    assert_eq!(r.code, 1);
    assert!(r.stdout.is_empty());
    let r = sumi(
        &["--lang", "rust", "--check-only"],
        "// 実装はしていない。\nlet x = \"実装はしていない。\";".as_bytes(),
    );
    assert_eq!(r.code, 1);
    assert_eq!(r.stdout.lines().count(), 1);
    let r = sumi(
        &["--lang", "html", "--check-only"],
        "<p>実装はしていない。</p><code>実装はしていない。</code>".as_bytes(),
    );
    assert_eq!(r.code, 1);
    assert_eq!(r.stdout.lines().count(), 1);
    for (lang, text) in [
        ("html", "<p>結果は確認した</p><p>追加しない。</p>"),
        ("rust", "let a = 0; // 結果は確認した\n// 追加しない。"),
    ] {
        let r = sumi(&["--lang", lang, "--check-only"], text.as_bytes());
        assert_eq!(r.code, 0, "{text}: {}", r.stdout);
    }
}

#[test]
fn default_output_puts_the_prompt_before_the_hits() {
    let doc = "この層は独自算術を持つ。\n";
    let p = sumi(&[], doc.as_bytes());
    assert_eq!(p.code, 1);
    let rules = sumi(&["--rules"], b"");
    for l in rules.stdout.lines() {
        assert!(p.stdout.contains(&format!("- {l}\n")), "{l}");
    }
    for needle in ["役割", "守ること", "確認"] {
        assert!(p.stdout.contains(needle));
    }
    // 見出しの後はヒットの行だけで、-q の出力と一致する
    let (_, hits) = p
        .stdout
        .split_once("\n## 検出結果\n\n")
        .expect("見出しがある");
    assert_eq!(hits, sumi(&["--check-only"], doc.as_bytes()).stdout);
    // ヒットが無ければプロンプトも出さない
    let r = sumi(&[], "自然な日本語の文書。\n".as_bytes());
    assert_eq!((r.code, r.stdout.as_str()), (0, ""));
    // --silent と -q は標準出力に何も出さず、終了コードだけを返す
    for flag in ["--silent", "-q"] {
        let r = sumi(&[flag], doc.as_bytes());
        assert_eq!((r.code, r.stdout.as_str()), (1, ""));
    }
    // 未定義の --prompt を拒否する
    assert_eq!(sumi(&["--prompt"], b"").code, 2);
}

#[test]
fn words_lists_four_tables() {
    let r = sumi(&["--words"], b"");
    assert_eq!(r.code, 0);
    assert_eq!(r.stdout.matches("\n## ").count() + 1, 4);
}

#[test]
fn mask_marks_fences_and_code_spans_per_char() {
    let doc = "ab `cd` e\n```\nxy\n```\nz「q」\n\n";
    let r = sumi(&["--mask"], doc.as_bytes());
    assert_eq!(r.code, 0);
    assert_eq!(r.stdout, "000111100\n111\n11\n111\n0000\n\n");
}

#[test]
fn fences_code_spans_quotes_and_keep_hide_hits() {
    let doc =
        "```\n交点\n```\n`交点` は識別子\n「交点」は避ける\n道具が言語ごとに違う\n手元の赤字\n";
    let r = sumi(&["--check-only"], doc.as_bytes());
    assert_eq!(r.code, 0, "{}", r.stdout);
    assert_eq!(r.stdout, "");
    // 除外の外にある同じ語は当たる
    let doc = "「交点」の外の交点\n";
    let r = sumi(&["--check-only"], doc.as_bytes());
    assert_eq!(r.code, 1);
    assert_eq!(r.stdout, "1:8: 「交点」-> 状態の組み合わせ\n");
}

#[test]
fn a_quote_may_span_lines_within_a_paragraph() {
    // 行をまたぐ「」の内側を検査対象から除く
    let doc = "操作者は「この\n交点は避ける」と言った。\n";
    let r = sumi(&["--check-only"], doc.as_bytes());
    assert_eq!(r.code, 0, "{}", r.stdout);
    assert_eq!(r.stdout, "");
    // 空行をまたぐ「」の内側も検査対象に残す
    let doc = "操作者は「この\n\n交点は避ける」と言った。\n";
    let r = sumi(&["--check-only"], doc.as_bytes());
    assert_eq!(r.code, 1);
    assert_eq!(r.stdout, "3:1: 「交点」-> 状態の組み合わせ\n");
    // 閉じ忘れた引用の本文を検査対象に残す
    let doc = "操作者は「この\n交点は避ける\n";
    let r = sumi(&["--check-only"], doc.as_bytes());
    assert_eq!(r.code, 1);
    assert_eq!(r.stdout, "2:1: 「交点」-> 状態の組み合わせ\n");
    // --mask は引用だけの本文に 0 を返す
    let r = sumi(&["--mask"], "a「b\nc」d\n".as_bytes());
    assert_eq!(r.stdout, "000\n000\n");
}

#[test]
fn sumi_own_documents_and_comments_pass() {
    // sumi 自身が sumi の検査を通ることを、テストの条件にする。repo の全てのファイル (target/ と
    // .git/ を除く) を --file で検査するので、足したファイルも自動で対象に入る。言語はファイル名から
    // 推定する (.md は文書、.rs はコメントだけ、.html は本文と一部の属性値、それ以外は Markdown)。
    // anti-cases/ の *.before.md は検出を期待する原文で、専用テストでもヒット内容を検査する。
    fn walk(dir: &std::path::Path, out: &mut Vec<std::path::PathBuf>) {
        for entry in std::fs::read_dir(dir).unwrap() {
            let path = entry.unwrap().path();
            let name = path.file_name().unwrap().to_str().unwrap();
            if name == "target" || name == ".git" {
                continue;
            }
            if path.is_dir() {
                walk(&path, out);
            } else {
                out.push(path);
            }
        }
    }
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let mut files = Vec::new();
    walk(root, &mut files);
    files.sort();
    for must in [
        "README.md",
        "DESIGN.md",
        "src/main.rs",
        "src/rules.rs",
        "tests/cli.rs",
    ] {
        assert!(
            files.contains(&root.join(must)),
            "{must} が走査に入っていない"
        );
    }
    let mut failures = String::new();
    let anti_cases = root.join("anti-cases");
    for path in &files {
        let r = sumi(&["--check-only", "--file", path.to_str().unwrap()], b"");
        let expected = i32::from(
            path.parent() == Some(anti_cases.as_path())
                && path
                    .file_name()
                    .unwrap()
                    .to_str()
                    .unwrap()
                    .ends_with(".before.md"),
        );
        if r.code != expected {
            failures.push_str(&format!(
                "{}: expected {expected}, got {}\n{}",
                path.display(),
                r.code,
                r.stdout
            ));
        }
    }
    assert!(
        failures.is_empty(),
        "sumi 自身が検査を通らない:\n{failures}"
    );
}

#[test]
fn file_reads_the_path_and_infers_the_language() {
    let dir = std::env::temp_dir().join(format!("sumi-cli-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let text = "let s = \"この層は独自算術を持つ。\";\n// この層は独自算術を持つ。\n";
    for name in ["a.rs", "a.md", "a.txt"] {
        std::fs::write(dir.join(name), text).unwrap();
    }
    let path = |name: &str| dir.join(name).to_str().unwrap().to_string();
    // .rs はコメントだけを検査する
    let r = sumi(&["--check-only", "-f", &path("a.rs")], b"");
    assert_eq!((r.code, r.stdout.lines().count()), (1, 1), "{}", r.stdout);
    // .md は全文を検査する
    let r = sumi(&["--check-only", "--file", &path("a.md")], b"");
    assert_eq!((r.code, r.stdout.lines().count()), (1, 2), "{}", r.stdout);
    // --lang の指定は推定より優先する
    let r = sumi(
        &["--check-only", "--lang", "markdown", "-f", &path("a.rs")],
        b"",
    );
    assert_eq!(r.stdout.lines().count(), 2, "{}", r.stdout);
    // 推定できない拡張子は Markdown として読み、その旨を検出結果の前に伝える
    let r = sumi(&["-f", &path("a.txt")], b"");
    assert_eq!(r.code, 1);
    let (_, rest) = r
        .stdout
        .split_once("\n## 検出結果\n\n")
        .expect("見出しがある");
    assert!(rest.starts_with("note: "), "{rest}");
    assert!(rest.contains("checked as Markdown"), "{rest}");
    let hits = sumi(&["--check-only", "-f", &path("a.txt")], b"").stdout;
    assert!(rest.ends_with(&format!("\n\n{hits}")), "{rest}");
    // --check-only の標準出力は検出結果の行だけで、注意は標準エラー出力へ出る
    let r = sumi(&["--check-only", "-f", &path("a.txt")], b"");
    assert_eq!((r.code, r.stdout.lines().count()), (1, 2), "{}", r.stdout);
    assert!(r.stderr.contains("checked as Markdown"), "{}", r.stderr);
    // --lang を指定すれば指定された言語として検査し、標準エラー出力を空に保つ
    let r = sumi(
        &["--check-only", "--lang", "rust", "-f", &path("a.txt")],
        b"",
    );
    assert_eq!((r.code, r.stderr.as_str()), (1, ""));
    // 読めないファイル、値の無い --file、2 回の --file は exit 2
    assert_eq!(sumi(&["-f", &path("none.md")], b"").code, 2);
    assert_eq!(sumi(&["--file"], b"").code, 2);
    assert_eq!(
        sumi(&["-f", &path("a.md"), "-f", &path("a.md")], b"").code,
        2
    );
    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn rust_mode_checks_only_comments() {
    let src = "let s = \"この層は独自算術を持つ。\";\n// この層は独自算術を持つ。\n";
    let r = sumi(&["--lang", "rust", "--check-only"], src.as_bytes());
    assert_eq!(r.code, 1);
    assert_eq!(r.stdout.lines().count(), 1);
    assert!(r.stdout.starts_with("2:"), "{}", r.stdout);
    // 出力の形の指定は 1 つだけ
    assert_eq!(sumi(&["--check-only", "-q"], b"").code, 2);
    // 言語の指定を 1 つに限り、未対応の言語を拒否する
    assert_eq!(sumi(&["--lang", "rust", "--lang", "rust"], b"").code, 2);
    assert_eq!(sumi(&["--lang", "c"], b"").code, 2);
    assert_eq!(sumi(&["--lang"], b"").code, 2);
    assert_eq!(sumi(&["--rust"], b"").code, 2);
}

#[test]
fn html_mode_checks_text_and_prose_attributes_only() {
    let src = concat!(
        "<style>p { content: \"この層は独自算術を持つ。\"; }</style>\n",
        "<p class=\"独自算術\">本文の<code>独自算術</code></p>\n",
        "<svg><text>この層は独自算術を持つ。</text></svg>\n",
        "<button aria-label=\"独自算術\">x</button>\n",
    );
    let r = sumi(&["--lang", "html", "--check-only"], src.as_bytes());
    assert_eq!(r.code, 1, "{}", r.stdout);
    assert_eq!(
        r.stdout,
        "3:16: 「独自算術」-> 独自の計算ロジック\n4:21: 「独自算術」-> 独自の計算ロジック\n"
    );
    // .html と .htm は html と推定し、標準エラー出力を空に保つ
    let dir = std::env::temp_dir().join(format!("sumi-cli-html-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    for name in ["a.html", "a.htm"] {
        let path = dir.join(name);
        std::fs::write(&path, src).unwrap();
        let f = sumi(&["--check-only", "-f", path.to_str().unwrap()], b"");
        assert_eq!(
            (f.stdout.as_str(), f.stderr.as_str()),
            (r.stdout.as_str(), "")
        );
    }
    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn mixed_japanese_and_english_in_one_unit_is_a_hit() {
    // 2026-09-28 に操作者が指摘した doc comment の形。語の規則のヒットと同じ行の形で出る
    let src = "/// path to a JSON array of real-input steps played before the test.\n\
               /// 実入力が必要な振る舞いを測るために使う。\n\
               fn f() {}\n";
    let r = sumi(&["--check-only", "--lang", "rust"], src.as_bytes());
    assert_eq!(r.code, 1, "{}", r.stderr);
    assert_eq!(r.stdout.lines().count(), 1, "{}", r.stdout);
    assert!(
        r.stdout.starts_with(
            "1:5: 「path to a JSON array of real-input steps played before the t ...」-> 単位 ("
        ),
        "{}",
        r.stdout
    );
    // 同じ文を 1 つの言語で書けば通る
    let ja = "/// `--test-js` の前に流す、実入力の手順の JSON 配列のパスである。\n\
              /// 実入力が必要な振る舞いを測るために使う。\n";
    assert_eq!(
        sumi(&["--check-only", "--lang", "rust"], ja.as_bytes()).code,
        0
    );
    // 語の規則のヒットと混在のヒットは、行と列の順に並ぶ
    let doc = "この層は独自算術を持つ。\n\nこれは the rest of this line is English である。\n";
    let r = sumi(&["--check-only"], doc.as_bytes());
    let lines: Vec<&str> = r.stdout.lines().collect();
    assert_eq!(lines.len(), 2, "{}", r.stdout);
    assert!(lines[0].starts_with("1:5: 「独自算術」"));
    assert!(lines[1].starts_with("3:5: 「the rest of this line is English」"));
}

#[test]
fn a_single_kanji_word_off_the_allowed_list_is_a_hit() {
    // 「筐」は許可リストの外にある。「件」はリストにあり、「「筐」」は引用である
    let r = sumi(
        &["--check-only"],
        "この筐を開き、次の件と「筐」を読む。\n".as_bytes(),
    );
    assert_eq!(r.code, 1, "{}", r.stderr);
    assert_eq!(r.stdout.lines().count(), 1, "{}", r.stdout);
    assert!(
        r.stdout.starts_with("1:3: 「筐」-> 国語辞典に載る意味で"),
        "{}",
        r.stdout
    );
}

#[test]
fn a_ruled_single_kanji_word_gets_only_the_rule_hit() {
    // 語の規則が当たる「節」には、規則のヒットの 1 行だけを出す
    let r = sumi(&["--check-only"], "この節を読む。\n".as_bytes());
    assert_eq!(r.code, 1, "{}", r.stderr);
    assert_eq!(r.stdout.lines().count(), 1, "{}", r.stdout);
    assert!(!r.stdout.contains("ALLOWED"), "{}", r.stdout);
}

struct TempDir(std::path::PathBuf);

impl TempDir {
    fn new() -> Self {
        use std::sync::atomic::{AtomicUsize, Ordering};
        static NEXT: AtomicUsize = AtomicUsize::new(0);
        let path = std::env::temp_dir().join(format!(
            "sumi-install-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        std::fs::create_dir(&path).unwrap();
        Self(path)
    }

    fn command(&self) -> Command {
        let mut command = Command::new(env!("CARGO_BIN_EXE_sumi"));
        command.current_dir(&self.0).env_remove("HOME");
        command
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

fn check_installed_skill(path: &std::path::Path, cwd: &std::path::Path) {
    let binary = path.join(format!("sumi{}", std::env::consts::EXE_SUFFIX));
    assert_eq!(std::fs::read_dir(path).unwrap().count(), 3);
    assert_eq!(
        std::fs::read_to_string(path.join("LICENSE")).unwrap(),
        include_str!("../LICENSE")
    );
    assert_eq!(
        std::fs::read_to_string(path.join("SKILL.md")).unwrap(),
        include_str!("../skills/sumi/SKILL.md")
    );
    assert_eq!(
        std::fs::read(&binary).unwrap(),
        std::fs::read(env!("CARGO_BIN_EXE_sumi")).unwrap()
    );
    for input in [
        "自然な日本語の文書。".as_bytes(),
        "独自算術".as_bytes(),
        b"\xff",
    ] {
        let expected = sumi(&["--check-only"], input);
        let installed = run(
            Command::new(&binary)
                .current_dir(cwd)
                .env("PATH", "")
                .arg("--check-only"),
            input,
        );
        assert_eq!(installed.code, expected.code, "{}", installed.stderr);
        assert_eq!(installed.stdout, expected.stdout);
        assert_eq!(installed.stderr, expected.stderr);
    }
}

#[test]
fn install_for_codex_uses_the_user_skill_directory() {
    let temp = TempDir::new();
    let home = temp.0.join("home");
    let result = run(
        temp.command()
            .env("HOME", &home)
            .env("CODEX_HOME", temp.0.join("other"))
            .args(["install-skill", "--target", "codex"]),
        b"",
    );
    assert_eq!(result.code, 0, "{}", result.stderr);
    let destination = home.join(".agents/skills/sumi");
    assert!(result.stdout.contains(&destination.display().to_string()));
    assert!(result.stderr.is_empty());
    assert!(!temp.0.join("other").exists());
    check_installed_skill(&destination, &temp.0);
}

#[test]
fn install_to_a_relative_path_works_without_home() {
    let temp = TempDir::new();
    let result = run(
        temp.command()
            .args(["install-skill", "--path", "日本語 space/skill"]),
        b"",
    );
    assert_eq!(result.code, 0, "{}", result.stderr);
    check_installed_skill(&temp.0.join("日本語 space/skill"), &temp.0);
}

#[test]
fn install_rejects_bad_arguments_without_creating_files() {
    let temp = TempDir::new();
    for args in [
        vec!["install-skill"],
        vec!["install-skill", "--path"],
        vec!["install-skill", "--path", "--target"],
        vec!["install-skill", "--path", "--path"],
        vec!["install-skill", "--path", "--check-only"],
        vec!["install-skill", "--target"],
        vec!["install-skill", "--path", ""],
        vec!["install-skill", "--target", "unknown"],
        vec!["install-skill", "--target", ""],
        vec!["install-skill", "--path", "a", "--target", "codex"],
        vec!["install-skill", "--target", "codex", "--path", "a"],
        vec!["install-skill", "--path", "a", "--path", "b"],
        vec!["install-skill", "--target", "codex", "--target", "codex"],
        vec!["install-skill", "--path", "a", "extra"],
        vec!["install-skill", "--target", "codex", "--check-only"],
        vec!["--file", "doc.md", "install-skill", "--path", "a"],
    ] {
        let result = run(temp.command().env("HOME", &temp.0).args(&args), b"");
        assert_eq!(result.code, 2, "{args:?}: {}", result.stdout);
        assert!(result.stdout.is_empty());
        assert!(!result.stderr.is_empty());
    }
    assert_eq!(std::fs::read_dir(&temp.0).unwrap().count(), 0);
}

#[test]
fn codex_install_requires_a_nonempty_home() {
    let temp = TempDir::new();
    for home in [None, Some("")] {
        let mut command = temp.command();
        if let Some(value) = home {
            command.env("HOME", value);
        }
        let result = run(command.args(["install-skill", "--target", "codex"]), b"");
        assert_eq!(result.code, 2);
        assert!(result.stdout.is_empty());
        assert!(result.stderr.contains("HOME"));
        assert!(result.stderr.contains("--path"));
    }
    assert_eq!(std::fs::read_dir(&temp.0).unwrap().count(), 0);
}

#[test]
fn install_preserves_existing_files_and_directories() {
    let temp = TempDir::new();
    std::fs::write(temp.0.join("file"), "keep file").unwrap();
    std::fs::create_dir(temp.0.join("empty")).unwrap();
    let installed = temp.0.join("installed");
    let result = run(
        temp.command()
            .args(["install-skill", "--path"])
            .arg(&installed),
        b"",
    );
    assert_eq!(result.code, 0, "{}", result.stderr);
    std::fs::write(installed.join("SKILL.md"), "keep modified skill").unwrap();
    for path in ["file", "empty", "installed", "file/child"] {
        let result = run(temp.command().args(["install-skill", "--path", path]), b"");
        assert_eq!(result.code, 2, "{path}: {}", result.stdout);
        assert!(result.stdout.is_empty());
        assert!(result.stderr.contains(&temp.0.display().to_string()));
    }
    assert_eq!(
        std::fs::read_to_string(temp.0.join("file")).unwrap(),
        "keep file"
    );
    assert_eq!(std::fs::read_dir(temp.0.join("empty")).unwrap().count(), 0);
    assert_eq!(
        std::fs::read_to_string(installed.join("SKILL.md")).unwrap(),
        "keep modified skill"
    );
    assert_eq!(std::fs::read_dir(installed).unwrap().count(), 3);
}

#[cfg(unix)]
#[test]
fn install_preserves_symlinks_including_dangling_links() {
    let temp = TempDir::new();
    let target = temp.0.join("target");
    std::fs::create_dir(&target).unwrap();
    std::fs::write(target.join("keep"), "original").unwrap();
    let missing = temp.0.join("missing");
    for (link, to) in [("linked", &target), ("dangling", &missing)] {
        let path = temp.0.join(link);
        std::os::unix::fs::symlink(to, &path).unwrap();
        let result = run(temp.command().args(["install-skill", "--path", link]), b"");
        assert_eq!(result.code, 2);
        assert_eq!(std::fs::read_link(path).unwrap(), *to);
    }
    assert_eq!(
        std::fs::read_to_string(target.join("keep")).unwrap(),
        "original"
    );
    assert_eq!(std::fs::read_dir(target).unwrap().count(), 1);
    assert!(!missing.exists());
}

#[test]
fn fallback_notices_require_a_hit_in_a_file_with_unknown_language() {
    let temp = TempDir::new();
    for (body, code) in [("自然な日本語の文書。", 0), ("独自算術", 1)] {
        for output in [vec![], vec!["--check-only"], vec!["--silent"]] {
            let stdin = sumi(&output, body.as_bytes());
            assert_eq!(stdin.code, code);
            assert!(!stdin.stdout.contains("note: could not infer the language"));
            assert!(stdin.stderr.is_empty());
            for name in ["doc.md", "doc.txt"] {
                let path = temp.0.join(name);
                std::fs::write(&path, body).unwrap();
                let mut args = output.clone();
                args.extend(["--file", path.to_str().unwrap()]);
                let result = sumi(&args, b"");
                assert_eq!(result.code, code);
                let notice = name == "doc.txt" && code == 1;
                assert_eq!(
                    result.stdout.contains("note: could not infer the language"),
                    notice && output.is_empty()
                );
                assert_eq!(
                    result.stderr.contains("note: could not infer the language"),
                    notice && !output.is_empty()
                );
                if code == 0 {
                    assert!(result.stdout.is_empty());
                    assert!(result.stderr.is_empty());
                }
                if output == ["--silent"] {
                    assert!(result.stdout.is_empty());
                }
                args.extend(["--lang", "markdown"]);
                let explicit = sumi(&args, b"");
                assert_eq!(explicit.code, code);
                assert!(
                    !explicit
                        .stdout
                        .contains("note: could not infer the language")
                );
                assert!(explicit.stderr.is_empty());
            }
        }
    }
}
