//! Project templates for "New project" and the "Welcome to KernTeX" example.

use super::error::{AppError, ErrorSource, Result};
use super::project::{self, BUILD_DIR};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum Template {
    Blank,
    Article,
    Report,
    Beamer,
    Letter,
}

impl Template {
    fn main_tex(self) -> &'static str {
        match self {
            Template::Blank => include_str!("../../templates/blank.tex"),
            Template::Article => include_str!("../../templates/article.tex"),
            Template::Report => include_str!("../../templates/report.tex"),
            Template::Beamer => include_str!("../../templates/beamer.tex"),
            Template::Letter => include_str!("../../templates/letter.tex"),
        }
    }
}

pub const WELCOME_NAME: &str = "Welcome to KernTeX";

const WELCOME_FILES: &[(&str, &[u8])] = &[
    ("main.tex", include_bytes!("../../templates/welcome/main.tex")),
    ("references.bib", include_bytes!("../../templates/welcome/references.bib")),
    ("images/example.png", include_bytes!("../../templates/welcome/images/example.png")),
];

fn gitignore() -> String {
    format!("{BUILD_DIR}/\n")
}

fn check_name(name: &str) -> Result<&str> {
    let name = name.trim();
    if name.is_empty() || name == "." || name == ".." || name.contains(['/', '\\', ':', '*', '?', '"', '<', '>', '|']) {
        return Err(AppError::new(ErrorSource::App, "Invalid project name")
            .hint("Use letters, numbers, spaces, dashes or underscores — no slashes or special characters like : * ? \" < > |."));
    }
    Ok(name)
}

/// Creates `parent/name/main.tex` from a template; returns the new project root.
pub fn new_project(parent: &Path, name: &str, template: Template) -> Result<PathBuf> {
    let name = check_name(name)?;
    let root = parent.join(name);
    if root.exists() {
        return Err(AppError::io("create the project", &root, std::io::ErrorKind::AlreadyExists.into()));
    }
    fs::create_dir_all(&root).map_err(|e| AppError::io("create the project", &root, e))?;
    project::write_text(&root, "main.tex", template.main_tex())?;
    project::write_text(&root, ".gitignore", &gitignore())?;
    Ok(root)
}

/// Writes the example project into `parent/Welcome to KernTeX`. An existing
/// copy is left untouched (the user may have edited it) and simply returned;
/// with `fresh`, a new copy is created next to it instead (`… 2`).
pub fn ensure_welcome(parent: &Path, fresh: bool) -> Result<PathBuf> {
    let mut root = parent.join(WELCOME_NAME);
    if root.join("main.tex").is_file() {
        if !fresh {
            return Ok(root);
        }
        let mut n = 2;
        while root.exists() {
            root = parent.join(format!("{WELCOME_NAME} {n}"));
            n += 1;
        }
    }
    for (rel, bytes) in WELCOME_FILES {
        let abs = project::resolve(&root, rel)?;
        if let Some(p) = abs.parent() {
            fs::create_dir_all(p).map_err(|e| AppError::io("create", p, e))?;
        }
        if !abs.exists() {
            fs::write(&abs, bytes).map_err(|e| AppError::io("write", &abs, e))?;
        }
    }
    let gi = root.join(".gitignore");
    if !gi.exists() {
        project::write_text(&root, ".gitignore", &gitignore())?;
    }
    Ok(root)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn creates_projects_and_welcome() {
        let base = std::env::temp_dir().join(format!("kerntex-tpl-{}", std::process::id()));
        let _ = fs::remove_dir_all(&base);
        fs::create_dir_all(&base).unwrap();

        let p = new_project(&base, "Thesis", Template::Report).unwrap();
        assert!(fs::read_to_string(p.join("main.tex")).unwrap().contains("{report}"));
        assert_eq!(fs::read_to_string(p.join(".gitignore")).unwrap(), ".kerntex/\n");
        assert!(new_project(&base, "Thesis", Template::Blank).is_err(), "never overwrite");
        assert!(new_project(&base, "a/b", Template::Blank).is_err());
        assert!(new_project(&base, "..", Template::Blank).is_err());

        let w = ensure_welcome(&base, false).unwrap();
        assert!(w.join("images/example.png").is_file());
        fs::write(w.join("main.tex"), "edited").unwrap();
        assert_eq!(ensure_welcome(&base, false).unwrap(), w);
        assert_eq!(fs::read_to_string(w.join("main.tex")).unwrap(), "edited", "keeps user edits");
        let w2 = ensure_welcome(&base, true).unwrap();
        assert_eq!(w2.file_name().unwrap(), "Welcome to KernTeX 2");
        fs::remove_dir_all(base).ok();
    }
}
