//! Turns raw TeX / Tectonic output into structured, actionable diagnostics.
//!
//! Two input formats are handled:
//! * Tectonic's console output: `error: main.tex:12: Undefined control sequence`
//! * A classic TeX `.log` written with `-file-line-error`:
//!   `./main.tex:12: Undefined control sequence.` followed by `l.12 \foo`,
//!   plus `LaTeX Warning: ... on input line 12.` style warnings.

use serde::Serialize;

#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "camelCase")]
pub enum Severity {
    Error,
    Warning,
    Info,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct Diagnostic {
    pub severity: Severity,
    /// Who reported it: "LaTeX", "Tectonic", "BibTeX", "Biber", "latexmk", a package name …
    pub source: String,
    /// Project-relative file, if known.
    pub file: Option<String>,
    pub line: Option<u32>,
    /// 1-based column and end position, where the engine reports them (Typst does).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub column: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub end_line: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub end_column: Option<u32>,
    pub message: String,
    /// Plain-language explanation and fix.
    pub hint: Option<String>,
}

/// Tectonic console output (stderr + stdout).
pub fn parse_tectonic(output: &str) -> Vec<Diagnostic> {
    let mut out = Vec::new();
    let lines: Vec<&str> = output.lines().collect();
    let mut i = 0;
    while i < lines.len() {
        let line = lines[i].trim_end();
        // Tectonic prints the TeX warnings of every pass. Those of an earlier pass
        // are often stale (undefined refs and citations widen lines until BibTeX
        // has run); real ones are printed again by the final pass.
        if line.starts_with("note: Rerunning TeX") {
            out.retain(|d: &Diagnostic| !(d.severity == Severity::Warning && d.file.is_some()));
            i += 1;
            continue;
        }
        let (severity, rest) = if let Some(r) = line.strip_prefix("error: ") {
            (Severity::Error, r)
        } else if let Some(r) = line.strip_prefix("warning: ") {
            (Severity::Warning, r)
        } else {
            i += 1;
            continue;
        };
        // Tectonic indents continuation lines ("caused by:", context).
        let mut message = rest.to_string();
        let mut j = i + 1;
        while j < lines.len() && lines[j].starts_with("    ") && !lines[j].trim().is_empty() {
            if let Some(cause) = lines[j].trim().strip_prefix("caused by: ") {
                message = format!("{message}: {cause}");
            }
            j += 1;
        }
        if let Some((file, ln, msg)) = split_file_line(rest) {
            message = msg.to_string();
            push(&mut out, severity, source_for(&message, "LaTeX"), Some(file), Some(ln), message);
        } else if !is_tectonic_noise(&message) {
            let source = if message.contains("biber") {
                "Biber"
            } else if message.contains("bibtex") {
                "BibTeX"
            } else {
                "Tectonic"
            };
            push(&mut out, severity, source.to_string(), None, None, message);
        }
        i = j;
    }
    finish(out)
}

/// Classic TeX log, written with `-file-line-error` and a long `max_print_line`.
pub fn parse_tex_log(log: &str) -> Vec<Diagnostic> {
    let mut out = Vec::new();
    let mut files = FileStack::default();
    let lines: Vec<&str> = log.lines().collect();
    let mut i = 0;
    while i < lines.len() {
        let line = lines[i];

        // `./main.tex:12: Undefined control sequence.`
        if let Some((file, ln, msg)) = split_file_line(line) {
            if is_tex_noise(msg) {
                i += 1;
                continue;
            }
            let mut message = msg.trim_end_matches('.').to_string();
            // Append the offending source snippet (`l.12 \foo`) when present.
            if let Some(ctx) = lines[i + 1..].iter().take(8).find(|l| l.starts_with(&format!("l.{ln} "))) {
                let snippet = ctx[format!("l.{ln} ").len()..].trim();
                if !snippet.is_empty() && message.len() < 120 {
                    message = format!("{message} — near “{snippet}”");
                }
            }
            push(&mut out, Severity::Error, source_for(msg, "LaTeX"), Some(file), Some(ln), message);
            i += 1;
            continue;
        }

        // `! LaTeX Error: ...` (no file:line form, e.g. before the first file opened)
        if let Some(msg) = line.strip_prefix("! ") {
            let ln =
                lines[i + 1..].iter().take(10).find_map(|l| l.strip_prefix("l.").and_then(|r| r.split_whitespace().next()?.parse().ok()));
            if !is_tex_noise(msg) {
                push(&mut out, Severity::Error, source_for(msg, "LaTeX"), files.current(), ln, msg.trim_end_matches('.').to_string());
            }
            i += 1;
            continue;
        }

        // Warnings, possibly continued on following lines prefixed by `(pkg)`.
        if let Some((source, msg)) = warning_start(line) {
            let mut message = msg.trim().to_string();
            let mut j = i + 1;
            let cont_prefix = format!("({})", source.to_lowercase());
            while j < lines.len() {
                let l = lines[j].trim_start();
                if l.to_lowercase().starts_with(&cont_prefix) {
                    message.push(' ');
                    message.push_str(l[cont_prefix.len()..].trim());
                    j += 1;
                } else {
                    break;
                }
            }
            let ln = extract_input_line(&message);
            let message = strip_input_line(&message);
            let severity = if ["may have changed", "Rerun"].iter().any(|k| message.contains(k)) || is_bibtex_fallback(&message) {
                Severity::Info
            } else {
                Severity::Warning
            };
            push(&mut out, severity, source, files.current(), ln, message);
            files.scan(line);
            i = j;
            continue;
        }

        // Overfull / underfull boxes.
        if line.starts_with("Overfull \\") || line.starts_with("Underfull \\") {
            let ln = line
                .split("at lines ")
                .nth(1)
                .or_else(|| line.split("at line ").nth(1))
                .and_then(|r| r.split(|c: char| !c.is_ascii_digit()).next()?.parse().ok());
            let severity = if line.starts_with("Overfull") { Severity::Warning } else { Severity::Info };
            let message = line.split(" in paragraph").next().unwrap_or(line).split(" detected at").next().unwrap_or(line).to_string();
            push(&mut out, severity, "LaTeX".into(), files.current(), ln, message);
            i += 1;
            continue;
        }

        files.scan(line);
        i += 1;
    }
    finish(out)
}

/// TeX reports a missing package or class on the line it had read up to when
/// it gave up, usually the one *after* `\usepackage{…}`. Point such errors at
/// the package name in the `\usepackage` / `\documentclass` line instead, and
/// select it. `read` returns the text of a project file; when the name can't be
/// found there, TeX's line is kept.
pub fn locate_missing_files(diags: &mut [Diagnostic], mut read: impl FnMut(&str) -> Option<String>) {
    for d in diags.iter_mut() {
        let Some(file) = d.file.clone() else { continue };
        let Some((name, ext)) = missing_file_name(&d.message) else { continue };
        let commands: &[&str] = match ext {
            "sty" => &["\\usepackage", "\\RequirePackage", "\\RequirePackageWithOptions"],
            "cls" => &["\\documentclass", "\\LoadClass", "\\LoadClassWithOptions"],
            _ => continue,
        };
        let Some(text) = read(&file) else { continue };
        if let Some((line, column, end_column)) = find_loaded_name(&text, commands, name) {
            d.line = Some(line);
            d.column = Some(column);
            d.end_line = Some(line);
            d.end_column = Some(end_column);
            // The "near …" snippet showed TeX's line, which is no longer the one meant.
            if let Some(idx) = d.message.find(" — near “") {
                d.message.truncate(idx);
            }
        }
    }
}

/// ``File `foo.sty' not found`` → ("foo", "sty").
fn missing_file_name(msg: &str) -> Option<(&str, &str)> {
    let start = msg.find("File `").or_else(|| msg.find("file `"))? + 6;
    let rest = &msg[start..];
    let end = rest.find('\'')?;
    if !rest[end..].contains("not found") {
        return None;
    }
    let (name, ext) = rest[..end].rsplit_once('.')?;
    (!name.is_empty()).then_some((name, ext))
}

/// Finds `name` in the braced list of the first matching load command
/// (`\usepackage[opts]{a, name, b}`), ignoring comments. Returns the 1-based
/// line and the start/end columns (UTF-16, as the editor counts them).
fn find_loaded_name(text: &str, commands: &[&str], name: &str) -> Option<(u32, u32, u32)> {
    // Blank out comments so `% \usepackage{x}` doesn't count; byte positions stay.
    let mut code = text.as_bytes().to_vec();
    let mut i = 0;
    while i < code.len() {
        if code[i] == b'%' && code[..i].iter().rev().take_while(|&&b| b == b'\\').count() % 2 == 0 {
            while i < code.len() && code[i] != b'\n' {
                code[i] = b' ';
                i += 1;
            }
        }
        i += 1;
    }
    let mut best: Option<usize> = None;
    for cmd in commands {
        let cmd = cmd.as_bytes();
        let mut from = 0;
        while let Some(pos) = find_bytes(&code[from..], cmd).map(|p| p + from) {
            from = pos + cmd.len();
            if code.get(from).is_some_and(|b| b.is_ascii_alphabetic()) {
                continue; // a longer command name, e.g. \usepackagex
            }
            if let Some(at) = name_in_args(&code, from, name.as_bytes()) {
                if best.is_none_or(|b| at < b) {
                    best = Some(at);
                }
                break;
            }
        }
    }
    let at = best?;
    let line_start = text[..at].rfind('\n').map_or(0, |p| p + 1);
    let line = text[..at].matches('\n').count() as u32 + 1;
    let column = text[line_start..at].encode_utf16().count() as u32 + 1;
    Some((line, column, column + name.encode_utf16().count() as u32))
}

/// After a command name: skips `[…]` and returns the byte offset of `name` in
/// the following `{…}` list.
fn name_in_args(code: &[u8], mut i: usize, name: &[u8]) -> Option<usize> {
    let skip_ws = |i: &mut usize| {
        while code.get(*i).is_some_and(|b| b.is_ascii_whitespace()) {
            *i += 1;
        }
    };
    skip_ws(&mut i);
    if code.get(i) == Some(&b'[') {
        let mut depth = 0;
        while i < code.len() {
            match code[i] {
                b'[' => depth += 1,
                b']' => {
                    depth -= 1;
                    if depth == 0 {
                        break;
                    }
                }
                _ => {}
            }
            i += 1;
        }
        i += 1;
        skip_ws(&mut i);
    }
    if code.get(i) != Some(&b'{') {
        return None;
    }
    let close = i + code[i..].iter().position(|&b| b == b'}')?;
    let mut item = i + 1;
    for part in code[i + 1..close].split(|&b| b == b',') {
        let lead = part.iter().take_while(|b| b.is_ascii_whitespace()).count();
        let trimmed = part[lead..].trim_ascii_end();
        if trimmed == name {
            return Some(item + lead);
        }
        item += part.len() + 1;
    }
    None
}

fn find_bytes(hay: &[u8], needle: &[u8]) -> Option<usize> {
    hay.windows(needle.len()).position(|w| w == needle)
}

/// BibTeX `.blg` / Biber output: surfaces missing entries and syntax errors.
pub fn parse_bib_log(log: &str, biber: bool) -> Vec<Diagnostic> {
    let src = if biber { "Biber" } else { "BibTeX" };
    let mut out = Vec::new();
    for line in log.lines() {
        let l = line.trim();
        let (sev, msg) = if let Some(m) = l.strip_prefix("Warning--") {
            (Severity::Warning, m)
        } else if let Some(m) = l.split_once("] WARN - ").map(|(_, m)| m) {
            (Severity::Warning, m)
        } else if let Some(m) = l.split_once("] ERROR - ").map(|(_, m)| m) {
            (Severity::Error, m)
        } else if l.starts_with("I couldn't open") || l.contains("---line ") {
            (Severity::Error, l)
        } else {
            continue;
        };
        push(&mut out, sev, src.into(), None, None, msg.to_string());
    }
    finish(out)
}

fn push(out: &mut Vec<Diagnostic>, severity: Severity, source: String, file: Option<String>, line: Option<u32>, message: String) {
    let file = file.map(|f| normalize_file(&f));
    let hint = hint_for(&message);
    let d = Diagnostic { severity, source, file, line, column: None, end_line: None, end_column: None, message, hint };
    if !out.contains(&d) {
        out.push(d);
    }
}

/// Errors first; drop generic summary lines when concrete errors exist.
fn finish(mut out: Vec<Diagnostic>) -> Vec<Diagnostic> {
    out.sort_by_key(|d| d.severity);
    out
}

fn normalize_file(f: &str) -> String {
    let f = f.replace('\\', "/");
    f.trim_start_matches("./").to_string()
}

/// `path/file.tex:12: message` → (file, 12, message). Paths may contain `:`
/// on Windows (`C:/x.tex:3: ...`), so search for the `:<digits>: ` pattern.
fn split_file_line(s: &str) -> Option<(String, u32, &str)> {
    let bytes = s.as_bytes();
    let mut start = 0;
    while let Some(pos) = s[start..].find(':') {
        let colon = start + pos;
        let digits_end = bytes[colon + 1..].iter().position(|b| !b.is_ascii_digit()).map(|p| colon + 1 + p)?;
        if digits_end > colon + 1 && bytes.get(digits_end) == Some(&b':') && bytes.get(digits_end + 1) == Some(&b' ') {
            let file = &s[..colon];
            if file.is_empty() || !looks_like_tex_path(file) {
                return None;
            }
            let ln = s[colon + 1..digits_end].parse().ok()?;
            return Some((file.to_string(), ln, s[digits_end + 2..].trim()));
        }
        start = colon + 1;
    }
    None
}

fn looks_like_tex_path(file: &str) -> bool {
    let lower = file.to_lowercase();
    [".tex", ".sty", ".cls", ".bib", ".ltx", ".dtx", ".bbl", ".aux", ".def", ".cfg", ".clo", ".fd"].iter().any(|e| lower.ends_with(e))
}

fn warning_start(line: &str) -> Option<(String, &str)> {
    if let Some(m) = line.strip_prefix("LaTeX Warning: ") {
        return Some(("LaTeX".into(), m));
    }
    if let Some(m) = line.strip_prefix("LaTeX Font Warning: ") {
        return Some(("LaTeX Font".into(), m));
    }
    for prefix in ["Package ", "Class "] {
        if let Some(rest) = line.strip_prefix(prefix) {
            if let Some((name, msg)) = rest.split_once(" Warning: ") {
                if !name.contains(' ') {
                    return Some((name.to_string(), msg));
                }
            }
        }
    }
    None
}

fn extract_input_line(msg: &str) -> Option<u32> {
    let idx = msg.rfind("input line ")?;
    msg[idx + 11..].split(|c: char| !c.is_ascii_digit()).next()?.parse().ok()
}

fn strip_input_line(msg: &str) -> String {
    match msg.rfind(" on input line ") {
        Some(idx) => msg[..idx].to_string(),
        None => msg.trim_end_matches('.').to_string(),
    }
}

fn source_for(msg: &str, default: &str) -> String {
    // `Package foo Error: ...` / `Class foo Error: ...`
    for prefix in ["Package ", "Class "] {
        if let Some(rest) = msg.strip_prefix(prefix) {
            if let Some((name, _)) = rest.split_once(" Error") {
                if !name.contains(' ') {
                    return name.to_string();
                }
            }
        }
    }
    default.to_string()
}

fn is_tectonic_noise(msg: &str) -> bool {
    msg.starts_with("halted on potentially-recoverable error")
        || msg.starts_with("the TeX engine had an error")
        || msg.starts_with("failed to run ")
        || msg.starts_with("an error occurred while processing")
}

/// biblatex's note that it runs BibTeX, not Biber ("fall-back BibTeX(8) backend").
fn is_bibtex_fallback(msg: &str) -> bool {
    msg.to_lowercase().contains("fall-back bibtex")
}

/// Follow-up messages that only repeat that TeX gave up after a real error.
fn is_tex_noise(msg: &str) -> bool {
    let m = msg.trim_start();
    m.starts_with("Emergency stop") || m.starts_with("==> Fatal error") || m.starts_with("*** (job aborted")
}

fn is_network_error(m: &str) -> bool {
    let l = m.to_lowercase();
    [
        "error sending request",
        "failed to lookup address",
        "dns error",
        "connection refused",
        "network is unreachable",
        "timed out",
        "tcp connect error",
        "could not resolve host",
    ]
    .iter()
    .any(|k| l.contains(k))
}

/// Plain-language help for common Typst messages that come without a hint.
pub fn typst_hint(msg: &str) -> Option<String> {
    let m = msg.to_lowercase();
    let h = if m.starts_with("unknown variable") {
        "This name isn't defined. Check the spelling, define it with #let, or import it (e.g. #import \"@preview/…\": name)."
    } else if m.starts_with("expected") {
        "Typst expected something else here — often a missing closing bracket, parenthesis or quote just before this spot."
    } else if m.contains("unclosed delimiter") {
        "A bracket, parenthesis or brace is opened but never closed. Add the closing counterpart."
    } else if m.contains("file not found") {
        "The file doesn't exist at this path. Paths are relative to the file that uses them; check the name and folder."
    } else if m.contains("package not found") || m.contains("failed to download package") {
        "Typst couldn't get this package. Check the name and version on typst.app/universe, and that you're online — packages are downloaded once and then cached."
    } else if m.contains("label") && m.contains("does not exist") {
        "No element has this label. Add <label> after the heading or figure you want to reference, or fix the spelling."
    } else if m.contains("unknown font family") {
        "This font isn't installed. Install it, or pick another font in #set text(font: \"…\")."
    } else {
        return None;
    };
    Some(h.to_string())
}

/// Plain-language explanations for the most common problems.
pub fn hint_for(msg: &str) -> Option<String> {
    let m = msg;
    let quoted = || -> Option<String> {
        // Text between `…' or "…"
        let start = m.find('`').or_else(|| m.find('"')).map(|i| i + 1)?;
        let end = m[start..].find(['\'', '"']).map(|e| start + e)?;
        Some(m[start..end].to_string())
    }()
    .unwrap_or_default();

    let hint = if m.contains("Undefined control sequence") {
        "This command isn't defined. Check its spelling, or load the package that provides it with \\usepackage{…} in the preamble."
            .to_string()
    } else if m.contains("Missing $ inserted") {
        "Math-only content (e.g. ^, _ or \\alpha) appears outside math mode. Wrap it in $…$, or write the character literally as \\_ or \\^{}.".to_string()
    } else if (m.contains("File `") || m.contains("file `")) && m.contains("not found") || m.contains("I can't find file") {
        let name = if quoted.is_empty() { "the file".to_string() } else { format!("“{quoted}”") };
        format!("LaTeX can't find {name}. If it's a package, check the name — Tectonic downloads packages automatically, with latexmk install it via your TeX distribution (tlmgr / MiKTeX Console). If it's one of your files, check the path relative to the main file.")
    } else if m.contains("Missing \\begin{document}") {
        "Something is typeset in the preamble (before \\begin{document}). Move the text below \\begin{document}, or look for a stray character.".to_string()
    } else if m.contains("Environment") && m.contains("undefined") {
        "This environment doesn't exist. Check the spelling in \\begin{…}, or load the package that defines it.".to_string()
    } else if m.contains("ended by \\end{") || m.contains("\\begin{document} ended by") {
        "\\begin and \\end don't match. Every \\begin{x} must be closed by \\end{x}, and nested environments must close in reverse order."
            .to_string()
    } else if m.contains("Runaway argument") || m.contains("Paragraph ended before") || m.contains("File ended while scanning") {
        "An opening brace { (or bracket) is never closed. Look just before the reported line for a missing }.".to_string()
    } else if m.contains("Too many }'s") || m.contains("Extra }") {
        "There's a closing brace } without a matching {. Remove it or add the missing opening brace.".to_string()
    } else if m.contains("Extra alignment tab") {
        "A table row has more & separators than the column specification allows. Add a column to \\begin{tabular}{…} or remove an &."
            .to_string()
    } else if m.contains("Misplaced alignment tab") {
        "& is only allowed inside tables and alignments. Write \\& for a literal ampersand.".to_string()
    } else if m.contains("Citation") && m.contains("undefined") {
        "No bibliography entry has this key. Check the key in your .bib file and that the bibliography is included (\\bibliography{…} or \\addbibresource{…}).".to_string()
    } else if m.contains("Reference") && m.contains("undefined") {
        "No matching \\label{…} was found. Check the spelling — references also stay unresolved until the next compile pass.".to_string()
    } else if m.contains("multiply defined") || m.contains("multiply-defined") {
        "The same \\label is used more than once. Give every label a unique name.".to_string()
    } else if m.starts_with("Overfull \\hbox") {
        "A line sticks out into the margin. Rephrase it, allow hyphenation (e.g. long URLs with \\url{…}), or load \\usepackage{microtype}."
            .to_string()
    } else if m.starts_with("Underfull") {
        "A line or page is stretched more than LaTeX likes. Usually harmless; often caused by \\\\ or \\newline at the end of a paragraph."
            .to_string()
    } else if m.contains("not set up for use with LaTeX") || m.contains("Invalid UTF-8") {
        "A special Unicode character can't be typeset with pdfLaTeX. Replace it with a LaTeX command, or switch the engine to XeLaTeX/LuaLaTeX (Tectonic supports Unicode natively).".to_string()
    } else if m.contains("Font") && m.contains("not found") || m.contains("The font") && m.contains("cannot be found") {
        "This font isn't installed. Install it on your system or pick another font in \\setmainfont{…}.".to_string()
    } else if m.contains("Option clash") {
        "The same package is loaded twice with different options. Load it once with all options, or use \\PassOptionsToPackage{…}{…} before \\documentclass.".to_string()
    } else if m.contains("Can be used only in preamble") {
        "This command must be placed before \\begin{document}.".to_string()
    } else if m.contains("There's no line here to end") {
        "\\\\ or \\newline is used where there is no line, e.g. right after a paragraph break. Remove it, or use \\vspace{…} for extra space.".to_string()
    } else if m.contains("Float(s) lost") || m.contains("Not in outer par mode") {
        "A figure or table is placed somewhere floats aren't allowed (e.g. inside a minipage or another float). Move it out, or use [H] from the float package.".to_string()
    } else if is_bibtex_fallback(m) {
        "biblatex uses BibTeX instead of Biber here. That's fine for most documents (and what the built-in Tectonic engine supports); install Biber and remove `backend=bibtex` if you need Biber-only features.".to_string()
    } else if m.contains("Label(s) may have changed") || m.contains("Rerun") {
        "Cross-references changed. KernTeX re-runs automatically; if it persists, compile once more.".to_string()
    } else if m.contains("I didn't find a database entry") {
        let key = if quoted.is_empty() { String::new() } else { format!(" for “{quoted}”") };
        format!("Your .bib file has no entry{key}. Check the citation key and that the right .bib file is referenced.")
    } else if m.contains("I couldn't open database file") || m.contains("Cannot find") && m.contains(".bib") {
        "The .bib file couldn't be found. Check the name in \\bibliography{…} / \\addbibresource{…} (path relative to the main file)."
            .to_string()
    } else if is_network_error(m) {
        "Tectonic needs an internet connection to download TeX packages — always on the first compile, and later whenever a document uses a package it hasn't downloaded yet. Connect to the internet and compile again; afterwards it works offline.".to_string()
    } else if m.contains("Missing number") {
        "LaTeX expected a number or length here (e.g. 2cm or 0.5\\textwidth). Check the arguments near the reported line.".to_string()
    } else if m.contains("Illegal unit of measure") {
        "A length is missing its unit. Write e.g. 2cm, 10pt or 0.5\\linewidth instead of a bare number.".to_string()
    } else {
        return None;
    };
    Some(hint)
}

/// Heuristic tracking of which file TeX is currently reading, from the
/// `(./chapter.tex ... )` nesting in the log. Good enough to attribute
/// warnings that only say "on input line N".
#[derive(Default)]
struct FileStack {
    stack: Vec<Option<String>>,
}

impl FileStack {
    fn current(&self) -> Option<String> {
        self.stack.iter().rev().find_map(|f| f.clone())
    }

    fn scan(&mut self, line: &str) {
        let chars: Vec<char> = line.chars().collect();
        let mut i = 0;
        while i < chars.len() {
            match chars[i] {
                '(' => {
                    let rest: String = chars[i + 1..].iter().take_while(|c| !c.is_whitespace() && **c != '(' && **c != ')').collect();
                    let is_user_file = (rest.starts_with("./") || rest.starts_with(".\\")) && looks_like_tex_path(&rest);
                    self.stack.push(if is_user_file { Some(rest.clone()) } else { None });
                    i += 1 + rest.chars().count();
                    continue;
                }
                ')' => {
                    self.stack.pop();
                }
                _ => {}
            }
            i += 1;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tectonic_located_error() {
        let out = "note: Running TeX ...\nerror: chapters/intro.tex:12: Undefined control sequence\nerror: halted on potentially-recoverable error as specified\n";
        let d = parse_tectonic(out);
        assert_eq!(d.len(), 1);
        assert_eq!(d[0].file.as_deref(), Some("chapters/intro.tex"));
        assert_eq!(d[0].line, Some(12));
        assert!(d[0].hint.as_deref().unwrap().contains("\\usepackage"));
    }

    #[test]
    fn tectonic_keeps_only_final_pass_warnings() {
        // Output shape taken from a real Tectonic run: a box that is overfull only
        // before BibTeX resolves the citations, and one that stays overfull.
        let out = "note: Running TeX ...\n\
            warning: main.tex:139: Overfull \\hbox (32.9pt too wide) in paragraph at lines 135--139\n\
            warning: main.tex:9: Overfull \\hbox (379.5pt too wide) in paragraph at lines 5--9\n\
            note: Running BibTeX on main.aux ...\n\
            note: Rerunning TeX because bibtex was run ...\n\
            warning: main.tex:9: Overfull \\hbox (379.5pt too wide) in paragraph at lines 5--9\n\
            note: Rerunning TeX because \"main.toc\" changed ...\n\
            warning: main.tex:9: Overfull \\hbox (379.07pt too wide) in paragraph at lines 5--9\n\
            note: Running xdvipdfmx ...\n";
        let d = parse_tectonic(out);
        assert_eq!(d.len(), 1, "{d:?}");
        assert_eq!(d[0].line, Some(9));
        assert!(d[0].message.contains("379.07pt"));
    }

    #[test]
    fn tectonic_missing_package() {
        let d = parse_tectonic("error: main.tex:3: LaTeX Error: File `foobar.sty' not found.\n");
        assert_eq!(d[0].severity, Severity::Error);
        assert!(d[0].hint.as_deref().unwrap().contains("“foobar.sty”"));
    }

    #[test]
    fn tex_log_error_with_context() {
        let log = "(./main.tex\n./main.tex:7: Undefined control sequence.\nl.7 \\foo\n         bar\n)";
        let d = parse_tex_log(log);
        assert_eq!(d[0].line, Some(7));
        assert_eq!(d[0].file.as_deref(), Some("main.tex"));
        assert!(d[0].message.contains("\\foo"));
    }

    #[test]
    fn tex_log_warning_in_included_file() {
        let log = "(./main.tex (./chapters/a.tex\nLaTeX Warning: Reference `fig:x' on page 1 undefined on input line 4.\n)\n";
        let d = parse_tex_log(log);
        assert_eq!(d[0].severity, Severity::Warning);
        assert_eq!(d[0].file.as_deref(), Some("chapters/a.tex"));
        assert_eq!(d[0].line, Some(4));
        assert_eq!(d[0].message, "Reference `fig:x' on page 1 undefined");
    }

    #[test]
    fn package_warning_continuation() {
        let log = "(./main.tex\nPackage hyperref Warning: Token not allowed in a PDF string\n(hyperref)                removing `math shift' on input line 9.\n)";
        let d = parse_tex_log(log);
        assert_eq!(d[0].source, "hyperref");
        assert_eq!(d[0].line, Some(9));
        assert!(d[0].message.contains("removing"));
    }

    #[test]
    fn package_error_source() {
        let log = "(./main.tex\n./main.tex:5: Package babel Error: Unknown option `klingon'.\n)";
        let d = parse_tex_log(log);
        assert_eq!(d[0].source, "babel");
    }

    #[test]
    fn windows_paths() {
        let (f, l, m) = split_file_line("C:/Users/n/thesis.tex:42: Missing $ inserted").unwrap();
        assert_eq!((f.as_str(), l, m), ("C:/Users/n/thesis.tex", 42, "Missing $ inserted"));
        assert!(split_file_line("note: 12: nothing").is_none());
    }

    #[test]
    fn tectonic_offline() {
        let d = parse_tectonic("error: failed to fetch https://relay.fullyjustified.net/default_bundle.tar\n    caused by: error sending request for url\nerror: error sending request for url (https://relay.fullyjustified.net/): dns error\n");
        assert!(d.iter().any(|d| d.hint.as_deref().is_some_and(|h| h.contains("internet connection"))));
    }

    #[test]
    fn bib_logs() {
        let d = parse_bib_log("Warning--I didn't find a database entry for \"nokey\"\n", false);
        assert_eq!(d[0].source, "BibTeX");
        assert!(d[0].hint.as_deref().unwrap().contains("“nokey”"));
        let d = parse_bib_log("INFO - This is Biber 2.19\n[12] Utils.pm:410> WARN - I didn't find a database entry for 'x' (section 0)\n[13] ERROR - Cannot find 'refs.bib'!\n", true);
        assert_eq!(d[0].severity, Severity::Error);
        assert_eq!(d[0].source, "Biber");
        assert!(d[0].hint.as_deref().unwrap().contains(".bib file couldn't be found"));
    }

    #[test]
    fn missing_package_without_noise() {
        // Real pdfLaTeX log excerpt (-file-line-error).
        let log = "(./main.tex\nLaTeX2e <2023-11-01>\n! LaTeX Error: File `nonexistentpkg.sty' not found.\n\nType X to quit or <RETURN> to proceed,\nor enter new name. (Default extension: sty)\n\nEnter file name: \n./main.tex:3: Emergency stop.\n<read *> \n         \nl.3 \\begin\n             {document}\n*** (cannot \\read from terminal in nonstop modes)\n\n./main.tex:3:  ==> Fatal error occurred, no output PDF file produced!\n";
        let d = parse_tex_log(log);
        assert_eq!(d.len(), 1, "{d:?}");
        assert!(d[0].message.contains("nonexistentpkg.sty"));
        assert_eq!(d[0].file.as_deref(), Some("main.tex"));
        assert_eq!(d[0].line, Some(3));
    }

    #[test]
    fn overfull_box() {
        let d = parse_tex_log("(./main.tex\nOverfull \\hbox (12.0pt too wide) in paragraph at lines 20--22\n)");
        assert_eq!(d[0].line, Some(20));
        assert_eq!(d[0].severity, Severity::Warning);
        assert!(d[0].hint.is_some());
    }

    #[test]
    fn biblatex_bibtex_fallback_is_only_a_note() {
        let log = "Package biblatex Warning: Using fall-back BibTeX(8) backend:
(biblatex)                functionality may be reduced/unavailable.
";
        let d = parse_tex_log(log);
        assert_eq!(d.len(), 1, "{d:?}");
        assert_eq!(d[0].severity, Severity::Info);
        assert!(d[0].hint.as_deref().is_some_and(|h| h.contains("Biber")), "{d:?}");
    }

    #[test]
    fn missing_package_points_at_usepackage() {
        // Same real log as above: TeX reports l.3, the package is loaded on line 2.
        let log = "(./main.tex\nLaTeX2e <2023-11-01>\n! LaTeX Error: File `nonexistentpkg.sty' not found.\n\nType X to quit or <RETURN> to proceed,\nor enter new name. (Default extension: sty)\n\nEnter file name: \n./main.tex:3: Emergency stop.\n<read *> \n         \nl.3 \\begin\n             {document}\n*** (cannot \\read from terminal in nonstop modes)\n\n./main.tex:3:  ==> Fatal error occurred, no output PDF file produced!\n";
        let mut d = parse_tex_log(log);
        let src = "\\documentclass{article}\n\\usepackage{nonexistentpkg}\n\\begin{document}\nHi\n\\end{document}\n";
        locate_missing_files(&mut d, |f| (f == "main.tex").then(|| src.to_string()));
        assert_eq!((d[0].line, d[0].column, d[0].end_column), (Some(2), Some(13), Some(27)), "{d:?}");
    }

    #[test]
    fn missing_package_tectonic_and_lists() {
        let mut d = parse_tectonic("error: main.tex:5: LaTeX Error: File `foobar.sty' not found.\n");
        let src = "\\documentclass{article}\n% \\usepackage{foobar}\n\\usepackage[utf8]{inputenc}\n\\usepackage[a, b=[c]]{amsmath,\n  foobar , graphicx}\n\\begin{document}\n";
        locate_missing_files(&mut d, |_| Some(src.to_string()));
        assert_eq!((d[0].line, d[0].column, d[0].end_column), (Some(5), Some(3), Some(9)), "{d:?}");
    }

    #[test]
    fn missing_class_and_fallback() {
        let mut d = parse_tectonic("error: main.tex:1: LaTeX Error: File `fancyclass.cls' not found.\n");
        locate_missing_files(&mut d, |_| Some("ä \\documentclass[12pt]{fancyclass}\n".into()));
        assert_eq!((d[0].line, d[0].column), (Some(1), Some(24)));
        // Not loaded in that file (e.g. a nested \input): TeX's line stays.
        let mut d = parse_tectonic("error: main.tex:7: LaTeX Error: File `other.sty' not found.\n");
        locate_missing_files(&mut d, |_| Some("\\usepackage{otherx}\n\\usepackagex{other}\n".into()));
        assert_eq!((d[0].line, d[0].column), (Some(7), None));
    }
}
