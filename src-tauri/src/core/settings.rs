//! Per-project settings stored in `<project>/.kerntex/project.json`, plus
//! detection of the hints a project already carries (`% !TEX root`,
//! `% !TEX program`, a `latexmkrc`).

use super::compile::{Engine, TexFlavor};
use super::error::{AppError, ErrorSource, Result};
use super::project::{self, BUILD_DIR};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;

pub const SETTINGS_FILE: &str = "project.json";
pub const SETTINGS_VERSION: u32 = 1;

/// Everything is optional: a missing field means "use the app default /
/// auto-detect", so older or hand-written files keep working.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ProjectSettings {
    #[serde(default = "version")]
    pub version: u32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub main_file: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub engine: Option<Engine>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub flavor: Option<TexFlavor>,
}

fn version() -> u32 {
    SETTINGS_VERSION
}

pub fn load(root: &Path) -> ProjectSettings {
    let path = root.join(BUILD_DIR).join(SETTINGS_FILE);
    fs::read_to_string(path).ok().and_then(|s| serde_json::from_str(&s).ok()).unwrap_or_default()
}

pub fn save(root: &Path, settings: &ProjectSettings) -> Result<()> {
    let mut s = settings.clone();
    s.version = SETTINGS_VERSION;
    if let Some(main) = &s.main_file {
        // Validate: must stay inside the project.
        project::resolve(root, main)?;
    }
    let json = serde_json::to_string_pretty(&s)
        .map_err(|e| AppError::new(ErrorSource::App, "Couldn't serialize project settings").detail(e.to_string()))?;
    project::write_text(root, &format!("{BUILD_DIR}/{SETTINGS_FILE}"), &(json + "\n"))
}

/// What the project's own files suggest about how to build it.
#[derive(Debug, Clone, Default, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct BuildHints {
    /// `% !TEX program = xelatex` in the main file.
    pub flavor: Option<TexFlavor>,
    /// A `latexmkrc` / `.latexmkrc` exists in the project root.
    pub has_latexmkrc: bool,
}

pub fn build_hints(root: &Path, main_file: Option<&str>) -> BuildHints {
    let has_latexmkrc = ["latexmkrc", ".latexmkrc"].iter().any(|n| root.join(n).is_file());
    let flavor = main_file.and_then(|m| project::read_text(root, m).ok()).and_then(|text| magic_comment(&text, "program")).and_then(|p| {
        match p.to_lowercase().as_str() {
            "pdflatex" | "latex" => Some(TexFlavor::Pdflatex),
            "xelatex" | "xetex" => Some(TexFlavor::Xelatex),
            "lualatex" | "luatex" => Some(TexFlavor::Lualatex),
            _ => None,
        }
    });
    BuildHints { flavor, has_latexmkrc }
}

/// Reads `% !TEX <key> = value` (also `%!TeX`) from the first lines of a file.
pub fn magic_comment(text: &str, key: &str) -> Option<String> {
    for line in text.lines().take(30) {
        let t = line.trim_start();
        let Some(rest) = t.strip_prefix('%') else {
            if t.is_empty() {
                continue;
            }
            break; // magic comments must come before any code
        };
        let rest = rest.trim_start();
        if rest.len() < 4 || !rest[..4].eq_ignore_ascii_case("!tex") {
            continue;
        }
        let rest = rest[4..].trim_start();
        let Some((k, v)) = rest.split_once('=') else { continue };
        let k = k.trim();
        // Accept "TS-program" as alias for "program".
        if k.eq_ignore_ascii_case(key) || (key == "program" && k.eq_ignore_ascii_case("ts-program")) {
            return Some(v.trim().to_string());
        }
    }
    None
}

/// Follows `% !TEX root = ../main.tex` from `rel` to the document that
/// should actually be compiled. Returns a project-relative path.
pub fn tex_root(root: &Path, rel: &str) -> Option<String> {
    let text = project::read_text(root, rel).ok()?;
    let target = magic_comment(&text, "root")?;
    let base = match rel.rfind('/') {
        Some(i) => &rel[..i],
        None => "",
    };
    let joined = if base.is_empty() { target.replace('\\', "/") } else { format!("{base}/{}", target.replace('\\', "/")) };
    let normalized = normalize(&joined)?;
    project::resolve(root, &normalized).ok()?.is_file().then_some(normalized)
}

/// Collapses `a/b/../c` → `a/c`; None if it would leave the project.
fn normalize(path: &str) -> Option<String> {
    let mut parts: Vec<&str> = Vec::new();
    for p in path.split('/') {
        match p {
            "" | "." => {}
            ".." => {
                parts.pop()?;
            }
            other => parts.push(other),
        }
    }
    Some(parts.join("/"))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tmp(tag: &str) -> std::path::PathBuf {
        let d = std::env::temp_dir().join(format!("kerntex-settings-{tag}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&d);
        fs::create_dir_all(&d).unwrap();
        d
    }

    #[test]
    fn magic_comments() {
        let t = "% !TEX program = xelatex\n%!TeX root = ../main.tex\n\\documentclass{article}\n% !TEX spellcheck = en";
        assert_eq!(magic_comment(t, "program").as_deref(), Some("xelatex"));
        assert_eq!(magic_comment(t, "root").as_deref(), Some("../main.tex"));
        assert_eq!(magic_comment(t, "spellcheck"), None, "only before code");
        assert_eq!(magic_comment("% !TEX TS-program = lualatex", "program").as_deref(), Some("lualatex"));
    }

    #[test]
    fn follows_tex_root() {
        let root = tmp("root");
        project::write_text(&root, "main.tex", "\\documentclass{article}").unwrap();
        project::write_text(&root, "chapters/a.tex", "% !TEX root = ../main.tex\n\\section{A}").unwrap();
        project::write_text(&root, "chapters/b.tex", "% !TEX root = ../../outside.tex\n").unwrap();
        assert_eq!(tex_root(&root, "chapters/a.tex").as_deref(), Some("main.tex"));
        assert_eq!(tex_root(&root, "chapters/b.tex"), None);
        assert_eq!(tex_root(&root, "main.tex"), None);
        fs::remove_dir_all(root).ok();
    }

    #[test]
    fn settings_roundtrip_and_hints() {
        let root = tmp("rt");
        assert_eq!(load(&root), ProjectSettings::default());
        let s = ProjectSettings {
            version: 0,
            main_file: Some("thesis.tex".into()),
            engine: Some(Engine::Latexmk),
            flavor: Some(TexFlavor::Lualatex),
        };
        save(&root, &s).unwrap();
        let loaded = load(&root);
        assert_eq!(loaded.version, SETTINGS_VERSION);
        assert_eq!(loaded.main_file.as_deref(), Some("thesis.tex"));
        assert_eq!(loaded.engine, Some(Engine::Latexmk));
        assert!(save(&root, &ProjectSettings { main_file: Some("../x.tex".into()), ..Default::default() }).is_err());

        project::write_text(&root, "thesis.tex", "% !TEX program = lualatex\n\\documentclass{report}").unwrap();
        project::write_text(&root, "latexmkrc", "$pdf_mode = 1;").unwrap();
        let h = build_hints(&root, Some("thesis.tex"));
        assert_eq!(h.flavor, Some(TexFlavor::Lualatex));
        assert!(h.has_latexmkrc);
        fs::remove_dir_all(root).ok();
    }
}
