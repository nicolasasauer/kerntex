//! MCP server: lets AI agents work on the project that is open in KernTeX
//! (see docs/mcp.md).
//!
//! ```text
//! Agent ──stdio──▶ kerntex-mcp ──named pipe / Unix socket──▶ [`Hub::listen`]
//!                                                              │ one rmcp server per connection
//!                                                              ▼
//!                                   core (files on disk)  +  frontend (editor, undo, compile)
//! ```
//!
//! Requests that must see or change what the user sees — unsaved editor
//! text, edits that can be undone, compiling with live progress — go to the
//! web UI as `mcp://request` events and come back through
//! [`Hub::respond`]. Nothing here depends on Tauri: the UI is reached
//! through the [`Frontend`] trait, so the server can be tested on its own.

mod server;
pub mod setup;
#[cfg(test)]
mod tests;

pub use server::{CompileReport, KernTexServer};

use crate::core::compile::{CompileResult, Progress};
use crate::core::error::AppError;
use serde::Serialize;
use serde_json::{json, Value};
use std::collections::{HashMap, VecDeque};
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use tokio::io::{AsyncRead, AsyncWrite};
use tokio::sync::{broadcast, oneshot};
use tokio_util::sync::CancellationToken;

/// How the hub reaches the web UI.
pub trait Frontend: Send + Sync + 'static {
    fn emit(&self, event: &str, payload: Value);
}

/// The project the user has open.
#[derive(Debug, Clone)]
pub struct Project {
    pub root: PathBuf,
    pub name: String,
    pub main_file: Option<String>,
}

/// What the user allows, plus the open project. Set by the UI.
#[derive(Debug, Clone, Default)]
pub struct Config {
    /// Off by default: until the user turns it on, every tool call explains how.
    pub enabled: bool,
    /// When false the agent may read and compile, but not change files.
    pub allow_edits: bool,
    pub project: Option<Project>,
}

/// A connected agent, as shown in the UI.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ClientInfo {
    pub id: u64,
    pub name: String,
}

struct Client {
    id: u64,
    name: String,
    disconnect: CancellationToken,
    calls: VecDeque<Instant>,
}

/// Tool calls allowed per client within [`RATE_WINDOW`].
const RATE_LIMIT: usize = 300;
const RATE_WINDOW: Duration = Duration::from_secs(60);

pub struct Hub {
    frontend: Box<dyn Frontend>,
    config: Mutex<Config>,
    pending: Mutex<HashMap<u64, oneshot::Sender<Result<Value, String>>>>,
    next_request: AtomicU64,
    clients: Mutex<Vec<Client>>,
    next_client: AtomicU64,
    /// Result of the last compile in this project, with a generation counter
    /// so `compile` can tell whether *its* build has finished.
    last_compile: Mutex<Option<(u64, CompileReport)>>,
    compile_generation: AtomicU64,
    progress: broadcast::Sender<Progress>,
}

impl Hub {
    pub fn new(frontend: impl Frontend) -> Arc<Hub> {
        Arc::new(Hub {
            frontend: Box::new(frontend),
            config: Mutex::default(),
            pending: Mutex::default(),
            next_request: AtomicU64::new(1),
            clients: Mutex::default(),
            next_client: AtomicU64::new(1),
            last_compile: Mutex::default(),
            compile_generation: AtomicU64::new(0),
            progress: broadcast::channel(64).0,
        })
    }

    // ---------- Configuration (from the UI) ----------

    pub fn configure(&self, config: Config) {
        let mut cur = self.config.lock().unwrap();
        let same_project = cur.project.as_ref().map(|p| &p.root) == config.project.as_ref().map(|p| &p.root);
        if !same_project {
            *self.last_compile.lock().unwrap() = None;
        }
        *cur = config;
    }

    pub fn config(&self) -> Config {
        self.config.lock().unwrap().clone()
    }

    // ---------- Requests to the UI ----------

    /// Asks the UI to do `op` and waits for its answer.
    pub async fn request(&self, op: &str, args: Value, timeout: Duration) -> Result<Value, String> {
        let id = self.next_request.fetch_add(1, Ordering::Relaxed);
        let (tx, rx) = oneshot::channel();
        self.pending.lock().unwrap().insert(id, tx);
        self.frontend.emit("mcp://request", json!({ "id": id, "op": op, "args": args }));
        let answer = tokio::time::timeout(timeout, rx).await;
        self.pending.lock().unwrap().remove(&id);
        match answer {
            Ok(Ok(result)) => result,
            Ok(Err(_)) => Err("KernTeX's window closed before it answered.".into()),
            Err(_) => Err(format!(
                "KernTeX's window didn't answer within {} s. Make sure KernTeX is open and not showing a dialog, then try again.",
                timeout.as_secs()
            )),
        }
    }

    /// The UI's answer to [`Hub::request`].
    pub fn respond(&self, id: u64, result: Result<Value, String>) {
        if let Some(tx) = self.pending.lock().unwrap().remove(&id) {
            let _ = tx.send(result);
        }
    }

    // ---------- Compile results and progress (from the compile command) ----------

    pub fn record_compile(&self, result: &Result<CompileResult, AppError>) {
        let root = self.config().project.map(|p| p.root);
        let report = CompileReport::new(result, root.as_deref());
        let generation = self.compile_generation.fetch_add(1, Ordering::SeqCst) + 1;
        *self.last_compile.lock().unwrap() = Some((generation, report));
    }

    pub fn compile_generation(&self) -> u64 {
        self.compile_generation.load(Ordering::SeqCst)
    }

    /// The last compile result, if it is newer than `generation`.
    pub fn compile_since(&self, generation: u64) -> Option<CompileReport> {
        self.last_compile.lock().unwrap().as_ref().filter(|(g, _)| *g > generation).map(|(_, r)| r.clone())
    }

    pub fn last_compile(&self) -> Option<CompileReport> {
        self.compile_since(0)
    }

    pub fn compile_progress(&self, p: &Progress) {
        let _ = self.progress.send(p.clone());
    }

    fn subscribe_progress(&self) -> broadcast::Receiver<Progress> {
        self.progress.subscribe()
    }

    // ---------- Connected agents ----------

    pub fn clients(&self) -> Vec<ClientInfo> {
        self.clients.lock().unwrap().iter().map(|c| ClientInfo { id: c.id, name: c.name.clone() }).collect()
    }

    /// Ends every agent connection (the toolbar's "Disconnect").
    pub fn disconnect_all(&self) {
        for c in self.clients.lock().unwrap().iter() {
            c.disconnect.cancel();
        }
    }

    fn register(&self) -> (u64, CancellationToken) {
        let id = self.next_client.fetch_add(1, Ordering::Relaxed);
        let disconnect = CancellationToken::new();
        self.clients.lock().unwrap().push(Client {
            id,
            name: "AI assistant".into(),
            disconnect: disconnect.clone(),
            calls: VecDeque::new(),
        });
        self.emit_clients();
        (id, disconnect)
    }

    fn unregister(&self, id: u64) {
        self.clients.lock().unwrap().retain(|c| c.id != id);
        self.emit_clients();
    }

    fn name_client(&self, id: u64, name: &str) {
        let changed = {
            let mut clients = self.clients.lock().unwrap();
            match clients.iter_mut().find(|c| c.id == id) {
                Some(c) if c.name != name => {
                    c.name = name.to_string();
                    true
                }
                _ => false,
            }
        };
        if changed {
            self.emit_clients();
        }
    }

    fn client_name(&self, id: u64) -> String {
        self.clients.lock().unwrap().iter().find(|c| c.id == id).map(|c| c.name.clone()).unwrap_or_else(|| "AI assistant".into())
    }

    /// Sliding-window rate limit per client. False when the limit is hit.
    fn allow_call(&self, id: u64) -> bool {
        let mut clients = self.clients.lock().unwrap();
        let Some(c) = clients.iter_mut().find(|c| c.id == id) else { return true };
        let now = Instant::now();
        while c.calls.front().is_some_and(|t| now.duration_since(*t) > RATE_WINDOW) {
            c.calls.pop_front();
        }
        if c.calls.len() >= RATE_LIMIT {
            return false;
        }
        c.calls.push_back(now);
        true
    }

    fn emit_clients(&self) {
        self.frontend.emit("mcp://clients", json!(self.clients()));
    }

    /// Tells the UI what an agent did, for the activity list.
    fn activity(&self, client: u64, tool: &str, detail: &str, error: Option<&str>) {
        self.frontend.emit(
            "mcp://activity",
            json!({ "client": self.client_name(client), "tool": tool, "detail": detail, "ok": error.is_none(), "error": error }),
        );
    }

    // ---------- Connections ----------

    /// Serves one agent connection until it closes or the user disconnects it.
    pub async fn serve_connection<T>(self: Arc<Self>, io: T)
    where
        T: AsyncRead + AsyncWrite + Send + Unpin + 'static,
    {
        let (id, disconnect) = self.register();
        let server = KernTexServer::new(self.clone(), id);
        match rmcp::service::serve_server_with_ct(server, io, disconnect.clone()).await {
            Ok(running) => {
                if let Some(info) = running.peer().peer_info() {
                    self.name_client(id, &server::display_name(&info.client_info));
                }
                let _ = running.waiting().await;
            }
            Err(e) => eprintln!("KernTeX MCP: a connection failed to start: {e}"),
        }
        self.unregister(id);
    }

    /// Accepts agent connections from `kerntex-mcp` for the lifetime of the app.
    ///
    /// The socket is open even while MCP is turned off, so an agent gets a
    /// clear "turn it on in KernTeX" answer instead of a connection error.
    pub async fn listen(self: Arc<Self>) {
        use interprocess::local_socket::tokio::prelude::*;
        use interprocess::local_socket::ListenerOptions;

        let endpoint = match kerntex_mcp::Endpoint::for_current_user() {
            Ok(e) => e,
            Err(e) => return eprintln!("KernTeX MCP: no place to listen: {e}"),
        };
        let listener = match bind(&endpoint) {
            Ok(l) => l,
            Err(e) => {
                return eprintln!("KernTeX MCP: not listening at {} ({e}). Is another KernTeX window already running?", endpoint.display())
            }
        };
        loop {
            match listener.accept().await {
                Ok(conn) => {
                    tokio::spawn(self.clone().serve_connection(conn));
                }
                Err(e) => {
                    eprintln!("KernTeX MCP: accepting a connection failed: {e}");
                    tokio::time::sleep(Duration::from_millis(200)).await;
                }
            }
        }

        fn bind(endpoint: &kerntex_mcp::Endpoint) -> std::io::Result<interprocess::local_socket::tokio::Listener> {
            let create = || ListenerOptions::new().name(endpoint.name()?).create_tokio();
            match create() {
                Err(e) if e.kind() == std::io::ErrorKind::AddrInUse => {
                    // A socket file is left over. If nobody answers on it, the app
                    // that made it is gone: remove it and try again. If someone
                    // answers, another KernTeX is serving — leave it alone.
                    let kerntex_mcp::Endpoint::Path(path) = endpoint else { return Err(e) };
                    if <interprocess::local_socket::Stream as interprocess::local_socket::traits::Stream>::connect(endpoint.name()?).is_ok()
                    {
                        return Err(e);
                    }
                    std::fs::remove_file(path)?;
                    create()
                }
                other => other,
            }
        }
    }
}
