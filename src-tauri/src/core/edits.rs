//! Targeted text edits, project search and content revisions — the pure
//! logic behind the MCP tools `edit_file`, `search` and the `revision` that
//! lets an agent detect concurrent changes.

use super::error::{AppError, ErrorSource, Result};
use serde::Deserialize;

/// Replace `old_text` (which must occur exactly once) with `new_text`.
#[derive(Debug, Clone, Deserialize, schemars::JsonSchema)]
pub struct TextEdit {
    /// Text to replace, copied exactly from the file (including whitespace and line breaks). It must occur exactly once; include surrounding lines to make it unique.
    pub old_text: String,
    /// Replacement text.
    pub new_text: String,
}

/// Applies `edits` one after another. Each `old_text` must match exactly once
/// in the text as left by the previous edits. Nothing is changed if any edit
/// fails. Files with Windows line endings (CRLF) also accept edits written
/// with plain `\n`.
pub fn apply_edits(path: &str, text: &str, edits: &[TextEdit]) -> Result<String> {
    if edits.is_empty() {
        return Err(edit_error(path, "No edits given", "Pass at least one { old_text, new_text } pair."));
    }
    let crlf = text.contains("\r\n");
    let mut out = text.to_string();
    for (i, edit) in edits.iter().enumerate() {
        let n = i + 1;
        if edit.old_text.is_empty() {
            if out.is_empty() {
                out = edit.new_text.clone();
                continue;
            }
            return Err(edit_error(
                path,
                &format!("Edit {n}: old_text is empty"),
                "Copy the lines you want to change into old_text. To add text, include a nearby line in old_text and repeat it in new_text.",
            ));
        }
        let (old, new) = match count(&out, &edit.old_text) {
            0 if crlf && !edit.old_text.contains("\r\n") => (to_crlf(&edit.old_text), to_crlf(&edit.new_text)),
            _ => (edit.old_text.clone(), edit.new_text.clone()),
        };
        match count(&out, &old) {
            1 => out = out.replacen(&old, &new, 1),
            0 => {
                return Err(edit_error(
                    path,
                    &format!("Edit {n}: old_text wasn't found in {path}"),
                    "Read the file again with read_file (it may have changed) and copy old_text exactly, including spaces, tabs and line breaks.",
                ))
            }
            k => {
                return Err(edit_error(
                    path,
                    &format!("Edit {n}: old_text occurs {k} times in {path}"),
                    "old_text must be unique. Include more surrounding lines so it matches exactly one place.",
                ))
            }
        }
    }
    Ok(out)
}

fn count(haystack: &str, needle: &str) -> usize {
    haystack.match_indices(needle).count()
}

fn to_crlf(s: &str) -> String {
    s.replace("\r\n", "\n").replace('\n', "\r\n")
}

fn edit_error(path: &str, title: &str, hint: &str) -> AppError {
    AppError::new(ErrorSource::App, title).detail(path.to_string()).hint(hint)
}

/// Short fingerprint of a file's content (FNV-1a, 64 bit). Equal text ⇔
/// equal revision, so it needs no stored state.
pub fn revision(text: &str) -> String {
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    for b in text.as_bytes() {
        h ^= u64::from(*b);
        h = h.wrapping_mul(0x0000_0100_0000_01b3);
    }
    format!("{h:016x}")
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, schemars::JsonSchema)]
pub struct SearchHit {
    /// Project-relative path.
    pub path: String,
    /// 1-based line number.
    pub line: usize,
    /// The matching line (shortened if very long).
    pub text: String,
}

/// Longest line excerpt returned per hit.
const MAX_LINE_CHARS: usize = 240;

/// Appends the lines of `text` containing `query` to `out`, up to `max` hits
/// in total. Returns false once `max` is reached.
pub fn search_text(path: &str, text: &str, query: &str, case_sensitive: bool, max: usize, out: &mut Vec<SearchHit>) -> bool {
    if query.is_empty() {
        return true;
    }
    let q = if case_sensitive { query.to_string() } else { query.to_lowercase() };
    for (i, line) in text.lines().enumerate() {
        let hit = if case_sensitive { line.contains(&q) } else { line.to_lowercase().contains(&q) };
        if !hit {
            continue;
        }
        if out.len() >= max {
            return false;
        }
        let trimmed = line.trim();
        let text = if trimmed.chars().count() > MAX_LINE_CHARS {
            format!("{}…", trimmed.chars().take(MAX_LINE_CHARS).collect::<String>())
        } else {
            trimmed.to_string()
        };
        out.push(SearchHit { path: path.to_string(), line: i + 1, text });
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    fn e(old: &str, new: &str) -> TextEdit {
        TextEdit { old_text: old.into(), new_text: new.into() }
    }

    #[test]
    fn applies_unique_edits_in_order() {
        let text = "\\section{Intro}\nHello world.\n\\section{End}\n";
        let out = apply_edits("main.tex", text, &[e("Hello world.", "Hello KernTeX."), e("{End}", "{Conclusion}")]).unwrap();
        assert_eq!(out, "\\section{Intro}\nHello KernTeX.\n\\section{Conclusion}\n");
    }

    #[test]
    fn later_edits_see_earlier_results() {
        let out = apply_edits("a.tex", "one", &[e("one", "two"), e("two", "three")]).unwrap();
        assert_eq!(out, "three");
    }

    #[test]
    fn refuses_missing_and_ambiguous_text_without_changing_anything() {
        let err = apply_edits("a.tex", "x x", &[e("x", "y")]).unwrap_err();
        assert!(err.title.contains("occurs 2 times"), "{}", err.title);
        let err = apply_edits("a.tex", "abc", &[e("b", "B"), e("zzz", "")]).unwrap_err();
        assert!(err.title.starts_with("Edit 2"), "{}", err.title);
        assert!(err.hint.unwrap().contains("read_file"));
    }

    #[test]
    fn empty_old_text_only_fills_an_empty_file() {
        assert_eq!(apply_edits("new.tex", "", &[e("", "hello")]).unwrap(), "hello");
        assert!(apply_edits("a.tex", "x", &[e("", "y")]).is_err());
        assert!(apply_edits("a.tex", "x", &[]).is_err());
    }

    #[test]
    fn accepts_lf_edits_in_crlf_files() {
        let text = "a\r\nb\r\nc\r\n";
        let out = apply_edits("w.tex", text, &[e("a\nb", "A\nB")]).unwrap();
        assert_eq!(out, "A\r\nB\r\nc\r\n");
    }

    #[test]
    fn revision_tracks_content() {
        assert_eq!(revision("abc"), revision("abc"));
        assert_ne!(revision("abc"), revision("abd"));
        assert_eq!(revision("").len(), 16);
    }

    #[test]
    fn search_finds_lines_case_insensitively_and_stops_at_max() {
        let mut hits = Vec::new();
        assert!(search_text("a.tex", "Foo\nbar\nfoo bar", "FOO", false, 10, &mut hits));
        assert_eq!(hits.iter().map(|h| h.line).collect::<Vec<_>>(), vec![1, 3]);
        let mut hits = Vec::new();
        assert!(!search_text("a.tex", "x\nx\nx", "x", true, 2, &mut hits));
        assert_eq!(hits.len(), 2);
        let mut hits = Vec::new();
        search_text("a.tex", "Foo", "foo", true, 10, &mut hits);
        assert!(hits.is_empty());
    }
}
