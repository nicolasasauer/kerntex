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
    /// Family names of the authors (editors if there are none); a trailing
    /// `others` stays as it is.
    pub authors: Vec<String>,
    pub year: Option<String>,
    pub file: String,
}

#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectRefs {
    pub labels: Vec<LabelRef>,
    pub citations: Vec<CiteRef>,
}

const MAX_FILE_BYTES: u64 = 4 * 1024 * 1024;
/// A whole reference-manager library (Zotero, JabRef …) with abstracts easily
/// outgrows the limit for source files.
const MAX_BIB_BYTES: u64 = 32 * 1024 * 1024;

pub fn scan(root: &Path, tree: &[FileNode]) -> ProjectRefs {
    let mut refs = ProjectRefs::default();
    let mut files = Vec::new();
    collect(tree, &mut files);
    for rel in files {
        let Ok(abs) = project::resolve(root, &rel) else { continue };
        let is_bib = rel.to_lowercase().ends_with(".bib");
        let limit = if is_bib { MAX_BIB_BYTES } else { MAX_FILE_BYTES };
        if abs.metadata().map(|m| m.len() > limit).unwrap_or(true) {
            continue;
        }
        let Ok(text) = std::fs::read_to_string(&abs) else { continue };
        if is_bib {
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
        // Fields: look within this entry only (until the next '@' at line start).
        let entry_end = body.find("\n@").unwrap_or(body.len());
        let entry = &body[..entry_end];
        let title = field(entry, "title").map(|t| clean(&t));
        let authors = field(entry, "author").or_else(|| field(entry, "editor")).map(|a| family_names(&a)).unwrap_or_default();
        // biblatex writes `date = {2020-03-01}` instead of `year`.
        let year = field(entry, "year").or_else(|| field(entry, "date")).and_then(|y| first_year(&y));
        out.push(CiteRef { key: key.to_string(), kind, title, authors, year, file: file.to_string() });
        rest = &body[comma..];
    }
    out
}

/// The raw value of a field: `{…}`, `"…"` or a bare word like `1984`.
fn field(entry: &str, name: &str) -> Option<String> {
    // ASCII lowercasing keeps byte offsets valid for `entry`.
    let lower = entry.to_ascii_lowercase();
    let mut search = 0;
    while let Some(p) = lower[search..].find(name) {
        let idx = search + p;
        search = idx + name.len();
        let before_ok = idx == 0 || !lower.as_bytes()[idx - 1].is_ascii_alphanumeric();
        let after = lower[search..].trim_start();
        if !before_ok || !after.starts_with('=') {
            continue;
        }
        let raw = entry[entry.len() - after.len() + 1..].trim_start();
        let quoted = raw.starts_with('"');
        if !quoted && !raw.starts_with('{') {
            let end = raw.find(|c: char| !c.is_ascii_alphanumeric()).unwrap_or(raw.len());
            return Some(raw[..end].to_string());
        }
        // The value ends at the matching brace, or at a quote outside braces
        // (`{\"o}` doesn't end it).
        let v = &raw[1..];
        let mut depth = 0i32;
        let end = v.char_indices().find(|&(_, c)| {
            match c {
                '{' => depth += 1,
                '}' => depth -= 1,
                _ => {}
            }
            if quoted {
                depth == 0 && c == '"'
            } else {
                depth < 0
            }
        })?;
        return Some(v[..end.0].to_string());
    }
    None
}

/// A field value as readable text: common accents decoded, braces dropped,
/// whitespace collapsed.
fn clean(value: &str) -> String {
    const ACCENTS: [(char, &str, &str); 5] = [
        ('"', "aeiouyAEIOU", "äëïöüÿÄËÏÖÜ"),
        ('\'', "aeiouycnszAEIOUCS", "áéíóúýćńśźÁÉÍÓÚĆŚ"),
        ('`', "aeiouAEIOU", "àèìòùÀÈÌÒÙ"),
        ('^', "aeiouAEIOU", "âêîôûÂÊÎÔÛ"),
        ('~', "anoANO", "ãñõÃÑÕ"),
    ];
    let mut text = value.replace("{\\ss}", "ß").replace("\\ss{}", "ß");
    for (mark, plain, accented) in ACCENTS {
        if !text.contains(&format!("\\{mark}")) {
            continue;
        }
        for (p, a) in plain.chars().zip(accented.chars()) {
            // `\"{o}` and `\"o`
            text = text.replace(&format!("\\{mark}{{{p}}}"), &a.to_string()).replace(&format!("\\{mark}{p}"), &a.to_string());
        }
    }
    let text: String = text.chars().filter(|c| *c != '{' && *c != '}').collect();
    text.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// Split at `separator` outside braces, so `{Barnes and Noble}` stays whole.
fn split_top(value: &str, separator: impl Fn(char) -> bool) -> Vec<&str> {
    let (mut depth, mut start, mut out) = (0i32, 0, Vec::new());
    for (i, c) in value.char_indices() {
        match c {
            '{' => depth += 1,
            '}' => depth -= 1,
            c if depth == 0 && separator(c) => {
                out.push(&value[start..i]);
                start = i + c.len_utf8();
            }
            _ => {}
        }
    }
    out.push(&value[start..]);
    out
}

/// `Knuth, Donald E. and Leslie Lamport` → `Knuth`, `Lamport`.
fn family_names(value: &str) -> Vec<String> {
    let words: Vec<&str> = split_top(value, char::is_whitespace).into_iter().filter(|w| !w.is_empty()).collect();
    words
        .split(|w| w.eq_ignore_ascii_case("and"))
        .filter_map(|name| {
            let name = name.join(" ");
            let parts = split_top(&name, |c| c == ',');
            let family = if parts.len() > 1 { parts[0] } else { split_top(&name, char::is_whitespace).last().copied().unwrap_or("") };
            Some(clean(family)).filter(|f| !f.is_empty())
        })
        .collect()
}

/// The first four-digit run, e.g. from `2020-03-01`.
fn first_year(value: &str) -> Option<String> {
    value.split(|c: char| !c.is_ascii_digit()).find(|part| part.len() == 4).map(str::to_string)
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
    fn reads_authors_and_year() {
        let bib = r#"@article{a, author = {Knuth, Donald E. and Leslie Lamport and others}, year = 1984, title = {A}}
@book{b, editor = {{World Health Organization}}, date = {2020-03-01}}
@misc{c, author = "M{\"u}ller, J{\'e} and Wei{\ss}, Eva", urldate = {2024-01-01}}
@misc{d, title = {İstanbul ẞ}, author = {Ada Lovelace}}"#;
        let e = bib_entries(bib, "refs.bib");
        let got: Vec<_> = e.iter().map(|e| (e.authors.join("|"), e.year.as_deref())).collect();
        assert_eq!(
            got,
            vec![
                ("Knuth|Lamport|others".to_string(), Some("1984")),
                ("World Health Organization".to_string(), Some("2020")),
                ("Müller|Weiß".to_string(), None),
                ("Lovelace".to_string(), None),
            ]
        );
        assert_eq!(e[3].title.as_deref(), Some("İstanbul ẞ"));
    }

    #[test]
    fn finds_typst_labels() {
        let l = typst_labels("= Intro <sec:intro>\n$ a < b > c $\n#figure(image(\"x.png\")) <fig-x> // <commented>\n", "main.typ");
        let names: Vec<_> = l.iter().map(|l| (l.name.as_str(), l.line)).collect();
        assert_eq!(names, vec![("sec:intro", 1), ("fig-x", 3)]);
    }
}
