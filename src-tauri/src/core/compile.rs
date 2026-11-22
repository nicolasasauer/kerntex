//! Running the TeX engine with live progress, cancellation and parsed results.

use super::diagnostics::{self, Diagnostic, Severity};
use super::error::{AppError, ErrorSource, Result};
use super::project::{self, BUILD_DIR};
use serde::{Deserialize, Serialize};
use std::io::{BufRead, BufReader, Read};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum Engine {
    /// Bundled, self-contained; downloads packages on demand.
    Tectonic,
    /// System TeX distribution (TeX Live / MiKTeX) via latexmk.
    Latexmk,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum TexFlavor {
    Pdflatex,
    Xelatex,
    Lualatex,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CompileRequest {
    pub root: String,
    pub main_file: String,
    pub engine: Engine,
    /// Only used by latexmk (Tectonic is always XeTeX-based).
    pub flavor: TexFlavor,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Progress {
    /// Short, human readable: "Typesetting (pass 2)", "Downloading packages" …
    pub stage: String,
    pub detail: Option<String>,
    pub elapsed_ms: u64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CompileResult {
    pub ok: bool,
    pub canceled: bool,
    /// Absolute path of the produced PDF (also set when errors occurred but a PDF exists).
    pub pdf_path: Option<String>,
    pub diagnostics: Vec<Diagnostic>,
    pub duration_ms: u64,
    /// Raw engine output for the "Raw log" view.
    pub log: String,
    pub engine_label: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EngineStatus {
    pub tectonic: Option<String>,
    pub latexmk: Option<String>,
    /// False until Tectonic has downloaded its package bundle once. The UI
    /// uses this to announce the one-time download on the first compile.
    pub tectonic_cache_ready: bool,
}

/// Shared handle so a running build can be cancelled from another thread.
#[derive(Default, Clone)]
pub struct CancelHandle {
    child: Arc<Mutex<Option<Child>>>,
    canceled: Arc<AtomicBool>,
}

impl CancelHandle {
    /// Kills the running engine process. Returns false if nothing was running.
    pub fn cancel(&self) -> bool {
        if let Some(child) = self.child.lock().unwrap().as_mut() {
            self.canceled.store(true, Ordering::SeqCst);
            kill_tree(child);
            return true;
        }
        false
    }

    pub fn is_running(&self) -> bool {
        self.child.lock().unwrap().is_some()
    }
}

pub fn detect_engines() -> EngineStatus {
    EngineStatus {
        tectonic: find_tectonic().map(|p| p.display().to_string()),
        latexmk: find_in_path("latexmk").map(|p| p.display().to_string()),
        tectonic_cache_ready: tectonic_cache_dir().is_some_and(|d| d.read_dir().map(|mut r| r.next().is_some()).unwrap_or(false)),
    }
}

/// Where Tectonic keeps downloaded packages (mirrors its own lookup).
fn tectonic_cache_dir() -> Option<PathBuf> {
    if let Some(d) = std::env::var_os("TECTONIC_CACHE_DIR") {
        return Some(PathBuf::from(d));
    }
    let env = |k: &str| std::env::var_os(k).map(PathBuf::from);
    if cfg!(windows) {
        env("LOCALAPPDATA").map(|d| d.join("TectonicProject").join("Tectonic"))
    } else if cfg!(target_os = "macos") {
        env("HOME").map(|d| d.join("Library/Caches/Tectonic"))
    } else {
        env("XDG_CACHE_HOME").or_else(|| env("HOME").map(|h| h.join(".cache"))).map(|d| d.join("Tectonic"))
    }
}

/// Bundled sidecar next to the executable first, then `PATH`.
pub fn find_tectonic() -> Option<PathBuf> {
    if let Ok(exe) = std::env::current_exe() {
        if let Some(dir) = exe.parent() {
            let p = dir.join(exe_name("tectonic"));
            if p.is_file() {
                return Some(p);
            }
        }
    }
    find_in_path("tectonic")
}

fn exe_name(name: &str) -> String {
    if cfg!(windows) {
        format!("{name}.exe")
    } else {
        name.to_string()
    }
}

pub fn find_in_path(name: &str) -> Option<PathBuf> {
    let path = std::env::var_os("PATH")?;
    let exts: Vec<String> = if cfg!(windows) {
        std::env::var("PATHEXT").unwrap_or(".EXE;.BAT;.CMD".into()).split(';').map(|s| s.to_lowercase()).collect()
    } else {
        vec![String::new()]
    };
    for dir in std::env::split_paths(&path) {
        for ext in &exts {
            let p = dir.join(format!("{name}{ext}"));
            if p.is_file() {
                return Some(p);
            }
        }
    }
    None
}

pub fn compile(req: &CompileRequest, cancel: &CancelHandle, mut on_progress: impl FnMut(Progress) + Send) -> Result<CompileResult> {
    let root = PathBuf::from(&req.root);
    let main_abs = project::resolve(&root, &req.main_file)?;
    if !main_abs.is_file() {
        return Err(AppError::new(ErrorSource::App, format!("Main file “{}” doesn't exist", req.main_file))
            .hint("Pick the root document in the file tree (right-click → “Set as main file”)."));
    }
    let out_dir = root.join(BUILD_DIR).join("out");
    std::fs::create_dir_all(&out_dir).map_err(|e| AppError::io("create the build folder", &out_dir, e))?;

    let (mut cmd, label) = build_command(req, &root, &out_dir)?;
    let start = Instant::now();
    on_progress(Progress { stage: "Starting engine".into(), detail: Some(label.clone()), elapsed_ms: 0 });

    let child = cmd
        .current_dir(&root)
        .env("max_print_line", "10000") // stop TeX wrapping log lines at 79 chars; >~30000 breaks BibTeX
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| {
            AppError::new(ErrorSource::Engine, format!("Couldn't start {label}"))
                .detail(e.to_string())
                .hint(missing_engine_hint(req.engine))
        })?;

    let (stdout, stderr) = {
        cancel.canceled.store(false, Ordering::SeqCst);
        let mut slot = cancel.child.lock().unwrap();
        *slot = Some(child);
        let c = slot.as_mut().unwrap();
        (c.stdout.take().unwrap(), c.stderr.take().unwrap())
    };

    // Collect both streams; stderr lines also drive progress updates.
    let (tx, rx) = std::sync::mpsc::channel::<String>();
    let tx2 = tx.clone();
    let t_out = std::thread::spawn(move || read_lines(stdout, tx));
    let t_err = std::thread::spawn(move || read_lines(stderr, tx2));

    let mut log = String::new();
    let mut pass = 0u32;
    let status = loop {
        while let Ok(line) = rx.try_recv() {
            if let Some(p) = progress_for(&line, req.engine, &mut pass, start) {
                on_progress(p);
            }
            if log.len() < 4_000_000 {
                log.push_str(&line);
                log.push('\n');
            }
        }
        let done = {
            let mut slot = cancel.child.lock().unwrap();
            match slot.as_mut().map(|c| c.try_wait()) {
                Some(Ok(Some(status))) => {
                    *slot = None;
                    Some(Ok(status))
                }
                Some(Ok(None)) => None,
                Some(Err(e)) => {
                    *slot = None;
                    Some(Err(e))
                }
                None => Some(Err(std::io::Error::other("canceled"))),
            }
        };
        if let Some(s) = done {
            break s;
        }
        std::thread::sleep(Duration::from_millis(30));
    };
    let _ = t_out.join();
    let _ = t_err.join();
    while let Ok(line) = rx.try_recv() {
        log.push_str(&line);
        log.push('\n');
    }

    let duration_ms = start.elapsed().as_millis() as u64;
    let stem = main_abs.file_stem().unwrap().to_string_lossy().into_owned();
    let pdf = out_dir.join(format!("{stem}.pdf"));

    let status = match status {
        Ok(s) => s,
        Err(_) => {
            return Ok(CompileResult {
                ok: false,
                canceled: true,
                pdf_path: existing(&pdf),
                diagnostics: vec![],
                duration_ms,
                log,
                engine_label: label,
            })
        }
    };
    let killed = cancel.canceled.swap(false, Ordering::SeqCst);
    if killed {
        return Ok(CompileResult {
            ok: false,
            canceled: true,
            pdf_path: existing(&pdf),
            diagnostics: vec![],
            duration_ms,
            log,
            engine_label: label,
        });
    }

    let mut diags = match req.engine {
        Engine::Tectonic => diagnostics::parse_tectonic(&log),
        Engine::Latexmk => {
            let tex_log = std::fs::read_to_string(out_dir.join(format!("{stem}.log"))).unwrap_or_default();
            let mut d = diagnostics::parse_tex_log(&tex_log);
            for (ext, biber) in [("blg", false), ("blg", true)] {
                if let Ok(b) = std::fs::read_to_string(out_dir.join(format!("{stem}.{ext}"))) {
                    if b.contains("biber") == biber {
                        d.extend(diagnostics::parse_bib_log(&b, biber));
                    }
                }
            }
            d
        }
    };
    // Make file paths relative to the project if the engine reported absolute ones.
    for d in &mut diags {
        if let Some(f) = &d.file {
            let p = Path::new(f);
            if p.is_absolute() {
                d.file = project::to_rel(&root, p).or(d.file.take());
            }
        }
    }

    let has_errors = diags.iter().any(|d| d.severity == Severity::Error);
    if !status.success() && !has_errors {
        // The engine failed but we couldn't pinpoint why: say so honestly.
        diags.insert(0, Diagnostic {
            severity: Severity::Error,
            source: label.clone(),
            file: None,
            line: None,
            message: format!("{label} stopped with exit code {}", status.code().unwrap_or(-1)),
            hint: Some("KernTeX couldn't find a specific error message. Open “Raw log” at the bottom of the Problems panel to see the engine's full output.".into()),
        });
    }

    Ok(CompileResult {
        ok: status.success() && !has_errors,
        canceled: false,
        pdf_path: existing(&pdf),
        diagnostics: diags,
        duration_ms,
        log,
        engine_label: label,
    })
}

fn existing(p: &Path) -> Option<String> {
    p.is_file().then(|| p.display().to_string())
}

fn read_lines(stream: impl Read, tx: std::sync::mpsc::Sender<String>) {
    for line in BufReader::new(stream).lines().map_while(|l| l.ok()) {
        if tx.send(line).is_err() {
            break;
        }
    }
}

fn build_command(req: &CompileRequest, _root: &Path, out_dir: &Path) -> Result<(Command, String)> {
    match req.engine {
        Engine::Tectonic => {
            let bin = find_tectonic()
                .ok_or_else(|| AppError::new(ErrorSource::Engine, "Tectonic wasn't found").hint(missing_engine_hint(Engine::Tectonic)))?;
            let mut c = Command::new(bin);
            c.args(["-X", "compile", "--synctex", "--keep-logs", "--keep-intermediates", "--outdir"]).arg(out_dir).arg(&req.main_file);
            hide_window(&mut c);
            Ok((c, "Tectonic".into()))
        }
        Engine::Latexmk => {
            let bin = find_in_path("latexmk")
                .ok_or_else(|| AppError::new(ErrorSource::Engine, "latexmk wasn't found").hint(missing_engine_hint(Engine::Latexmk)))?;
            let (flag, name) = match req.flavor {
                TexFlavor::Pdflatex => ("-pdf", "pdfLaTeX"),
                TexFlavor::Xelatex => ("-xelatex", "XeLaTeX"),
                TexFlavor::Lualatex => ("-lualatex", "LuaLaTeX"),
            };
            let mut c = Command::new(bin);
            c.args([flag, "-interaction=nonstopmode", "-file-line-error", "-synctex=1", "-f"])
                .arg(format!("-outdir={}", out_dir.display()))
                .arg(&req.main_file);
            hide_window(&mut c);
            Ok((c, format!("latexmk ({name})")))
        }
    }
}

/// No console window on Windows; on Unix the engine gets its own process
/// group so cancelling also stops the TeX runs latexmk spawns.
#[cfg(windows)]
fn hide_window(c: &mut Command) {
    use std::os::windows::process::CommandExt;
    const CREATE_NO_WINDOW: u32 = 0x0800_0000;
    c.creation_flags(CREATE_NO_WINDOW);
}
#[cfg(unix)]
fn hide_window(c: &mut Command) {
    use std::os::unix::process::CommandExt;
    c.process_group(0);
}

/// Kills the engine and everything it started (latexmk → pdflatex, biber …).
fn kill_tree(child: &mut Child) {
    #[cfg(unix)]
    {
        // Negative PID = the whole process group created in `hide_window`.
        let _ =
            Command::new("kill").args(["-KILL", "--", &format!("-{}", child.id())]).stdout(Stdio::null()).stderr(Stdio::null()).status();
    }
    #[cfg(windows)]
    {
        let mut c = Command::new("taskkill");
        c.args(["/T", "/F", "/PID", &child.id().to_string()]).stdout(Stdio::null()).stderr(Stdio::null());
        hide_window(&mut c);
        let _ = c.status();
    }
    let _ = child.kill();
}

fn missing_engine_hint(engine: Engine) -> &'static str {
    match engine {
        Engine::Tectonic => "Tectonic ships with the KernTeX installer. If you built KernTeX yourself, run `npm run setup:tectonic` or install Tectonic (https://tectonic-typesetting.github.io). Alternatively switch to latexmk in Settings.",
        Engine::Latexmk => "latexmk comes with TeX Live and MiKTeX. Install one of them and make sure it's on your PATH (restart KernTeX afterwards) — or switch to the built-in Tectonic engine in Settings.",
    }
}

/// Maps engine chatter to user-facing progress steps.
fn progress_for(line: &str, engine: Engine, pass: &mut u32, start: Instant) -> Option<Progress> {
    let elapsed_ms = start.elapsed().as_millis() as u64;
    let p = |stage: String, detail: Option<String>| Some(Progress { stage, detail, elapsed_ms });
    match engine {
        Engine::Tectonic => {
            let note = line.strip_prefix("note: ")?;
            if note.starts_with("downloading") {
                p("Downloading packages".into(), note.strip_prefix("downloading ").map(|s| s.to_string()))
            } else if note.starts_with("Running TeX") || note.starts_with("Rerunning TeX") {
                *pass += 1;
                p(format!("Typesetting (pass {pass})"), note.strip_prefix("Rerunning TeX because ").map(|s| format!("because {s}")))
            } else if note.contains("bibtex") || note.contains("biber") {
                p("Processing bibliography".into(), None)
            } else if note.contains("xdvipdfmx") {
                p("Writing PDF".into(), None)
            } else {
                None
            }
        }
        Engine::Latexmk => {
            if let Some(rest) = line.strip_prefix("Run number ") {
                let rule = rest.split('\'').nth(1).unwrap_or("latex");
                if rule.contains("bib") || rule.contains("biber") {
                    return p("Processing bibliography".into(), Some(rule.to_string()));
                }
                *pass += 1;
                p(format!("Typesetting (pass {pass})"), Some(rule.to_string()))
            } else if line.starts_with("Latexmk: All targets") && line.contains("up-to-date") {
                p("Already up to date".into(), None)
            } else {
                None
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn progress_mapping() {
        let s = Instant::now();
        let mut pass = 0;
        assert_eq!(progress_for("note: Running TeX ...", Engine::Tectonic, &mut pass, s).unwrap().stage, "Typesetting (pass 1)");
        assert_eq!(
            progress_for("note: downloading amsmath.sty", Engine::Tectonic, &mut pass, s).unwrap().detail.as_deref(),
            Some("amsmath.sty")
        );
        assert_eq!(progress_for("Run number 1 of rule 'pdflatex'", Engine::Latexmk, &mut pass, s).unwrap().stage, "Typesetting (pass 2)");
        assert!(progress_for("random", Engine::Latexmk, &mut pass, s).is_none());
    }

    /// End-to-end test; runs only where latexmk is installed.
    #[test]
    fn latexmk_end_to_end() {
        if find_in_path("latexmk").is_none() {
            return;
        }
        let root = std::env::temp_dir().join(format!("kerntex-e2e-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        project::write_text(
            &root,
            "main.tex",
            "\\documentclass{article}\n\\begin{document}\nHello \\foo world, see \\ref{nope}.\n\\end{document}\n",
        )
        .unwrap();
        let req = CompileRequest {
            root: root.display().to_string(),
            main_file: "main.tex".into(),
            engine: Engine::Latexmk,
            flavor: TexFlavor::Pdflatex,
        };
        let mut stages = vec![];
        let res = compile(&req, &CancelHandle::default(), |p| stages.push(p.stage)).unwrap();
        assert!(!res.ok);
        assert!(res.pdf_path.is_some(), "PDF should still be produced with -f");
        let err = res.diagnostics.iter().find(|d| d.severity == Severity::Error).expect("an error");
        assert_eq!(err.file.as_deref(), Some("main.tex"));
        assert_eq!(err.line, Some(3));
        assert!(err.hint.is_some());
        assert!(res.diagnostics.iter().any(|d| d.message.contains("Reference `nope'")));
        assert!(stages.iter().any(|s| s.starts_with("Typesetting")));
        std::fs::remove_dir_all(root).ok();
    }

    /// The welcome project and every template must compile cleanly.
    #[test]
    fn latexmk_compiles_welcome_and_templates() {
        use crate::core::templates::{self, Template};
        if find_in_path("latexmk").is_none() {
            eprintln!("latexmk not installed; skipping");
            return;
        }
        let base = std::env::temp_dir().join(format!("kerntex-e2e-tpl-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&base);
        std::fs::create_dir_all(&base).unwrap();
        let mut roots = vec![templates::ensure_welcome(&base, false).unwrap()];
        for (i, t) in [Template::Blank, Template::Article, Template::Report, Template::Beamer, Template::Letter].into_iter().enumerate() {
            roots.push(templates::new_project(&base, &format!("t{i}"), t).unwrap());
        }
        for root in roots {
            let req = CompileRequest {
                root: root.display().to_string(),
                main_file: "main.tex".into(),
                engine: Engine::Latexmk,
                flavor: TexFlavor::Pdflatex,
            };
            let res = compile(&req, &CancelHandle::default(), |_| {}).unwrap();
            let errors: Vec<_> = res.diagnostics.iter().filter(|d| d.severity == Severity::Error).collect();
            assert!(res.ok && res.pdf_path.is_some(), "{}: {errors:?}\n{}", root.display(), &res.log[res.log.len().saturating_sub(3000)..]);
            let undefined: Vec<_> = res.diagnostics.iter().filter(|d| d.message.contains("undefined")).collect();
            assert!(undefined.is_empty(), "{}: unresolved refs {undefined:?}", root.display());
        }
        std::fs::remove_dir_all(base).ok();
    }

    #[test]
    fn cancel_stops_build() {
        if find_in_path("latexmk").is_none() {
            return;
        }
        let root = std::env::temp_dir().join(format!("kerntex-cancel-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        // An endless loop keeps TeX busy until it is killed.
        project::write_text(&root, "main.tex", "\\documentclass{article}\\begin{document}\\def\\x{\\x}\\x\\end{document}").unwrap();
        let req = CompileRequest {
            root: root.display().to_string(),
            main_file: "main.tex".into(),
            engine: Engine::Latexmk,
            flavor: TexFlavor::Pdflatex,
        };
        let handle = CancelHandle::default();
        let h2 = handle.clone();
        let t = std::thread::spawn(move || {
            std::thread::sleep(Duration::from_millis(1500));
            assert!(h2.cancel());
        });
        let res = compile(&req, &handle, |_| {}).unwrap();
        t.join().unwrap();
        assert!(res.canceled && !res.ok);
        assert!(!handle.is_running());
        std::fs::remove_dir_all(root).ok();
    }
}
