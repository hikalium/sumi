//! sumi -- 日本語の文章から LLM が生成したような言い回しを検出する。1 文書を標準入力か
//! `--file` のファイルから読み、結果を標準出力へ書く。同梱の規則で検査する。

mod install;
mod mixed;
mod negative;
mod prompt;
mod rules;
mod scan;
mod single;
mod source;
mod words;

use std::io::{Read, Write};

const USAGE: &str = "\
usage: sumi [--file PATH] [--lang markdown|rust|html] [--check-only | --silent | -q]
       sumi --rules | --words | --mask | --help
       sumi install-skill --path PATH
       sumi install-skill --target codex

  sumi                Check one document read from stdin. If there are hits, print the
                      rewrite prompt and the rule list, then the heading \"## 検出結果\",
                      then one hit per line. Exit 1 on hits, 0 (with no output) otherwise,
                      and 2 if the input cannot be checked.
  sumi --check-only   Check, printing only the hit lines (no prompt, no heading).
  sumi --silent, -q   Check, printing nothing to stdout; only the exit code tells the result.
  sumi --file PATH, -f PATH
                      Read the document from PATH instead of stdin.
  sumi --lang LANG    Read the input as LANG: markdown, rust or html. With rust, only
                      comments are checked, with the same rules as documents. With html,
                      the text of the elements and the values of the aria-label, title
                      and alt attributes are checked; tags, other attribute values,
                      comments, character references and the contents of style, script,
                      code and pre are not. Without --lang, the language is inferred from
                      the --file name (.md and .markdown are markdown, .rs is rust, .html
                      and .htm are html); a file whose language cannot be inferred and
                      stdin are read as markdown. Only files whose language cannot be
                      inferred produce a fallback notice, and only when there are hits.
                      The notice goes to stdout before the hits, or to stderr with
                      --check-only or --silent. Stdin produces no fallback notice.
                      --file, --lang, --check-only and --silent may be combined.
  sumi --rules        List the bundled rules, one per line (avoid -> write | keep | note).
                      Includes structural checks for mixed Japanese and English prose,
                      the operator-prohibited Japanese negative constructions and
                      single-kanji words that are not on the allowed list.
  sumi --words        List the word records (registered words, phrasings to avoid,
                      accepted common words, accepted standard abbreviations).
  sumi --mask         For each character of each line, print 1 if it is inside a fence or
                      a code span, and 0 otherwise.
  sumi install-skill --path PATH
                      Install SKILL.md, LICENSE and this executable directly in PATH.
  sumi install-skill --target codex
                      Install in $HOME/.agents/skills/sumi for the current user.
                      Choose exactly one of --path or --target. The destination must
                      not exist. Exit 0 on success, 2 on error. Stdin is not read.

A hit line has the form LINE:COL: 「avoided phrase」-> suggestion. LINE and COL start at 1,
and COL counts characters.
";

fn main() {
    std::process::exit(run());
}

/// 引数を読み、各コマンドへ振り分ける。
///
/// フラグ (引数) の追加・変更・削除は、操作者の明示的な指示がある場合に限って行う。assistant は
/// 必要だと判断しても自分の判断でフラグを足さず、案として操作者へ出す (2026-09-26 操作者指示)。
/// `--lang` と `--file` (`-f`) は 2026-09-26 の操作者指示で足した。`--lang` の値の `html` は、
/// HTML も検査の対象に入れるという 2026-09-28 の操作者指示で足した。
fn run() -> i32 {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let mut stdout = std::io::stdout().lock();
    let argv: Vec<&str> = args.iter().map(String::as_str).collect();
    let result = match argv.as_slice() {
        ["install-skill", options @ ..] => install::run(options).map(|path| {
            emit(
                &mut stdout,
                &format!("Installed sumi skill in {}\n", path.display()),
            )
        }),
        ["--help"] | ["-h"] => Ok(emit(&mut stdout, USAGE)),
        ["--rules"] => Ok(emit(
            &mut stdout,
            &joined(&prompt::rule_lines(rules::RULES)),
        )),
        ["--words"] => Ok(emit(&mut stdout, &render_words())),
        ["--mask"] => read_stdin().map(|body| {
            let lines = scan::Document::parse(&body).mask_lines();
            emit(&mut stdout, &joined(&lines))
        }),
        _ => match check_options(&argv) {
            Some(opts) => check(&mut stdout, opts),
            None => Err(format!("invalid arguments: {}\n{USAGE}", args.join(" "))),
        },
    };
    match result {
        Ok(code) => code,
        Err(reason) => {
            eprintln!("sumi: {reason}");
            2
        }
    }
}

/// 入力の言語。
#[derive(Clone, Copy, PartialEq, Eq)]
enum Lang {
    /// 文書 (Markdown) として読む
    Markdown,
    /// Rust のソースとして読み、コメントだけを検査する
    Rust,
    /// HTML として読み、要素の本文と人が読む属性値だけを検査する
    Html,
}

impl Lang {
    /// ファイル名の拡張子から言語を推定する。推定できなければ None を返す。
    fn from_path(path: &str) -> Option<Lang> {
        match std::path::Path::new(path).extension()?.to_str()? {
            "md" | "markdown" => Some(Lang::Markdown),
            "rs" => Some(Lang::Rust),
            "html" | "htm" => Some(Lang::Html),
            _ => None,
        }
    }
}

/// 検査の引数。
struct CheckOptions {
    output: Output,
    /// `--lang` で指定した言語
    lang: Option<Lang>,
    /// `--file` (`-f`) で指定したファイルのパス
    file: Option<String>,
}

/// 検査の引数を読む。出力の形の指定、`--lang LANG`、`--file PATH` は、それぞれ高々 1 つである。
fn check_options(argv: &[&str]) -> Option<CheckOptions> {
    let mut output = None;
    let mut lang = None;
    let mut file = None;
    let mut rest = argv.iter();
    while let Some(a) = rest.next() {
        match *a {
            "--lang" => {
                let l = match *rest.next()? {
                    "markdown" => Lang::Markdown,
                    "rust" => Lang::Rust,
                    "html" => Lang::Html,
                    _ => return None,
                };
                if lang.replace(l).is_some() {
                    return None;
                }
            }
            "--file" | "-f" => {
                if file.replace(rest.next()?.to_string()).is_some() {
                    return None;
                }
            }
            _ => {
                let o = match *a {
                    "--check-only" => Output::HitsOnly,
                    "--silent" | "-q" => Output::Silent,
                    _ => return None,
                };
                if output.replace(o).is_some() {
                    return None;
                }
            }
        }
    }
    Some(CheckOptions {
        output: output.unwrap_or(Output::PromptAndHits),
        lang,
        file,
    })
}

/// 検査の出力の形。
#[derive(Clone, Copy, PartialEq, Eq)]
enum Output {
    /// 改善プロンプト、「## 検出結果」の見出し、ヒットの行
    PromptAndHits,
    /// ヒットの行だけ
    HitsOnly,
    /// 標準出力には何も出さない (終了コードだけ)
    Silent,
}

/// 検査。ヒットがあれば 1、無ければ 0 で何も出さない。ヒットがあるときの出力は `output` で決まる。
/// 入力は `--file` のファイルか標準入力から読む。言語は `--lang` の指定を使い、指定が無ければ
/// `--file` のファイル名から推定し、推定できないファイルと標準入力は文書 (Markdown) として読む。
fn check(out: &mut impl Write, opts: CheckOptions) -> Result<i32, String> {
    // 推定できない拡張子のファイルは文書 (Markdown) として読み、その旨を検出結果の前に伝える
    let mut notice = None;
    let lang = match (opts.lang, &opts.file) {
        (Some(l), _) => l,
        (None, Some(path)) => Lang::from_path(path).unwrap_or_else(|| {
            notice = Some(format!(
                "note: could not infer the language of {path} from its name, so it was checked as \
                 Markdown. Use --lang to specify another language."
            ));
            Lang::Markdown
        }),
        (None, None) => Lang::Markdown,
    };
    let src = match &opts.file {
        Some(path) => read_file(path)?,
        None => read_stdin()?,
    };
    // 検査する本文と単位の割り方。Rust と HTML は、行末のコメントや要素のタグを見分けるために
    // 元のソースも読む
    let (body, layout) = match lang {
        Lang::Markdown => (src, mixed::Layout::plain()),
        Lang::Rust => {
            let comments = source::rust_comments(&src);
            let layout = mixed::Layout::rust(&src, &comments);
            (comments, layout)
        }
        Lang::Html => {
            let text = source::html_text(&src);
            let layout = mixed::Layout::html(&src, &text);
            (text, layout)
        }
    };
    // 本文を 1 度だけ解析し、語の規則、混在、否定構文、漢字 1 字の語の 4 つの検査で使う
    let doc = scan::Document::parse(&body);
    let units = mixed::prose_units(&doc, &layout);
    let output = opts.output;
    let word_hits = scan::scan_doc(&doc, rules::RULES);
    // 語の規則が同じ位置で当たった漢字 1 字の語を落とす。操作者が裁定した語 (「節」) に、許可リストへ
    // 足す案内を並べないためである
    let ruled: std::collections::HashSet<(usize, usize)> =
        word_hits.iter().map(|h| (h.line, h.col)).collect();
    // 語の規則、混在、否定構文、漢字 1 字の語のヒットを、行と列の順に並べる (同じ位置なら語の規則が先)
    let mut hits: Vec<(usize, usize, String)> = word_hits
        .iter()
        .map(|h| (h.line, h.col, h.render()))
        .chain(
            mixed::scan_units(&units)
                .iter()
                .map(|h| (h.line, h.col, h.render())),
        )
        .chain(
            negative::scan_units(&units)
                .iter()
                .map(|h| (h.line, h.col, h.render())),
        )
        .chain(
            single::scan_units(&units)
                .iter()
                .filter(|h| !ruled.contains(&(h.line, h.col)))
                .map(|h| (h.line, h.col, h.render())),
        )
        .collect();
    hits.sort_by_key(|&(line, col, _)| (line, col));
    if hits.is_empty() {
        return Ok(0);
    }
    // 既定の出力では見出しと検出結果の間に、それ以外では標準エラー出力に注意を出す。
    // --check-only の標準出力は検出結果の行だけ、--silent の標準出力は空に保つ。
    if let Some(n) = &notice
        && output != Output::PromptAndHits
    {
        eprintln!("sumi: {n}");
    }
    if output == Output::Silent {
        return Ok(1);
    }
    let lines: Vec<String> = hits.into_iter().map(|(_, _, l)| l).collect();
    if output == Output::PromptAndHits {
        let notice = notice.map(|n| format!("{n}\n\n")).unwrap_or_default();
        emit(
            out,
            &format!(
                "{}\n{}\n\n{notice}",
                prompt::render(rules::RULES),
                prompt::HITS_HEADING
            ),
        );
    }
    emit(out, &joined(&lines));
    Ok(1)
}

fn read_file(path: &str) -> Result<String, String> {
    let buf = std::fs::read(path).map_err(|e| format!("cannot read {path}: {e}"))?;
    String::from_utf8(buf).map_err(|e| format!("{path} is not valid UTF-8: {e}"))
}

fn read_stdin() -> Result<String, String> {
    let mut buf = Vec::new();
    std::io::stdin()
        .read_to_end(&mut buf)
        .map_err(|e| format!("cannot read stdin: {e}"))?;
    String::from_utf8(buf).map_err(|e| format!("stdin is not valid UTF-8: {e}"))
}

/// 各行に改行を付けて繋ぐ。空なら空。
fn joined(lines: &[String]) -> String {
    let mut s = String::new();
    for l in lines {
        s.push_str(l);
        s.push('\n');
    }
    s
}

/// 書き出す。書き出し先が閉じていても (パイプの先が終わっていても) 終了コードを保つ。
fn emit(out: &mut impl Write, text: &str) -> i32 {
    let _ = out.write_all(text.as_bytes());
    let _ = out.flush();
    0
}

/// `--words` の出力。src/words.rs の 4 表を人が読める形にする。表が空でも各表の見出しを出す。
fn render_words() -> String {
    let mut s = String::new();
    let indent = |text: &str| -> String {
        text.trim()
            .lines()
            .map(|l| format!("    {}", l.trim_end()))
            .collect::<Vec<_>>()
            .join("\n")
    };
    let list = |items: &[&str]| -> String { items.join(", ") };

    s.push_str(&format!("## 登録した語 ({})\n\n", words::REGISTER.len()));
    if words::REGISTER.is_empty() {
        s.push_str("(なし)\n");
    }
    for r in words::REGISTER {
        s.push_str(&format!("- {}", r.word));
        if !r.ident.is_empty() {
            s.push_str(&format!(" [{}]", list(r.ident)));
        }
        s.push('\n');
        s.push_str(&format!("  meaning:\n{}\n", indent(r.meaning)));
        if !r.not.is_empty() {
            s.push_str(&format!("  not: {}\n", list(r.not)));
        }
        if !r.verb.is_empty() {
            s.push_str(&format!("  verb: {}\n", list(r.verb)));
        }
        if let Some(note) = r.note {
            s.push_str(&format!("  note:\n{}\n", indent(note)));
        }
    }

    s.push_str(&format!("\n## 避ける書き方 ({})\n\n", words::REWRITE.len()));
    if words::REWRITE.is_empty() {
        s.push_str("(なし)\n");
    }
    for r in words::REWRITE {
        s.push_str(&format!(
            "- {} -> {}\n",
            r.avoid.join(" / "),
            r.write.join(" / ")
        ));
        if let Some(when) = r.when {
            s.push_str(&format!("  when: {}\n", when.trim()));
        }
        if !r.keep.is_empty() {
            s.push_str(&format!("  keep: {}\n", list(r.keep)));
        }
        if let Some(note) = r.note {
            s.push_str(&format!("  note:\n{}\n", indent(note)));
        }
    }

    s.push_str(&format!(
        "\n## 一般語と認めた例 ({})\n\n",
        words::ORDINARY.len()
    ));
    if words::ORDINARY.is_empty() {
        s.push_str("(なし)\n");
    }
    for o in words::ORDINARY {
        s.push_str(&format!("- {}\n  note:\n{}\n", o.word, indent(o.note)));
    }

    s.push_str(&format!(
        "\n## 標準語と認めた略語 ({})\n\n",
        words::ABBREVIATION.len()
    ));
    if words::ABBREVIATION.is_empty() {
        s.push_str("(なし)\n");
    }
    for a in words::ABBREVIATION {
        s.push_str(&format!("- {}\n  note:\n{}\n", a.word, indent(a.note)));
    }
    s
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn words_listing_has_all_four_headings_even_when_empty() {
        let out = render_words();
        for h in [
            "## 登録した語 (",
            "## 避ける書き方 (",
            "## 一般語と認めた例 (",
            "## 標準語と認めた略語 (",
        ] {
            assert!(out.contains(h), "{h:?} が無い:\n{out}");
        }
    }

    #[test]
    fn joined_adds_one_newline_per_line_and_nothing_for_empty() {
        assert_eq!(joined(&[]), "");
        assert_eq!(joined(&["a".into(), "b".into()]), "a\nb\n");
    }
}
