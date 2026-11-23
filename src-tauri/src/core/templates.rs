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
    TypstBlank,
    TypstArticle,
    TypstReport,
    TypstSlides,
    TypstLetter,
}

impl Template {
    fn main_tex(self) -> &'static str {
        match self {
            Template::Blank => include_str!("../../templates/blank.tex"),
            Template::Article => include_str!("../../templates/article.tex"),
            Template::Report => include_str!("../../templates/report.tex"),
            Template::Beamer => include_str!("../../templates/beamer.tex"),
            Template::Letter => include_str!("../../templates/letter.tex"),
            Template::TypstBlank => include_str!("../../templates/typst/blank.typ"),
            Template::TypstArticle => include_str!("../../templates/typst/article.typ"),
            Template::TypstReport => include_str!("../../templates/typst/report.typ"),
            Template::TypstSlides => include_str!("../../templates/typst/slides.typ"),
            Template::TypstLetter => include_str!("../../templates/typst/letter.typ"),
        }
    }

    fn main_name(self) -> &'static str {
        match self {
            Template::TypstBlank | Template::TypstArticle | Template::TypstReport | Template::TypstSlides | Template::TypstLetter => {
                "main.typ"
            }
            _ => "main.tex",
        }
    }
}

pub const WELCOME_NAME: &str = "Welcome to KernTeX";
pub const TYPST_WELCOME_NAME: &str = "Welcome to Typst";

const TYPST_WELCOME_FILES: &[(&str, &[u8])] = &[
    ("main.typ", include_bytes!("../../templates/typst-welcome/main.typ")),
    ("references.bib", include_bytes!("../../templates/typst-welcome/references.bib")),
    ("images/example.png", include_bytes!("../../templates/welcome/images/example.png")),
];

const WELCOME_FILES: &[(&str, &[u8])] = &[
    ("main.tex", include_bytes!("../../templates/welcome/main.tex")),
    ("references.bib", include_bytes!("../../templates/welcome/references.bib")),
    ("images/example.png", include_bytes!("../../templates/welcome/images/example.png")),
];

/// Only build output is ignored; `.kerntex/project.json` (main file, engine)
/// is meant to be committed so it travels with the project.
fn gitignore() -> String {
    format!("{BUILD_DIR}/out/\n")
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
    project::write_text(&root, template.main_name(), template.main_tex())?;
    project::write_text(&root, ".gitignore", &gitignore())?;
    Ok(root)
}

/// Writes the example project into `parent/Welcome to KernTeX`. An existing
/// copy is left untouched (the user may have edited it) and simply returned;
/// with `fresh`, a new copy is created next to it instead (`… 2`).
pub fn ensure_welcome(parent: &Path, fresh: bool) -> Result<PathBuf> {
    write_example(parent, WELCOME_NAME, "main.tex", WELCOME_FILES, fresh)
}

/// Same as [`ensure_welcome`], for the Typst tour.
pub fn ensure_typst_welcome(parent: &Path, fresh: bool) -> Result<PathBuf> {
    write_example(parent, TYPST_WELCOME_NAME, "main.typ", TYPST_WELCOME_FILES, fresh)
}

fn write_example(parent: &Path, name: &str, main: &str, files: &[(&str, &[u8])], fresh: bool) -> Result<PathBuf> {
    let mut root = parent.join(name);
    if root.join(main).is_file() {
        if !fresh {
            return Ok(root);
        }
        let mut n = 2;
        while root.exists() {
            root = parent.join(format!("{name} {n}"));
            n += 1;
        }
    }
    for (rel, bytes) in files {
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
        assert_eq!(fs::read_to_string(p.join(".gitignore")).unwrap(), ".kerntex/out/\n");
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

    /// Every Typst template and the Typst tour must compile without errors
    /// (no network: they use no packages).
    #[test]
    fn typst_templates_compile() {
        use crate::core::compile::CancelHandle;
        use crate::core::diagnostics::Severity;
        let base = std::env::temp_dir().join(format!("kerntex-tpl-typst-{}", std::process::id()));
        let _ = fs::remove_dir_all(&base);
        fs::create_dir_all(&base).unwrap();
        let mut roots = vec![ensure_typst_welcome(&base, false).unwrap()];
        for (i, t) in [Template::TypstBlank, Template::TypstArticle, Template::TypstReport, Template::TypstSlides, Template::TypstLetter]
            .into_iter()
            .enumerate()
        {
            roots.push(new_project(&base, &format!("t{i}"), t).unwrap());
        }
        for root in roots {
            let res = crate::core::typst_engine::compile(&root, "main.typ", &CancelHandle::default(), |_| {}).unwrap();
            let problems: Vec<_> = res.diagnostics.iter().filter(|d| d.severity != Severity::Info).collect();
            assert!(res.ok && res.pdf_path.is_some(), "{}: {problems:?}", root.display());
            assert!(problems.is_empty(), "{}: {problems:?}", root.display());
            assert_eq!(crate::core::project::open(&root).unwrap().main_file.as_deref(), Some("main.typ"));
        }
        fs::remove_dir_all(base).ok();
    }
}
