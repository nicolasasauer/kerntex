//! C ABI over the KernTeX core for the Flutter UI.
//!
//! Deliberately tiny and hand-written — no code generator, no async runtime:
//!
//! * `kt_call(method, json)` runs one core function and returns
//!   `{"ok": …}` or `{"err": AppError}` as a NUL-terminated UTF-8 string that
//!   the caller releases with `kt_free`. It blocks, so Dart calls slow methods
//!   from a helper isolate.
//! * `kt_cancel` / `kt_progress` are cheap and safe from any thread, so the UI
//!   isolate can stop or observe a build running on another isolate.
//! * `kt_pdf_render` rasterises one PDF page straight into an RGBA buffer
//!   (`kt_buf_free`), so the UI never needs a PDF engine of its own.
//!
//! The core itself is shared verbatim with the Tauri app (`src-tauri/src/core`).

#[path = "../../../src-tauri/src/core/mod.rs"]
pub mod core;

mod pdf;

use crate::core::compile::{self, CancelHandle, CompileRequest, Progress};
use crate::core::error::{AppError, ErrorSource, Result};
use crate::core::project::{self, BUILD_DIR};
use crate::core::settings::{self, ProjectSettings};
use crate::core::templates::{self, Template};
use crate::core::{archive, refs};
use serde::de::DeserializeOwned;
use serde::Deserialize;
use serde_json::{json, Value};
use std::ffi::{c_char, CStr, CString};
use std::path::{Path, PathBuf};
use std::sync::{LazyLock, Mutex};

static CANCEL: LazyLock<CancelHandle> = LazyLock::new(CancelHandle::default);
static PROGRESS: Mutex<Option<Progress>> = Mutex::new(None);

fn text(p: *const c_char) -> String {
    if p.is_null() {
        return String::new();
    }
    // SAFETY: the caller passes a NUL-terminated string that outlives the call.
    unsafe { CStr::from_ptr(p) }.to_string_lossy().into_owned()
}

fn out(v: Value) -> *mut c_char {
    CString::new(v.to_string()).unwrap_or_default().into_raw()
}

/// Runs `method` with JSON `args`; see the module docs for the result shape.
///
/// # Safety
/// `method` and `args` are null or valid NUL-terminated strings.
#[no_mangle]
pub unsafe extern "C" fn kt_call(method: *const c_char, args: *const c_char) -> *mut c_char {
    let method = text(method);
    let args: Value = serde_json::from_str(&text(args)).unwrap_or(Value::Null);
    let res = std::panic::catch_unwind(|| dispatch(&method, args))
        .unwrap_or_else(|_| Err(AppError::new(ErrorSource::App, "Internal error").detail(format!("{method} panicked"))));
    out(match res {
        Ok(v) => json!({ "ok": v }),
        Err(e) => json!({ "err": e }),
    })
}

/// Releases a string returned by `kt_call` or `kt_progress`.
///
/// # Safety
/// `p` is null or came from this library and is released only once.
#[no_mangle]
pub unsafe extern "C" fn kt_free(p: *mut c_char) {
    if !p.is_null() {
        // SAFETY: `p` came from `CString::into_raw` in this library.
        drop(unsafe { CString::from_raw(p) });
    }
}

/// Stops the running build. Returns 1 if one was running.
#[no_mangle]
pub extern "C" fn kt_cancel() -> u8 {
    CANCEL.cancel() as u8
}

/// The latest progress report of the running build as JSON (`null` if none).
#[no_mangle]
pub extern "C" fn kt_progress() -> *mut c_char {
    out(serde_json::to_value(&*PROGRESS.lock().unwrap()).unwrap_or(Value::Null))
}

/// Renders page `page` of the PDF at `path` at `scale` pixels per PDF point.
/// Writes width/height to the out-pointers and returns a tightly packed RGBA
/// buffer of `w*h*4` bytes (release with `kt_buf_free`), or null on failure.
///
/// # Safety
/// `path` is a valid NUL-terminated string; `w` and `h` are valid for writes.
#[no_mangle]
pub unsafe extern "C" fn kt_pdf_render(path: *const c_char, page: u32, scale: f32, w: *mut u32, h: *mut u32) -> *mut u8 {
    let path = text(path);
    match std::panic::catch_unwind(|| pdf::render(Path::new(&path), page as usize, scale)) {
        Ok(Some((pw, ph, mut buf))) => {
            // SAFETY: the caller passes valid out-pointers.
            unsafe {
                *w = pw;
                *h = ph;
            }
            buf.shrink_to_fit();
            let p = buf.as_mut_ptr();
            std::mem::forget(buf);
            p
        }
        _ => std::ptr::null_mut(),
    }
}

/// Releases a buffer returned by `kt_pdf_render`.
///
/// # Safety
/// `p`/`len` come from one `kt_pdf_render` call and are released only once.
#[no_mangle]
pub unsafe extern "C" fn kt_buf_free(p: *mut u8, len: usize) {
    if !p.is_null() {
        // SAFETY: `p`/`len` describe a Vec<u8> leaked by `kt_pdf_render` (capacity == len).
        drop(unsafe { Vec::from_raw_parts(p, len, len) });
    }
}

fn arg<T: DeserializeOwned>(args: &Value, key: &str) -> Result<T> {
    serde_json::from_value(args.get(key).cloned().unwrap_or(Value::Null))
        .map_err(|e| AppError::new(ErrorSource::App, "Invalid request").detail(format!("{key}: {e}")))
}

fn s(args: &Value, key: &str) -> Result<String> {
    arg(args, key)
}

fn val<T: serde::Serialize>(v: T) -> Result<Value> {
    Ok(serde_json::to_value(v).unwrap_or(Value::Null))
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
enum WelcomeKind {
    Latex,
    Typst,
}

fn dispatch(method: &str, a: Value) -> Result<Value> {
    let root = || s(&a, "root").map(PathBuf::from);
    match method {
        "detectEngines" => val(compile::detect_engines()),
        "openProject" => val(project::open(&root()?)?),
        "readTree" => val(project::read_tree(&root()?)?),
        "readFile" => val(project::read_text(&root()?, &s(&a, "path")?)?),
        "writeFile" => val(project::write_text(&root()?, &s(&a, "path")?, &s(&a, "content")?)?),
        "createFile" => val(project::create_file(&root()?, &s(&a, "path")?)?),
        "createDir" => val(project::create_dir(&root()?, &s(&a, "path")?)?),
        "rename" => val(project::rename(&root()?, &s(&a, "from")?, &s(&a, "to")?)?),
        "trash" => val(project::trash(&root()?, &s(&a, "path")?)?),
        "importFiles" => {
            let sources: Vec<PathBuf> = arg::<Vec<String>>(&a, "sources")?.into_iter().map(PathBuf::from).collect();
            val(project::import_files(&root()?, &s(&a, "dir")?, &sources)?)
        }
        "scanRefs" => {
            let root = root()?;
            let tree = project::read_tree(&root)?;
            val(refs::scan(&root, &tree))
        }
        "saveSettings" => val(settings::save(&root()?, &arg::<ProjectSettings>(&a, "settings")?)?),
        "texRoot" => val(settings::tex_root(&root()?, &s(&a, "path")?)),
        "newProject" => {
            let p = templates::new_project(Path::new(&s(&a, "parent")?), &s(&a, "name")?, arg::<Template>(&a, "template")?)?;
            val(p.display().to_string())
        }
        "welcome" => {
            let docs = documents_dir()?;
            let fresh = arg::<Option<bool>>(&a, "fresh")?.unwrap_or(false);
            let p = match arg::<WelcomeKind>(&a, "kind")? {
                WelcomeKind::Latex => templates::ensure_welcome(&docs, fresh)?,
                WelcomeKind::Typst => templates::ensure_typst_welcome(&docs, fresh)?,
            };
            val(p.display().to_string())
        }
        "documentsDir" => val(documents_dir()?.display().to_string()),
        "importZip" => val(archive::import_zip(Path::new(&s(&a, "zip")?), Path::new(&s(&a, "destParent")?))?.display().to_string()),
        "exportZip" => val(archive::export_zip(&root()?, Path::new(&s(&a, "dest")?), arg(&a, "clean")?)?),
        "pdfPath" => val(pdf_path(&root()?, &s(&a, "mainFile")?)?.display().to_string()),
        "exportPdf" => {
            let p = pdf_path(&root()?, &s(&a, "mainFile")?)?;
            let dest = s(&a, "dest")?;
            std::fs::copy(&p, &dest).map_err(|e| AppError::io("export the PDF to", Path::new(&dest), e))?;
            val(())
        }
        "pdfInfo" => val(pdf::info(Path::new(&s(&a, "path")?))),
        "compile" => {
            let req: CompileRequest = arg(&a, "request")?;
            // Only one build at a time: stop a running one and let it wind down.
            if CANCEL.cancel() {
                let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
                while CANCEL.is_running() && std::time::Instant::now() < deadline {
                    std::thread::sleep(std::time::Duration::from_millis(20));
                }
            }
            let res = compile::compile(&req, &CANCEL, |p| *PROGRESS.lock().unwrap() = Some(p));
            *PROGRESS.lock().unwrap() = None;
            pdf::forget(&pdf_path(Path::new(&req.root), &req.main_file).unwrap_or_default());
            val(res?)
        }
        "openUrl" => {
            let url = s(&a, "url")?;
            opener::open_browser(&url).map_err(|e| AppError::new(ErrorSource::App, "Couldn't open the link").detail(e.to_string()))?;
            val(())
        }
        "reveal" => {
            let p = PathBuf::from(s(&a, "path")?);
            reveal(&p).map_err(|e| AppError::new(ErrorSource::App, "Couldn't show the file").detail(e))?;
            val(())
        }
        "loadState" => {
            let p = state_file()?;
            val(std::fs::read_to_string(p).ok().and_then(|t| serde_json::from_str::<Value>(&t).ok()))
        }
        "saveState" => {
            let p = state_file()?;
            if let Some(d) = p.parent() {
                let _ = std::fs::create_dir_all(d);
            }
            let tmp = p.with_extension("tmp");
            std::fs::write(&tmp, a.get("state").cloned().unwrap_or(Value::Null).to_string())
                .map_err(|e| AppError::io("save settings to", &tmp, e))?;
            std::fs::rename(&tmp, &p).map_err(|e| AppError::io("save settings to", &p, e))?;
            val(())
        }
        _ => Err(AppError::new(ErrorSource::App, "Unknown request").detail(method.to_string())),
    }
}

/// Shows a file in the system file manager (selected where the platform can).
fn reveal(p: &Path) -> std::result::Result<(), String> {
    #[cfg(windows)]
    {
        let mut arg = std::ffi::OsString::from("/select,");
        arg.push(p);
        return std::process::Command::new("explorer").arg(arg).spawn().map(|_| ()).map_err(|e| e.to_string());
    }
    #[cfg(target_os = "macos")]
    {
        return std::process::Command::new("open").arg("-R").arg(p).spawn().map(|_| ()).map_err(|e| e.to_string());
    }
    #[allow(unreachable_code)]
    {
        let dir = if p.is_dir() { p } else { p.parent().unwrap_or(p) };
        opener::open(dir).map_err(|e| e.to_string())
    }
}

fn pdf_path(root: &Path, main_file: &str) -> Result<PathBuf> {
    project::resolve(root, main_file)?;
    let stem = Path::new(main_file).file_stem().map(|s| s.to_string_lossy().into_owned()).unwrap_or_default();
    Ok(root.join(BUILD_DIR).join("out").join(format!("{stem}.pdf")))
}

fn documents_dir() -> Result<PathBuf> {
    dirs::document_dir()
        .or_else(|| dirs::home_dir().map(|h| if h.join("Documents").is_dir() { h.join("Documents") } else { h }))
        .ok_or_else(|| AppError::new(ErrorSource::FileSystem, "Couldn't find your Documents folder"))
}

/// App settings and recent projects (owned by the UI, stored as opaque JSON).
fn state_file() -> Result<PathBuf> {
    dirs::config_dir()
        .map(|d| d.join("dev.kerntex.flutter").join("state.json"))
        .ok_or_else(|| AppError::new(ErrorSource::FileSystem, "Couldn't find the settings folder"))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn call(method: &str, args: Value) -> Value {
        let m = CString::new(method).unwrap();
        let a = CString::new(args.to_string()).unwrap();
        // SAFETY: valid C strings; the reply is freed exactly once.
        let p = unsafe { kt_call(m.as_ptr(), a.as_ptr()) };
        let v = serde_json::from_str(&text(p)).unwrap();
        unsafe { kt_free(p) };
        v
    }

    #[test]
    fn typst_project_compiles_and_renders() {
        let dir = std::env::temp_dir().join(format!("kt-ffi-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let root = call("newProject", json!({"parent": dir, "name": "T", "template": "typstArticle"}))["ok"].as_str().unwrap().to_string();
        let info = call("openProject", json!({ "root": root }));
        let main = info["ok"]["mainFile"].as_str().unwrap().to_string();
        let res = call("compile", json!({"request": {"root": root, "mainFile": main, "engine": "tectonic", "flavor": "pdflatex"}}));
        assert_eq!(res["ok"]["ok"], true, "{res}");
        let pdf = res["ok"]["pdfPath"].as_str().unwrap().to_string();
        let info = call("pdfInfo", json!({ "path": pdf }));
        assert!(!info["ok"]["pages"].as_array().unwrap().is_empty(), "{info}");
        let (mut w, mut h) = (0, 0);
        let c = CString::new(pdf).unwrap();
        // SAFETY: valid path and out-pointers; the buffer is freed below.
        let buf = unsafe { kt_pdf_render(c.as_ptr(), 0, 1.0, &mut w, &mut h) };
        assert!(!buf.is_null() && w > 500 && h > 700, "{w}x{h}");
        unsafe { kt_buf_free(buf, (w * h * 4) as usize) };
        assert!(call("nope", Value::Null)["err"]["title"].is_string());
        let _ = std::fs::remove_dir_all(&dir);
    }
}
