---
name: sumi
description: Check and improve Japanese prose using sumi's bundled rules, including Markdown documents, Rust comments, and human-readable HTML text. Use when asked to check or remove unnatural or LLM-like phrasing in Japanese writing.
---

# sumi

Locate the executable beside the absolute path of this loaded `SKILL.md`: `sumi`
on Unix, or `sumi.exe` on Windows. Use that executable by its absolute path,
quoted for the shell. The skill directory and the working directory may differ.
The executable includes the rules and rewrite prompt and can run independently
of the original repository, Cargo, and commands on `PATH`.
Keep the accompanying `LICENSE` when redistributing the skill and executable.

For example, replace the example path with the actual path of the sibling binary:

```sh
sumi_bin="/absolute/path/to/skill/sumi"
"$sumi_bin" --file document.md
"$sumi_bin" --lang rust --file source.rs
"$sumi_bin" --lang html --file page.html
```

Check the document or draft requested by the user. Use `--file PATH`, or pipe UTF-8
text to stdin. File extensions `.md` and `.markdown`, `.rs`, and `.html` and `.htm`
select Markdown, Rust comments, and HTML text respectively. Use `--lang markdown`,
`--lang rust`, or `--lang html` to override detection. Stdin and files with unknown
extensions are read as Markdown. Only files with unknown extensions produce a
fallback notice, and only when there are hits. The notice goes to stdout before
the hits, or to stderr with `--check-only` or `--silent`. Stdin produces no notice.

Interpret the exit status:

- `0`: No hits; stdout is empty.
- `1`: Hits were found. Normal output contains the rewrite prompt, the rules, and
  findings with one-based line and character positions. This is a successful check
  with findings, so handle it before treating a nonzero exit as an execution error.
- `2`: An argument, input, or execution error occurred. Read stderr and resolve the
  error before relying on the result.

Use `--check-only` for findings without the rewrite prompt, or `--silent` for the
exit status alone. `--rules` lists the bundled rules; `--words` lists word records.

When asked to edit, read the emitted rewrite prompt and findings, revise the
requested text while preserving its meaning and facts, then run the check again.
Use context to rewrite sentences instead of mechanically replacing words. For a
check-only request, report the findings. If a finding is a false positive, preserve
the text and explain the rule issue. Stop and report remaining findings when
further edits would change meaning or repeated checks make no progress.
