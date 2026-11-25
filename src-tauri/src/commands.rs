//! Tauri commands: thin wrappers around `core`. No logic lives here beyond
//! argument plumbing, threading and event emission.

use crate::core::archive;
use crate::core::compile::{self, CancelHandle, CompileRequest, CompileResult, EngineStatus};
use crate::core::error::{AppError, ErrorSource, Result};
use crate::core::git::{self, CommitInfo, GitCancel, GitStatus, PullResult};
use crate::core::project::{self, FileNode, ProjectInfo, BUILD_DIR};
use crate::core::refs::{self, ProjectRefs};
use crate::core::settings::{self, ProjectSettings};
use crate::core::templates::{self, Template};
use crate::mcp::{self, Hub};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tauri::{AppHandle, Emitter, Manager, State};

#[derive(Default)]
pub struct AppState {
    pub compile: CancelHandle,
    pub git: GitCancel,
}

/// Runs blocking core work off the main thread.
async fn blocking<T: Send + 'static>(f: impl FnOnce() -> Result<T> + Send + 'static) -> Result<T> {
    tauri::async_runtime::spawn_blocking(f)
        .await
        .map_err(|e| AppError::new(ErrorSource::App, "Internal task failed").detail(e.to_string()))?
}

#[tauri::command]
pub async fn detect_engines() -> EngineStatus {
    tauri::async_runtime::spawn_blocking(compile::detect_engines).await.unwrap_or(EngineStatus {
        tectonic: None,
        latexmk: None,
        tectonic_cache_ready: false,
        tectonic_version: None,
        latexmk_version: None,
        tectonic_cache_dir: None,
        typst_version: crate::core::typst_engine::TYPST_VERSION.into(),
    })
}

#[tauri::command]
pub async fn open_project(root: String) -> Result<ProjectInfo> {
    blocking(move || project::open(Path::new(&root))).await
}

#[tauri::command]
pub async fn read_tree(root: String) -> Result<Vec<FileNode>> {
    blocking(move || project::read_tree(Path::new(&root))).await
}

#[tauri::command]
pub fn project_exists(root: String) -> bool {
    Path::new(&root).is_dir()
}

#[tauri::command]
pub async fn read_file(root: String, path: String) -> Result<String> {
    blocking(move || project::read_text(Path::new(&root), &path)).await
}

#[tauri::command]
pub async fn write_file(root: String, path: String, content: String) -> Result<()> {
    blocking(move || project::write_text(Path::new(&root), &path, &content)).await
}

#[tauri::command]
pub async fn create_file(root: String, path: String) -> Result<()> {
    blocking(move || project::create_file(Path::new(&root), &path)).await
}

#[tauri::command]
pub async fn create_dir(root: String, path: String) -> Result<()> {
    blocking(move || project::create_dir(Path::new(&root), &path)).await
}

#[tauri::command]
pub async fn rename_path(root: String, from: String, to: String) -> Result<()> {
    blocking(move || project::rename(Path::new(&root), &from, &to)).await
}

#[tauri::command]
pub async fn trash_path(root: String, path: String) -> Result<()> {
    blocking(move || project::trash(Path::new(&root), &path)).await
}

/// Copies files/folders from anywhere into the project (drag & drop, "Add files…").
#[tauri::command]
pub async fn import_files(root: String, dir: String, sources: Vec<String>) -> Result<Vec<String>> {
    blocking(move || {
        let sources: Vec<PathBuf> = sources.into_iter().map(PathBuf::from).collect();
        project::import_files(Path::new(&root), &dir, &sources)
    })
    .await
}

/// Raw bytes of a project file (image / PDF previews), without JSON number arrays.
#[tauri::command]
pub async fn read_binary(root: String, path: String) -> Result<tauri::ipc::Response> {
    blocking(move || project::read_bytes(Path::new(&root), &path).map(tauri::ipc::Response::new)).await
}

#[tauri::command]
pub async fn scan_refs(root: String) -> Result<ProjectRefs> {
    blocking(move || {
        let root = PathBuf::from(root);
        let tree = project::read_tree(&root)?;
        Ok(refs::scan(&root, &tree))
    })
    .await
}

#[tauri::command]
pub async fn save_project_settings(root: String, settings: ProjectSettings) -> Result<()> {
    blocking(move || settings::save(Path::new(&root), &settings)).await
}

#[tauri::command]
pub async fn tex_root(root: String, path: String) -> Option<String> {
    tauri::async_runtime::spawn_blocking(move || settings::tex_root(Path::new(&root), &path)).await.ok().flatten()
}

#[tauri::command]
pub async fn new_project(parent: String, name: String, template: Template) -> Result<String> {
    blocking(move || templates::new_project(Path::new(&parent), &name, template).map(|p| p.display().to_string())).await
}

/// Creates (or reuses) the example project in the user's Documents folder.
#[tauri::command]
pub async fn welcome_project(app: AppHandle, fresh: bool) -> Result<String> {
    let paths = app.path();
    let docs = paths
        .document_dir()
        .or_else(|_| {
            // No XDG user-dirs config (minimal Linux setups): ~/Documents if present, else ~.
            paths.home_dir().map(|h| if h.join("Documents").is_dir() { h.join("Documents") } else { h })
        })
        .map_err(|e| AppError::new(ErrorSource::FileSystem, "Couldn't find your Documents folder").detail(e.to_string()))?;
    blocking(move || templates::ensure_welcome(&docs, fresh).map(|p| p.display().to_string())).await
}

/// Creates (or reuses) the Typst tour in the user's Documents folder.
#[tauri::command]
pub async fn typst_welcome_project(app: AppHandle, fresh: bool) -> Result<String> {
    let docs = documents_dir(&app)?;
    blocking(move || templates::ensure_typst_welcome(&docs, fresh).map(|p| p.display().to_string())).await
}

fn documents_dir(app: &AppHandle) -> Result<PathBuf> {
    let paths = app.path();
    paths
        .document_dir()
        .or_else(|_| paths.home_dir().map(|h| if h.join("Documents").is_dir() { h.join("Documents") } else { h }))
        .map_err(|e| AppError::new(ErrorSource::FileSystem, "Couldn't find your Documents folder").detail(e.to_string()))
}

#[tauri::command]
pub async fn import_zip(zip: String, dest_parent: String) -> Result<String> {
    blocking(move || archive::import_zip(Path::new(&zip), Path::new(&dest_parent)).map(|p| p.display().to_string())).await
}

#[tauri::command]
pub async fn export_zip(root: String, dest: String, clean: bool) -> Result<usize> {
    blocking(move || archive::export_zip(Path::new(&root), Path::new(&dest), clean)).await
}

fn pdf_path(root: &str, main_file: &str) -> Result<PathBuf> {
    let root = Path::new(root);
    project::resolve(root, main_file)?;
    let stem = Path::new(main_file).file_stem().map(|s| s.to_string_lossy().into_owned()).unwrap_or_default();
    Ok(root.join(BUILD_DIR).join("out").join(format!("{stem}.pdf")))
}

/// The compiled PDF as raw bytes (no JSON number arrays).
#[tauri::command]
pub async fn read_pdf(root: String, main_file: String) -> Result<tauri::ipc::Response> {
    blocking(move || {
        let p = pdf_path(&root, &main_file)?;
        std::fs::read(&p).map(tauri::ipc::Response::new).map_err(|e| AppError::io("open the PDF", &p, e))
    })
    .await
}

#[tauri::command]
pub async fn export_pdf(root: String, main_file: String, dest: String) -> Result<()> {
    blocking(move || {
        let p = pdf_path(&root, &main_file)?;
        std::fs::copy(&p, &dest).map(|_| ()).map_err(|e| AppError::io("export the PDF to", Path::new(&dest), e))
    })
    .await
}

#[tauri::command]
pub async fn compile(
    app: AppHandle,
    state: State<'_, AppState>,
    hub: State<'_, Arc<Hub>>,
    request: CompileRequest,
) -> Result<CompileResult> {
    let cancel = state.compile.clone();
    let hub = hub.inner().clone();
    let progress_hub = hub.clone();
    let result = blocking(move || {
        // Only one build at a time: stop a running one and wait for it to wind down.
        if cancel.cancel() {
            let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
            while cancel.is_running() && std::time::Instant::now() < deadline {
                std::thread::sleep(std::time::Duration::from_millis(20));
            }
        }
        compile::compile(&request, &cancel, |p| {
            progress_hub.compile_progress(&p);
            let _ = app.emit("compile://progress", p);
        })
    })
    .await;
    // Agents read the outcome through the MCP tools `compile` / `get_diagnostics`.
    hub.record_compile(&result);
    result
}

#[tauri::command]
pub fn cancel_compile(state: State<'_, AppState>) -> bool {
    state.compile.cancel()
}

// ---------- Git ----------

#[tauri::command]
pub async fn git_status(root: String) -> Result<GitStatus> {
    blocking(move || git::status(Path::new(&root))).await
}

#[tauri::command]
pub async fn git_diff(root: String, path: String, old_path: Option<String>) -> Result<String> {
    blocking(move || git::diff(Path::new(&root), &path, old_path.as_deref())).await
}

#[tauri::command]
pub async fn git_suggest_message(root: String) -> Result<String> {
    blocking(move || git::suggest_message(Path::new(&root))).await
}

#[tauri::command]
pub async fn git_init(root: String) -> Result<()> {
    blocking(move || git::init(Path::new(&root))).await
}

#[tauri::command]
pub async fn git_commit(root: String, message: String) -> Result<CommitInfo> {
    blocking(move || git::commit(Path::new(&root), &message)).await
}

#[tauri::command]
pub async fn git_pull(app: AppHandle, state: State<'_, AppState>, root: String) -> Result<PullResult> {
    let cancel = state.git.clone();
    blocking(move || {
        git::pull(Path::new(&root), &cancel, |p| {
            let _ = app.emit("git://progress", p);
        })
    })
    .await
}

#[tauri::command]
pub async fn git_push(app: AppHandle, state: State<'_, AppState>, root: String) -> Result<()> {
    let cancel = state.git.clone();
    blocking(move || {
        git::push(Path::new(&root), &cancel, |p| {
            let _ = app.emit("git://progress", p);
        })
    })
    .await
}

#[tauri::command]
pub fn git_cancel(state: State<'_, AppState>) -> bool {
    state.git.cancel()
}

#[tauri::command]
pub async fn git_set_remote(root: String, url: String) -> Result<()> {
    blocking(move || git::set_remote(Path::new(&root), &url)).await
}

#[tauri::command]
pub async fn git_set_identity(root: String, name: String, email: String) -> Result<()> {
    blocking(move || git::set_identity(Path::new(&root), &name, &email)).await
}

#[tauri::command]
pub async fn git_abort_merge(root: String) -> Result<()> {
    blocking(move || git::abort_merge(Path::new(&root))).await
}

// ---------- MCP server (see crate::mcp) ----------

/// Delivers the MCP hub's requests to the web UI as events.
pub struct TauriFrontend(pub AppHandle);

impl mcp::Frontend for TauriFrontend {
    fn emit(&self, event: &str, payload: serde_json::Value) {
        let _ = self.0.emit(event, payload);
    }
}

#[derive(Debug, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct McpConfig {
    pub enabled: bool,
    pub allow_edits: bool,
    pub root: Option<String>,
    pub name: Option<String>,
    pub main_file: Option<String>,
}

/// The UI reports what the user allows and which project is open.
#[tauri::command]
pub fn mcp_configure(hub: State<'_, Arc<Hub>>, config: McpConfig) {
    let project = config.root.map(|root| mcp::Project {
        root: PathBuf::from(root),
        name: config.name.unwrap_or_default(),
        main_file: config.main_file,
    });
    hub.configure(mcp::Config { enabled: config.enabled, allow_edits: config.allow_edits, project });
}

/// The UI's answer to an `mcp://request` event.
#[tauri::command]
pub fn mcp_respond(hub: State<'_, Arc<Hub>>, id: u64, ok: bool, value: Option<serde_json::Value>, error: Option<String>) {
    hub.respond(
        id,
        if ok { Ok(value.unwrap_or(serde_json::Value::Null)) } else { Err(error.unwrap_or_else(|| "KernTeX couldn't do that.".into())) },
    );
}

#[tauri::command]
pub fn mcp_clients(hub: State<'_, Arc<Hub>>) -> Vec<mcp::ClientInfo> {
    hub.clients()
}

#[tauri::command]
pub fn mcp_disconnect(hub: State<'_, Arc<Hub>>) {
    hub.disconnect_all();
}

/// The `kerntex-mcp` command to give to AI assistants.
#[tauri::command]
pub async fn mcp_setup(app: AppHandle) -> mcp::setup::McpSetup {
    let data = app.path().app_local_data_dir().ok();
    tauri::async_runtime::spawn_blocking(move || mcp::setup::setup(data.as_deref())).await.unwrap_or_else(|e| mcp::setup::McpSetup {
        command: None,
        problem: Some(e.to_string()),
        endpoint: String::new(),
        claude_code: None,
    })
}

/// Opens Cursor's or VS Code's "add MCP server" dialog for `kerntex-mcp`.
#[tauri::command]
pub async fn mcp_install_link(app: AppHandle, target: String) -> Result<()> {
    use tauri_plugin_opener::OpenerExt;
    let data = app.path().app_local_data_dir().ok();
    let setup = tauri::async_runtime::spawn_blocking(move || mcp::setup::setup(data.as_deref()))
        .await
        .map_err(|e| AppError::new(ErrorSource::App, "Internal task failed").detail(e.to_string()))?;
    let command =
        setup.command.ok_or_else(|| AppError::new(ErrorSource::App, "kerntex-mcp wasn't found").hint(setup.problem.unwrap_or_default()))?;
    let link = mcp::setup::install_link(&target, &command)
        .ok_or_else(|| AppError::new(ErrorSource::App, "Unknown AI assistant").detail(target.clone()))?;
    app.opener().open_url(link, None::<&str>).map_err(|e| {
        AppError::new(ErrorSource::App, "The link didn't open")
            .detail(e.to_string())
            .hint("Is the app installed? Otherwise copy the configuration and add it in the app's MCP settings.")
    })
}

/// Runs `claude mcp add --scope user kerntex -- <kerntex-mcp>`.
#[tauri::command]
pub async fn mcp_add_to_claude_code(app: AppHandle) -> Result<String> {
    let data = app.path().app_local_data_dir().ok();
    blocking(move || {
        let setup = mcp::setup::setup(data.as_deref());
        let command = setup
            .command
            .ok_or_else(|| AppError::new(ErrorSource::App, "kerntex-mcp wasn't found").hint(setup.problem.unwrap_or_default()))?;
        mcp::setup::add_to_claude_code(&command).map_err(|e| AppError::new(ErrorSource::App, "Couldn't connect Claude Code").detail(e))
    })
    .await
}
