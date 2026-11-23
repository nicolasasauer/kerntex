//! Typst support: the Typst compiler runs in-process (it's a Rust library),
//! so `.typ` projects need nothing installed. Packages from Typst Universe
//! (`#import "@preview/…"`) are downloaded on first use and cached.

use super::buildinfo::{BuildDetails, PackageRef};
use super::compile::{CancelHandle, CompileResult, Progress};
use super::diagnostics::{Diagnostic, Severity};
use super::error::{AppError, ErrorSource, Result};
use super::project::{self, BUILD_DIR};
use std::any::Any;
use std::io::{self, Read};
use std::path::{Path, PathBuf};
use std::sync::atomic::Ordering;
use std::sync::mpsc::{self, Sender};
use std::sync::{Mutex, OnceLock};
use std::time::{Duration, Instant};
use typst::diag::{FileResult, Severity as TypstSeverity, SourceDiagnostic};
use typst::foundations::{Bytes, Datetime, Duration as TypstDuration};
use typst::syntax::{FileId, RootedPath, Source, VirtualPath, VirtualRoot};
use typst::text::{Font, FontBook};
use typst::utils::LazyHash;
use typst::{Library, LibraryExt, World, WorldExt};
use typst_kit::datetime::Time;
use typst_kit::downloader::{Downloader, SystemDownloader};
use typst_kit::files::{FileStore, FsRoot, SystemFiles};
use typst_kit::fonts::{self, FontStore};
use typst_kit::packages::SystemPackages;
use typst_layout::PagedDocument;

/// Version of the embedded compiler (keep in sync with Cargo.toml).
pub const TYPST_VERSION: &str = "0.15.1";

pub fn is_typst(path: &str) -> bool {
    path.to_ascii_lowercase().ends_with(".typ")
}

/// Fonts are expensive to discover, so they're scanned once per process:
/// Typst's bundled fonts plus the fonts installed on the system.
fn fonts() -> &'static FontStore {
    static FONTS: OnceLock<FontStore> = OnceLock::new();
    FONTS.get_or_init(|| {
        let mut store = FontStore::new();
        store.extend(fonts::embedded());
        store.extend(fonts::system());
        store
    })
}

/// Forwards package downloads to the progress display.
struct ReportingDownloader {
    inner: SystemDownloader,
    tx: Mutex<Sender<String>>,
}

impl Downloader for ReportingDownloader {
    fn stream(&self, key: &dyn Any, url: &str) -> io::Result<(Option<usize>, Box<dyn Read>)> {
        let name = url.rsplit('/').next().unwrap_or(url).trim_end_matches(".tar.gz").to_string();
        let _ = self.tx.lock().unwrap().send(name);
        self.inner.stream(key, url)
    }
}

struct KernWorld {
    library: LazyHash<Library>,
    files: FileStore<SystemFiles>,
    main: FileId,
    time: Time,
}

impl World for KernWorld {
    fn library(&self) -> &LazyHash<Library> {
        &self.library
    }
    fn book(&self) -> &LazyHash<FontBook> {
        fonts().book()
    }
    fn main(&self) -> FileId {
        self.main
    }
    fn source(&self, id: FileId) -> FileResult<Source> {
        self.files.source(id)
    }
    fn file(&self, id: FileId) -> FileResult<Bytes> {
        self.files.file(id)
    }
    fn font(&self, index: usize) -> Option<Font> {
        fonts().font(index)
    }
    fn today(&self, offset: Option<TypstDuration>) -> Option<Datetime> {
        self.time.today(offset)
    }
}

fn project_file(rel: &str) -> Result<FileId> {
    let vpath = VirtualPath::new(rel)
        .map_err(|e| AppError::new(ErrorSource::App, format!("Invalid file path “{rel}”")).detail(format!("{e:?}")))?;
    Ok(RootedPath::new(VirtualRoot::Project, vpath).intern())
}

/// Compiles `main_file` (project-relative `.typ`) to `<root>/.kerntex/out/<stem>.pdf`.
pub fn compile(root: &Path, main_file: &str, cancel: &CancelHandle, mut on_progress: impl FnMut(Progress) + Send) -> Result<CompileResult> {
    let main_abs = project::resolve(root, main_file)?;
    if !main_abs.is_file() {
        return Err(AppError::new(ErrorSource::App, format!("Main file “{main_file}” doesn't exist"))
            .hint("Pick the root document in the file tree (right-click → “Set as main file”)."));
    }
    let out_dir = root.join(BUILD_DIR).join("out");
    std::fs::create_dir_all(&out_dir).map_err(|e| AppError::io("create the build folder", &out_dir, e))?;
    let stem = main_abs.file_stem().map(|s| s.to_string_lossy().into_owned()).unwrap_or_else(|| "main".into());
    let pdf_path = out_dir.join(format!("{stem}.pdf"));
    let label = format!("Typst {TYPST_VERSION}");
    let start = Instant::now();
    let elapsed = || start.elapsed().as_millis() as u64;

    on_progress(Progress { stage: "Typesetting".into(), detail: Some(label.clone()), elapsed_ms: 0 });

    // Compile on a worker thread so progress (package downloads) and
    // cancellation stay responsive.
    let (tx, rx) = mpsc::channel::<String>();
    let main = project_file(main_file)?;
    let root_buf: PathBuf = root.to_path_buf();
    let downloader =
        ReportingDownloader { inner: SystemDownloader::new(format!("kerntex/{}", env!("CARGO_PKG_VERSION"))), tx: Mutex::new(tx) };
    cancel.begin_in_process();
    let worker = std::thread::spawn(move || {
        let mut world = KernWorld {
            library: LazyHash::new(Library::default()),
            files: FileStore::new(SystemFiles::new(FsRoot::new(root_buf), SystemPackages::new(downloader))),
            main,
            time: Time::system(),
        };
        let warned = typst::compile::<PagedDocument>(&world);
        let mut diags: Vec<SourceDiagnostic> = warned.warnings.to_vec();
        let pdf = match warned.output {
            Ok(doc) => match typst_pdf::pdf(&doc, &typst_pdf::PdfOptions::default()) {
                Ok(bytes) => Some(bytes),
                Err(errs) => {
                    diags.extend(errs.iter().cloned());
                    None
                }
            },
            Err(errs) => {
                diags.extend(errs.iter().cloned());
                None
            }
        };
        let converted: Vec<Diagnostic> = diags.iter().map(|d| convert(&world, d)).collect();
        let packages = used_packages(&mut world);
        (pdf, converted, packages)
    });

    // Wait, relaying download progress; stop early if cancelled.
    let mut downloads = 0u32;
    loop {
        while let Ok(name) = rx.try_recv() {
            downloads += 1;
            on_progress(Progress { stage: format!("Downloading packages ({downloads})"), detail: Some(name), elapsed_ms: elapsed() });
        }
        if worker.is_finished() {
            break;
        }
        if cancel.take_in_process_cancel() {
            // Typst can't be interrupted; let the worker finish in the background
            // and discard its result.
            return Ok(CompileResult {
                ok: false,
                canceled: true,
                pdf_path: existing(&pdf_path),
                diagnostics: vec![],
                duration_ms: elapsed(),
                log: String::new(),
                engine_label: label,
                details: BuildDetails::default(),
            });
        }
        std::thread::sleep(Duration::from_millis(20));
    }
    cancel.end_in_process();
    let (pdf, mut diagnostics, packages) = worker.join().map_err(|_| {
        AppError::new(ErrorSource::Engine, "The Typst compiler crashed")
            .hint("Please report this with the document that caused it (Help → Report a problem).")
    })?;

    if let Some(bytes) = &pdf {
        on_progress(Progress { stage: "Writing PDF".into(), detail: None, elapsed_ms: elapsed() });
        // Atomic write so the preview never sees a half-written file.
        let tmp = out_dir.join(format!("{stem}.pdf.kerntex-tmp"));
        std::fs::write(&tmp, bytes)
            .and_then(|_| std::fs::rename(&tmp, &pdf_path))
            .map_err(|e| AppError::io("write the PDF", &pdf_path, e))?;
    }
    diagnostics.sort_by_key(|d| d.severity);
    let has_errors = diagnostics.iter().any(|d| d.severity == Severity::Error);
    let log = diagnostics.iter().map(log_line).collect::<Vec<_>>().join("\n");

    Ok(CompileResult {
        ok: pdf.is_some() && !has_errors,
        canceled: false,
        pdf_path: existing(&pdf_path),
        diagnostics,
        duration_ms: elapsed(),
        log: if log.is_empty() { format!("{label}: compiled {main_file} without warnings.") } else { log },
        engine_label: label.clone(),
        details: BuildDetails {
            program: Some(format!("Typst compiler {TYPST_VERSION} (built into KernTeX)")),
            format: None,
            document_class: None,
            packages,
            passes: 1,
            engine_path: None,
            out_dir: Some(out_dir.display().to_string()),
            log_path: None,
        },
    })
}

fn existing(p: &Path) -> Option<String> {
    p.is_file().then(|| p.display().to_string())
}

fn log_line(d: &Diagnostic) -> String {
    let sev = match d.severity {
        Severity::Error => "error",
        Severity::Warning => "warning",
        Severity::Info => "info",
    };
    let loc = match (&d.file, d.line) {
        (Some(f), Some(l)) => format!("{f}:{l}: "),
        (Some(f), None) => format!("{f}: "),
        _ => String::new(),
    };
    let hint = d.hint.as_deref().map(|h| format!("\n  hint: {h}")).unwrap_or_default();
    format!("{sev}: {loc}{}{hint}", d.message)
}

/// Typst packages (`@namespace/name:version`) the document actually loaded.
fn used_packages(world: &mut KernWorld) -> Vec<PackageRef> {
    let mut out: Vec<PackageRef> = Vec::new();
    let (_, ids) = world.files.dependencies();
    for id in ids {
        if let VirtualRoot::Package(spec) = id.root() {
            let name = format!("@{}/{}", spec.namespace, spec.name);
            if !out.iter().any(|p| p.name == name) {
                out.push(PackageRef { name, date: None, version: Some(format!("v{}", spec.version)), description: None });
            }
        }
    }
    out
}

fn convert(world: &KernWorld, d: &SourceDiagnostic) -> Diagnostic {
    let severity = match d.severity {
        TypstSeverity::Error => Severity::Error,
        TypstSeverity::Warning => Severity::Warning,
    };
    let mut file = None;
    let (mut line, mut column, mut end_line, mut end_column) = (None, None, None, None);
    let mut source = "Typst".to_string();
    if let Some(id) = d.span.id() {
        match id.root() {
            VirtualRoot::Project => file = Some(id.vpath().get_with_slash().trim_start_matches('/').to_string()),
            VirtualRoot::Package(spec) => {
                source = format!("@{}/{}", spec.namespace, spec.name);
                file = Some(format!("@{}/{}:{}{}", spec.namespace, spec.name, spec.version, id.vpath().get_with_slash()));
            }
        }
        if let (Some(range), Ok(src)) = (world.range(d.span), world.source(id)) {
            if let Some((l, c)) = src.lines().byte_to_line_column(range.start) {
                line = Some(l as u32 + 1);
                column = Some(c as u32 + 1);
            }
            if let Some((l, c)) = src.lines().byte_to_line_column(range.end) {
                end_line = Some(l as u32 + 1);
                end_column = Some(c as u32 + 1);
            }
        }
    }
    let hints: Vec<String> = d.hints.iter().map(|h| h.v.to_string()).collect();
    let mut hint = if hints.is_empty() { None } else { Some(hints.join(" ")) };
    if hint.is_none() {
        hint = super::diagnostics::typst_hint(&d.message);
    }
    Diagnostic { severity, source, file, line, column, end_line, end_column, message: d.message.to_string(), hint }
}

impl CancelHandle {
    fn begin_in_process(&self) {
        self.in_process.store(true, Ordering::SeqCst);
        self.canceled.store(false, Ordering::SeqCst);
    }
    fn end_in_process(&self) {
        self.in_process.store(false, Ordering::SeqCst);
    }
    /// True once if a cancel was requested while an in-process build runs.
    fn take_in_process_cancel(&self) -> bool {
        if self.canceled.swap(false, Ordering::SeqCst) {
            self.in_process.store(false, Ordering::SeqCst);
            return true;
        }
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn tmp(tag: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!("kerntex-typst-{tag}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&d);
        fs::create_dir_all(&d).unwrap();
        d
    }

    #[test]
    fn compiles_and_reports_precise_errors() {
        let root = tmp("basic");
        project::write_text(&root, "main.typ", "= Hello\nThis is *Typst*. $x^2$\n").unwrap();
        let res = compile(&root, "main.typ", &CancelHandle::default(), |_| {}).unwrap();
        assert!(res.ok, "{:?}", res.diagnostics);
        let pdf = fs::read(res.pdf_path.as_ref().unwrap()).unwrap();
        assert!(pdf.starts_with(b"%PDF"));

        project::write_text(&root, "main.typ", "= Broken\nSome text\n#unknownfunc(1)\n").unwrap();
        let res = compile(&root, "main.typ", &CancelHandle::default(), |_| {}).unwrap();
        assert!(!res.ok);
        let err = res.diagnostics.iter().find(|d| d.severity == Severity::Error).expect("an error");
        assert_eq!(err.file.as_deref(), Some("main.typ"));
        assert_eq!(err.line, Some(3));
        assert_eq!(err.column, Some(2), "{err:?}");
        assert!(err.message.contains("unknownfunc"), "{err:?}");
        fs::remove_dir_all(root).ok();
    }

    #[test]
    fn errors_in_included_files_point_there() {
        let root = tmp("include");
        project::write_text(&root, "main.typ", "#include \"chapters/a.typ\"\n").unwrap();
        project::write_text(&root, "chapters/a.typ", "Hello\n\n#let x = (1 + \"a\")\n").unwrap();
        let res = compile(&root, "main.typ", &CancelHandle::default(), |_| {}).unwrap();
        let err = res.diagnostics.iter().find(|d| d.severity == Severity::Error).expect("an error");
        assert_eq!((err.file.as_deref(), err.line), (Some("chapters/a.typ"), Some(3)), "{err:?}");
        fs::remove_dir_all(root).ok();
    }
}
