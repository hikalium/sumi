//! Rust と HTML の入力から、検査の対象にする文だけを取り出す。Rust のソースからはコメントを、
//! HTML からは人が読む本文と一部の属性値を取り出し、どちらも文書と同じ扱いで検査する。
//!
//! 取り出した本文は、元のソースと同じ行数と各行の文字数を保つ。検査しない文字は空白に
//! 置き換えるので、ヒットの LINE と COL はそのまま元のソースの位置を指す。コードやタグだけの行は
//! 空白だけの行になり、段落の区切りとして働く。

/// Rust のソースから、コメント (`//`、`///`、`//!`、入れ子を含む `/* */`) の中の文字だけを残し、
/// それ以外 (コード、文字列リテラル、文字リテラル、コメントの記号自身) を空白にした本文を返す。
pub fn rust_comments(src: &str) -> String {
    let chars: Vec<char> = src.chars().collect();
    let mut out: Vec<char> = chars
        .iter()
        .map(|&c| if c == '\n' { '\n' } else { ' ' })
        .collect();
    let mut i = 0;
    while i < chars.len() {
        let c = chars[i];
        let next = chars.get(i + 1).copied();
        if c == '/' && next == Some('/') {
            // 行コメント。`///` と `//!` の記号も本文から外す。
            let mut j = i + 2;
            if matches!(chars.get(j), Some('/') | Some('!')) {
                j += 1;
            }
            while j < chars.len() && chars[j] != '\n' {
                out[j] = chars[j];
                j += 1;
            }
            i = j;
        } else if c == '/' && next == Some('*') {
            // ブロックコメント。入れ子を深さで数える。
            let mut depth = 1;
            let mut j = i + 2;
            if matches!(chars.get(j), Some('*') | Some('!')) && chars.get(j + 1) != Some(&'/') {
                j += 1;
            }
            while j < chars.len() && depth > 0 {
                if chars[j] == '/' && chars.get(j + 1) == Some(&'*') {
                    depth += 1;
                    j += 2;
                } else if chars[j] == '*' && chars.get(j + 1) == Some(&'/') {
                    depth -= 1;
                    j += 2;
                } else {
                    out[j] = chars[j];
                    j += 1;
                }
            }
            i = j;
        } else if c == '"' {
            i = skip_string(&chars, i + 1);
        } else if (c == 'r' || (c == 'b' && next == Some('r'))) && !ident_char(&chars, i) {
            let start = if c == 'b' { i + 2 } else { i + 1 };
            let hashes = chars[start..].iter().take_while(|&&h| h == '#').count();
            if chars.get(start + hashes) == Some(&'"') {
                i = skip_raw_string(&chars, start + hashes + 1, hashes);
            } else {
                i += 1;
            }
        } else if c == '\'' {
            i = skip_char_or_lifetime(&chars, i);
        } else {
            i += 1;
        }
    }
    out.into_iter().collect()
}

/// `i` の直前が識別子の文字か (`bar"` の `r` を raw 文字列の始まりと取り違えないため)。
fn ident_char(chars: &[char], i: usize) -> bool {
    i > 0 && (chars[i - 1].is_alphanumeric() || chars[i - 1] == '_')
}

/// 通常の文字列リテラルの中身を読み飛ばし、閉じる `"` の次の位置を返す。
fn skip_string(chars: &[char], mut i: usize) -> usize {
    while i < chars.len() {
        match chars[i] {
            '\\' => i += 2,
            '"' => return i + 1,
            _ => i += 1,
        }
    }
    i
}

/// raw 文字列リテラルの中身を読み飛ばし、`"` と `#` の並びで閉じた次の位置を返す。
fn skip_raw_string(chars: &[char], mut i: usize, hashes: usize) -> usize {
    while i < chars.len() {
        if chars[i] == '"'
            && chars[i + 1..]
                .iter()
                .take(hashes)
                .filter(|&&h| h == '#')
                .count()
                == hashes
        {
            return i + 1 + hashes;
        }
        i += 1;
    }
    i
}

/// `'` から始まる文字リテラルを読み飛ばす。ライフタイム (`'a`) なら `'` だけを進める。
fn skip_char_or_lifetime(chars: &[char], i: usize) -> usize {
    match chars.get(i + 1) {
        Some('\\') => {
            let mut j = i + 2;
            while j < chars.len() && chars[j] != '\'' && chars[j] != '\n' {
                j += 1;
            }
            j + 1
        }
        Some(_) if chars.get(i + 2) == Some(&'\'') => i + 3,
        _ => i + 1,
    }
}

/// HTML の属性のうち、値を人が読む文として検査するもの。
const PROSE_ATTRIBUTES: &[&str] = &["aria-label", "title", "alt"];

/// HTML の要素のうち、中身を検査しないもの。`<style>` と `<script>` はコードで、`<code>` と
/// `<pre>` は Markdown のインラインコードとフェンスに当たる。
const SKIPPED_ELEMENTS: &[&str] = &["style", "script", "code", "pre"];

/// HTML から、人が読む文だけを残し、それ以外を空白にした本文を返す。
///
/// 残すのは、要素の中の本文 (SVG の `<text>` の中身を含む) と、`PROSE_ATTRIBUTES` に挙げた属性の
/// 値である。空白にするのは、タグそのもの (`<...>` と、その中の他の属性値)、`SKIPPED_ELEMENTS` の
/// 要素の中身、コメント (`<!-- -->`)、宣言 (`<!DOCTYPE ...>` など)、文字参照 (`&lt;` や `&#x3c;`)
/// である。タグで分かれた文字列 (`交<b>点</b>`) は、間に空白が入り、語の照合もそこで切れる。
pub fn html_text(src: &str) -> String {
    let chars: Vec<char> = src.chars().collect();
    let mut out: Vec<char> = chars
        .iter()
        .map(|&c| if c == '\n' { '\n' } else { ' ' })
        .collect();
    let mut i = 0;
    while i < chars.len() {
        let c = chars[i];
        let next = chars.get(i + 1).copied();
        if c == '<' && starts_with(&chars, i, "<!--") {
            i = find(&chars, i + 4, "-->").map_or(chars.len(), |e| e + 3);
        } else if c == '<' && matches!(next, Some('!') | Some('?')) {
            i = find(&chars, i + 2, ">").map_or(chars.len(), |e| e + 1);
        } else if c == '<' && next.is_some_and(|n| n.is_ascii_alphabetic() || n == '/') {
            let tag = read_tag(&chars, i, &mut out);
            i = tag.end;
            if !tag.closing && !tag.self_closing && SKIPPED_ELEMENTS.contains(&tag.name.as_str()) {
                i = find_close_tag(&chars, i, &tag.name);
            }
        } else {
            i = copy_text(&chars, i, chars.len(), &mut out);
        }
    }
    out.into_iter().collect()
}

/// 読んだタグ。`end` はタグを閉じる `>` の次の位置。
struct Tag {
    /// 小文字にした要素名
    name: String,
    /// 終了タグ (`</name>`) か
    closing: bool,
    /// `/>` で閉じたか
    self_closing: bool,
    end: usize,
}

/// `start` の `<` から始まるタグを読む。`PROSE_ATTRIBUTES` の属性の値だけを `out` へ写す。
fn read_tag(chars: &[char], start: usize, out: &mut [char]) -> Tag {
    let len = chars.len();
    let mut j = start + 1;
    let closing = chars.get(j) == Some(&'/');
    if closing {
        j += 1;
    }
    let name_start = j;
    while j < len && !chars[j].is_whitespace() && !matches!(chars[j], '>' | '/') {
        j += 1;
    }
    let name = lowercase(&chars[name_start..j]);
    loop {
        while j < len && (chars[j].is_whitespace() || chars[j] == '/') {
            j += 1;
        }
        if j >= len || chars[j] == '>' {
            return Tag {
                name,
                closing,
                self_closing: j < len && chars[j - 1] == '/',
                end: (j + 1).min(len),
            };
        }
        let attr_start = j;
        while j < len && !chars[j].is_whitespace() && !matches!(chars[j], '=' | '>' | '/') {
            j += 1;
        }
        let attr = lowercase(&chars[attr_start..j]);
        let mut k = j;
        while k < len && chars[k].is_whitespace() {
            k += 1;
        }
        if chars.get(k) != Some(&'=') {
            continue; // 値の無い属性
        }
        k += 1;
        while k < len && chars[k].is_whitespace() {
            k += 1;
        }
        let (value_start, value_end, after) = match chars.get(k) {
            Some(&q) if q == '"' || q == '\'' => {
                let e = chars[k + 1..]
                    .iter()
                    .position(|&c| c == q)
                    .map_or(len, |p| k + 1 + p);
                (k + 1, e, (e + 1).min(len))
            }
            _ => {
                let mut e = k;
                while e < len && !chars[e].is_whitespace() && chars[e] != '>' {
                    e += 1;
                }
                (k, e, e)
            }
        };
        if PROSE_ATTRIBUTES.contains(&attr.as_str()) {
            let mut v = value_start;
            while v < value_end {
                v = copy_text(chars, v, value_end, out);
            }
        }
        j = after;
    }
}

/// `i` の 1 文字を本文として `out` へ写し、次の位置を返す。`i` から文字参照 (`&name;`、`&#123;`、
/// `&#x7b;`) が `limit` までに閉じるなら、写さずにその次の位置を返す。
fn copy_text(chars: &[char], i: usize, limit: usize, out: &mut [char]) -> usize {
    if chars[i] == '&' {
        let mut j = i + 1;
        if chars.get(j) == Some(&'#') {
            j += 1;
        }
        let name_start = j;
        while j < chars.len() && chars[j].is_ascii_alphanumeric() && j - name_start < 32 {
            j += 1;
        }
        if j > name_start && j < limit && chars[j] == ';' {
            return j + 1;
        }
    }
    out[i] = chars[i];
    i + 1
}

/// `name` の要素を閉じる終了タグ (`</name` に名前の終わりが続くもの、大文字小文字を問わない) の
/// `<` の位置。無ければ入力の終わり。
fn find_close_tag(chars: &[char], from: usize, name: &str) -> usize {
    let pattern = format!("</{name}");
    let n = pattern.chars().count();
    let mut i = from;
    while i + n <= chars.len() {
        if lowercase(&chars[i..i + n]) == pattern
            && chars
                .get(i + n)
                .is_none_or(|&c| c.is_whitespace() || matches!(c, '>' | '/'))
        {
            return i;
        }
        i += 1;
    }
    chars.len()
}

fn lowercase(chars: &[char]) -> String {
    chars.iter().collect::<String>().to_ascii_lowercase()
}

fn starts_with(chars: &[char], i: usize, s: &str) -> bool {
    let s: Vec<char> = s.chars().collect();
    chars.get(i..i + s.len()) == Some(&s[..])
}

/// `from` 以降で `s` が始まる最初の位置。
fn find(chars: &[char], from: usize, s: &str) -> Option<usize> {
    (from..chars.len()).find(|&i| starts_with(chars, i, s))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keeps_only_comment_text_at_the_same_positions() {
        let src = "let s = \"交点\"; // 交点\n/// 説明\nfn f<'a>(c: char) {} /* ブロック /* 入れ子 */ 続き */\n";
        let out = rust_comments(src);
        let lines: Vec<&str> = out.lines().collect();
        assert_eq!(lines.len(), 3);
        assert_eq!(lines[0].trim(), "交点");
        assert_eq!(
            lines[0].chars().count(),
            src.lines().next().unwrap().chars().count()
        );
        assert_eq!(lines[0].chars().position(|c| c == '交'), Some(17));
        assert_eq!(lines[1].trim(), "説明");
        assert_eq!(lines[2].trim(), "ブロック    入れ子    続き");
    }

    #[test]
    fn strings_and_chars_do_not_open_comments() {
        let src =
            "let a = \"// 交点\"; let b = r#\"/* 交点 \"# ; let c = '\"'; let d = '\\''; // 本文\n";
        assert_eq!(rust_comments(src).trim(), "本文");
    }

    /// 1 行の、前後の空白を除いた中身を、連続する空白を 1 つに縮めて返す。
    fn squeezed(line: &str) -> String {
        line.split_whitespace().collect::<Vec<_>>().join(" ")
    }

    #[test]
    fn html_keeps_text_at_the_same_positions() {
        let src = "<p class=\"交点\">本文の交点</p>\n<div\n  title=\"題\">次</div>\n";
        let out = html_text(src);
        assert_eq!(out.lines().count(), src.lines().count());
        for (o, s) in out.lines().zip(src.lines()) {
            assert_eq!(o.chars().count(), s.chars().count(), "{s:?}");
        }
        let first = out.lines().next().unwrap();
        assert_eq!(first.chars().position(|c| c == '本'), Some(14));
        assert_eq!(squeezed(first), "本文の交点");
        let lines: Vec<&str> = out.lines().collect();
        assert_eq!(lines[1].trim(), "");
        assert_eq!(squeezed(lines[2]), "題 次");
    }

    #[test]
    fn html_code_style_script_comments_entities_and_other_attributes_are_not_checked() {
        let src = concat!(
            "<!DOCTYPE html><!-- 交点 -->\n",
            "<style>p::after { content: \"交点\"; }</style>\n",
            "<script>if (a < b) { s = \"</p>交点\"; }</script>\n",
            "<p>本文 <code>交点</code> と <PRE class=x>\n交点\n</pre>&lt;&#x4ea4;&amp;</p>\n",
            "<a href=\"交点\" data-x='交点' class=交点>本文</a>\n",
        );
        let out = html_text(src);
        assert!(!out.contains("交点"), "{out}");
        let words: Vec<String> = out.lines().map(squeezed).collect();
        assert_eq!(words, ["", "", "", "本文 と", "", "", "本文"]);
    }

    #[test]
    fn html_prose_attributes_and_svg_text_are_checked() {
        let src = concat!(
            "<svg><text x=\"1\" y=\"2\">図の交点</text></svg>\n",
            "<button aria-label=\"交点を開く\" TITLE='交点&amp;題'><img alt=交点 src=a.png/></button>\n",
        );
        let words: Vec<String> = html_text(src).lines().map(squeezed).collect();
        assert_eq!(words, ["図の交点", "交点を開く 交点 題 交点"]);
    }
}
