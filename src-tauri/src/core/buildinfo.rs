//! "Build details" for the curious: which TeX program ran, which document
//! class and packages (with versions) the document loaded. Parsed from the
//! TeX `.log`, which every engine writes; nothing extra is logged.

use serde::Serialize;

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct PackageRef {
    pub name: String,
    /// Release date as printed, e.g. `2023/05/13`.
    pub date: Option<String>,
    /// e.g. `v2.17o`.
    pub version: Option<String>,
    pub description: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct BuildDetails {
    /// First log line, e.g. `This is XeTeX, Version 3.141592653-2.6-0.999995 (Tectonic)`.
    pub program: Option<String>,
    /// e.g. `LaTeX2e <2023-11-01> patch level 1`.
    pub format: Option<String>,
    pub document_class: Option<PackageRef>,
    /// In load order, without duplicates.
    pub packages: Vec<PackageRef>,
    /// TeX runs needed (cross-references, bibliography).
    pub passes: u32,
    /// Engine binary that was used.
    pub engine_path: Option<String>,
    /// Build output folder (PDF, log, aux files).
    pub out_dir: Option<String>,
    /// The full `.log`, if one was written.
    pub log_path: Option<String>,
}

/// Fills the log-derived fields of `BuildDetails`.
pub fn parse_log(log: &str) -> BuildDetails {
    let mut d = BuildDetails::default();
    for line in log.lines() {
        let line = line.trim_end();
        if d.program.is_none() && line.starts_with("This is ") {
            // Drop the trailing "(preloaded format=…) <date>" noise.
            let p = line.split("  ").next().unwrap_or(line);
            let p = p.split(" (preloaded").next().unwrap_or(p);
            d.program = Some(p.trim().to_string());
        } else if d.format.is_none() && (line.starts_with("LaTeX2e <") || line.starts_with("LaTeX3 ")) {
            d.format = Some(line.to_string());
        } else if let Some(rest) = line.strip_prefix("Document Class: ") {
            if d.document_class.is_none() {
                d.document_class = parse_ref(rest);
            }
        } else if let Some(rest) = line.strip_prefix("Package: ") {
            if let Some(p) = parse_ref(rest) {
                if !d.packages.iter().any(|q| q.name == p.name) {
                    d.packages.push(p);
                }
            }
        }
    }
    d
}

/// `amsmath 2023/05/13 v2.17o AMS math features` → parts. Date, version and
/// description are optional; packages are inconsistent about them.
fn parse_ref(s: &str) -> Option<PackageRef> {
    let mut words = s.split_whitespace().peekable();
    let name = words.next()?.to_string();
    if !name.chars().all(|c| c.is_ascii_alphanumeric() || "-_.@".contains(c)) {
        return None;
    }
    let date = words.next_if(|w| is_date(w)).map(str::to_string);
    let version = words.next_if(|w| w.starts_with('v') && w[1..].starts_with(|c: char| c.is_ascii_digit())).map(str::to_string);
    let rest: Vec<&str> = words.collect();
    let description = (!rest.is_empty()).then(|| rest.join(" "));
    Some(PackageRef { name, date, version, description })
}

fn is_date(w: &str) -> bool {
    let b = w.as_bytes();
    b.len() == 10 && b[..4].iter().all(u8::is_ascii_digit) && matches!(b[4], b'/' | b'-') && matches!(b[7], b'/' | b'-')
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_real_log_excerpt() {
        let log = "This is pdfTeX, Version 3.141592653-2.6-1.40.25 (TeX Live 2023/Debian) (preloaded format=pdflatex 2024.1.1)  22 NOV 2026 20:30\n\
entering extended mode\n\
LaTeX2e <2023-11-01> patch level 1\n\
L3 programming layer <2024-01-22>\n\
(/usr/share/texlive/texmf-dist/tex/latex/base/article.cls\n\
Document Class: article 2023/05/17 v1.4n Standard LaTeX document class\n\
(/usr/share/texlive/texmf-dist/tex/latex/amsmath/amsmath.sty\n\
Package: amsmath 2023/05/13 v2.17o AMS math features\n\
Package: hyperref 2023-11-26 v7.01g Hypertext links for LaTeX\n\
Package: iftex 2022/02/03 v1.0f TeX engine tests\n\
Package: amsmath 2023/05/13 v2.17o AMS math features\n\
Package: lmodern\n\
Package hyperref Info: Hyper figures OFF on input line 4.\n";
        let d = parse_log(log);
        assert_eq!(d.program.as_deref(), Some("This is pdfTeX, Version 3.141592653-2.6-1.40.25 (TeX Live 2023/Debian)"));
        assert_eq!(d.format.as_deref(), Some("LaTeX2e <2023-11-01> patch level 1"));
        let cls = d.document_class.unwrap();
        assert_eq!((cls.name.as_str(), cls.version.as_deref()), ("article", Some("v1.4n")));
        let names: Vec<_> = d.packages.iter().map(|p| p.name.as_str()).collect();
        assert_eq!(names, ["amsmath", "hyperref", "iftex", "lmodern"]);
        assert_eq!(d.packages[1].date.as_deref(), Some("2023-11-26"));
        assert_eq!(d.packages[0].description.as_deref(), Some("AMS math features"));
        assert_eq!(d.packages[3], PackageRef { name: "lmodern".into(), date: None, version: None, description: None });
    }
}
