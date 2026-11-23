//! Project-wide index of `\label{…}` names and `.bib` keys, used for
//! `\ref` / `\cite` autocompletion.

use super::project::{self, FileNode};
use serde::Serialize;
use std::path::Path;

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct LabelRef {
    pub name: String,
    pub file: String,
    pub line: u32,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct CiteRef {
    pub key: String,
    /// Entry type, e.g. `article`.
    pub kind: String,
    /// Title if found, for the completion detail.
    pub title: Option<String>,
    pub file: String,
}

#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectRefs {
    pub labels: Vec<LabelRef>,
    pub citations: Vec<CiteRef>,
}

const MAX_FILE_BYTES: u64 = 4 * 1024 * 1024;

pub fn scan(root: &Path, tree: &[FileNode]) -> ProjectRefs {
    let mut refs = ProjectRefs::default();
    let mut files = Vec::new();
    collect(tree, &mut files);
    for rel in files {
        let Ok(abs) = project::resolve(root, &rel) else { continue };
        if abs.metadata().map(|m| m.len() > MAX_FILE_BYTES).unwrap_or(true) {
            continue;
        }
        let Ok(text) = std::fs::read_to_string(&abs) else { continue };
        if rel.to_lowercase().ends_with(".bib") {
            refs.citations.extend(bib_entries(&text, &rel));
        } else if rel.to_lowercase().ends_with(".typ") {
            refs.labels.extend(typst_labels(&text, &rel));
        } else {
            refs.labels.extend(labels(&text, &rel));
        }
    }
    refs
}

fn collect(nodes: &[FileNode], out: &mut Vec<String>) {
    for n in nodes {
        if n.is_dir {
            collect(&n.children, out);
        } else {
            let l = n.name.to_lowercase();
            if l.ends_with(".tex") || l.ends_with(".bib") || l.ends_with(".ltx") || l.ends_with(".typ") {
                out.push(n.path.clone());
            }
        }
    }
}

pub fn labels(text: &str, file: &str) -> Vec<LabelRef> {
    let mut out = Vec::new();
    for (i, line) in text.lines().enumerate() {
        let code = strip_comment(line);
        let mut rest = code;
        while let Some(pos) = rest.find("\\label{") {
            let after = &rest[pos + 7..];
            if let Some(end) = after.find('}') {
                let name = after[..end].trim();
                if !name.is_empty() {
                    out.push(LabelRef { name: name.to_string(), file: file.to_string(), line: i as u32 + 1 });
                }
                rest = &after[end..];
            } else {
                break;
            }
        }
    }
    out
}

pub fn bib_entries(text: &str, file: &str) -> Vec<CiteRef> {
    let mut out: Vec<CiteRef> = Vec::new();
    let mut rest = text;
    while let Some(at) = rest.find('@') {
        rest = &rest[at + 1..];
        let Some(brace) = rest.find(['{', '(']) else { break };
        let kind = rest[..brace].trim().to_lowercase();
        if kind.is_empty() || !kind.chars().all(|c| c.is_ascii_alphabetic()) || matches!(kind.as_str(), "comment" | "string" | "preamble") {
            continue;
        }
        let body = &rest[brace + 1..];
        let Some(comma) = body.find(',') else { continue };
        let key = body[..comma].trim();
        if key.is_empty() || key.contains(char::is_whitespace) {
            continue;
        }
        // Title: look within this entry only (until the next '@' at line start).
        let entry_end = body.find("\n@").unwrap_or(body.len());
        let title = field(&body[..entry_end], "title");
        out.push(CiteRef { key: key.to_string(), kind, title, file: file.to_string() });
        rest = &body[comma..];
    }
    out
}

fn field(entry: &str, name: &str) -> Option<String> {
    let lower = entry.to_lowercase();
    let mut search = 0;
    while let Some(p) = lower[search..].find(name) {
        let idx = search + p;
        let before_ok = idx == 0 || !lower.as_bytes()[idx - 1].is_ascii_alphanumeric();
        let after = lower[idx + name.len()..].trim_start();
        if before_ok && after.starts_with('=') {
            let raw = entry[entry.len() - after.len() + 1..].trim_start();
            let value = if let Some(v) = raw.strip_prefix('{') {
                let mut depth = 1;
                let end = v.char_indices().find(|(_, c)| {
                    match c {
                        '{' => depth += 1,
                        '}' => depth -= 1,
                        _ => {}
                    }
                    depth == 0
                })?;
                &v[..end.0]
            } else {
                let v = raw.strip_prefix('"')?;
                &v[..v.find('"')?]
            };
            let clean: String = value.chars().filter(|c| *c != '{' && *c != '}').collect();
            return Some(clean.split_whitespace().collect::<Vec<_>>().join(" "));
        }
        search = idx + name.len();
    }
    None
}

/// Typst labels: `<name>` after headings, figures, equations …
pub fn typst_labels(text: &str, file: &str) -> Vec<LabelRef> {
    let mut out = Vec::new();
    for (i, line) in text.lines().enumerate() {
        let code = line.split("//").next().unwrap_or(line);
        let b = code.as_bytes();
        let mut j = 0;
        while j < b.len() {
            if b[j] == b'<' {
                let rest = &code[j + 1..];
                let len = rest.find(|c: char| !(c.is_alphanumeric() || "-_.:".contains(c))).unwrap_or(rest.len());
                // A label must be closed directly by '>' and start with a letter.
                if len > 0 && rest[len..].starts_with('>') && rest.starts_with(|c: char| c.is_alphabetic()) {
                    out.push(LabelRef { name: rest[..len].to_string(), file: file.to_string(), line: i as u32 + 1 });
                    j += len + 2;
                    continue;
                }
            }
            j += 1;
        }
    }
    out
}

fn strip_comment(line: &str) -> &str {
    let b = line.as_bytes();
    for i in 0..b.len() {
        if b[i] == b'%' && (i == 0 || b[i - 1] != b'\\') {
            return &line[..i];
        }
    }
    line
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn finds_labels() {
        let l = labels("\\section{A}\\label{sec:a}\n% \\label{commented}\n\\label{eq:1} and \\label{eq:2}", "main.tex");
        let names: Vec<_> = l.iter().map(|l| (l.name.as_str(), l.line)).collect();
        assert_eq!(names, vec![("sec:a", 1), ("eq:1", 3), ("eq:2", 3)]);
    }

    #[test]
    fn finds_bib_entries() {
        let bib = "@comment{ignore}\n@Book{knuth84,\n  title = {The {\\TeX}book},\n  year=1984}\n@article(lamport, title = \"LaTeX\" )\n@string{x = \"y\"}";
        let e = bib_entries(bib, "refs.bib");
        assert_eq!(e.len(), 2);
        assert_eq!(e[0].key, "knuth84");
        assert_eq!(e[0].kind, "book");
        assert_eq!(e[0].title.as_deref(), Some("The \\TeXbook"));
        assert_eq!(e[1].key, "lamport");
        assert_eq!(e[1].title.as_deref(), Some("LaTeX"));
    }

    #[test]
    fn finds_typst_labels() {
        let l = typst_labels("= Intro <sec:intro>\n$ a < b > c $\n#figure(image(\"x.png\")) <fig-x> // <commented>\n", "main.typ");
        let names: Vec<_> = l.iter().map(|l| (l.name.as_str(), l.line)).collect();
        assert_eq!(names, vec![("sec:intro", 1), ("fig-x", 3)]);
    }
}
