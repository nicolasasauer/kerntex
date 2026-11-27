//! Structured errors. Every error that reaches the UI says *where* it came
//! from (`source`), *what* happened (`title`, `detail`) and *how to fix it*
//! (`hint`).

use serde::Serialize;
use std::io;
use std::path::Path;

#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum ErrorSource {
    /// Reading or writing files on disk.
    FileSystem,
    /// The TeX engine (Tectonic / latexmk) could not be started or failed.
    Engine,
    /// Importing or exporting ZIP archives.
    Archive,
    /// The system `git` (version control).
    Git,
    /// A bug or invalid request inside KernTeX itself.
    App,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AppError {
    pub source: ErrorSource,
    pub title: String,
    pub detail: Option<String>,
    pub hint: Option<String>,
    /// Machine-readable reason the UI reacts to, e.g. `changedOnDisk`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub code: Option<&'static str>,
}

pub type Result<T> = std::result::Result<T, AppError>;

impl AppError {
    pub fn new(source: ErrorSource, title: impl Into<String>) -> Self {
        Self { source, title: title.into(), detail: None, hint: None, code: None }
    }

    pub fn detail(mut self, detail: impl Into<String>) -> Self {
        self.detail = Some(detail.into());
        self
    }

    pub fn hint(mut self, hint: impl Into<String>) -> Self {
        self.hint = Some(hint.into());
        self
    }

    pub fn code(mut self, code: &'static str) -> Self {
        self.code = Some(code);
        self
    }

    /// Wraps an I/O error with a human-readable action ("save", "open" ...)
    /// and a fix suggestion derived from the error kind.
    pub fn io(action: &str, path: &Path, err: io::Error) -> Self {
        let hint = match err.kind() {
            io::ErrorKind::NotFound => {
                "The file or folder doesn't exist (anymore). It may have been moved, renamed or deleted outside KernTeX — refresh the file tree."
            }
            io::ErrorKind::PermissionDenied => {
                "Your user account isn't allowed to access this location. Check the folder permissions, or close other programs (e.g. a PDF viewer) that may lock the file."
            }
            io::ErrorKind::AlreadyExists => "Something with this name already exists. Choose a different name.",
            io::ErrorKind::InvalidData => "The file isn't valid UTF-8 text. KernTeX can only edit text files; binary files (images, PDFs) can be used from LaTeX but not edited.",
            _ => "Check that the drive is available and has free space, then try again.",
        };
        AppError::new(ErrorSource::FileSystem, format!("Couldn't {action} “{}”", display_name(path)))
            .detail(format!("{err} ({})", path.display()))
            .hint(hint)
    }
}

fn display_name(path: &Path) -> String {
    path.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_else(|| path.display().to_string())
}

impl std::fmt::Display for AppError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "[{:?}] {}", self.source, self.title)?;
        if let Some(d) = &self.detail {
            write!(f, ": {d}")?;
        }
        Ok(())
    }
}

impl std::error::Error for AppError {}
