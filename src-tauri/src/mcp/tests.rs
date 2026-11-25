//! End to end: a real MCP client talks to the server over an in-memory
//! stream; a fake frontend plays the editor.

use super::*;
use crate::core::buildinfo::BuildDetails;
use crate::core::compile::CompileResult;
use crate::core::diagnostics::{Diagnostic, Severity};
use rmcp::model::{CallToolRequestParams, CallToolResult};
use rmcp::service::RunningService;
use rmcp::{RoleClient, ServiceExt};
use std::path::Path;
use tokio::sync::mpsc;

/// Open editor buffers of the fake UI: path → text.
type Buffers = Arc<Mutex<HashMap<String, String>>>;

struct FakeFrontend(mpsc::UnboundedSender<(String, Value)>);

impl Frontend for FakeFrontend {
    fn emit(&self, event: &str, payload: Value) {
        let _ = self.0.send((event.to_string(), payload));
    }
}

struct Harness {
    hub: Arc<Hub>,
    client: RunningService<RoleClient, ()>,
    buffers: Buffers,
    root: PathBuf,
    events: Arc<Mutex<Vec<(String, Value)>>>,
}

fn temp_project(name: &str) -> PathBuf {
    let root = std::env::temp_dir().join(format!("kerntex-mcp-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(root.join("chapters")).unwrap();
    std::fs::write(
        root.join("main.tex"),
        "\\documentclass{article}\n\\begin{document}\nHello world.\n\\input{chapters/intro}\n\\end{document}\n",
    )
    .unwrap();
    std::fs::write(root.join("chapters/intro.tex"), "An introduction.\n").unwrap();
    std::fs::write(root.join("main.aux"), "\\relax\n").unwrap();
    root
}

/// Answers `mcp://request` events like the web UI does.
fn handle(hub: &Hub, buffers: &Buffers, root: &Path, op: &str, args: &Value) -> Result<Value, String> {
    let path = args.get("path").and_then(Value::as_str).unwrap_or_default().to_string();
    match op {
        "read" => Ok(match buffers.lock().unwrap().get(&path) {
            Some(t) => json!({ "text": t, "dirty": true }),
            None => json!({ "text": null }),
        }),
        "apply" => {
            let base = args["base"].as_str().unwrap();
            let mut b = buffers.lock().unwrap();
            let current = match b.get(&path) {
                Some(t) => t.clone(),
                None => std::fs::read_to_string(root.join(&path)).unwrap(),
            };
            if current != base {
                return Err("changed meanwhile".into());
            }
            b.insert(path, args["text"].as_str().unwrap().to_string());
            Ok(json!({}))
        }
        "buffers" => Ok(json!({ "buffers": *buffers.lock().unwrap() })),
        "created" => Ok(json!({})),
        "compile" => {
            hub.record_compile(&Ok(CompileResult {
                ok: false,
                canceled: false,
                pdf_path: Some(root.join(".kerntex/out/main.pdf").display().to_string()),
                diagnostics: vec![
                    Diagnostic {
                        severity: Severity::Warning,
                        source: "LaTeX".into(),
                        file: Some("main.tex".into()),
                        line: Some(3),
                        column: None,
                        end_line: None,
                        end_column: None,
                        message: "Overfull \\hbox".into(),
                        hint: None,
                    },
                    Diagnostic {
                        severity: Severity::Error,
                        source: "LaTeX".into(),
                        file: Some("main.tex".into()),
                        line: Some(4),
                        column: None,
                        end_line: None,
                        end_column: None,
                        message: "Undefined control sequence".into(),
                        hint: Some("Check the spelling of the command.".into()),
                    },
                ],
                duration_ms: 1234,
                log: String::new(),
                engine_label: "Tectonic".into(),
                details: BuildDetails::default(),
            }));
            Ok(json!({}))
        }
        other => Err(format!("unknown op {other}")),
    }
}

async fn harness(name: &str, enabled: bool, allow_edits: bool) -> Harness {
    let root = temp_project(name);
    let (tx, mut rx) = mpsc::unbounded_channel();
    let hub = Hub::new(FakeFrontend(tx));
    hub.configure(Config {
        enabled,
        allow_edits,
        project: Some(Project { root: root.clone(), name: "thesis".into(), main_file: Some("main.tex".into()) }),
    });
    let buffers: Buffers = Arc::default();
    let events: Arc<Mutex<Vec<(String, Value)>>> = Arc::default();
    {
        let (hub, buffers, root, events) = (hub.clone(), buffers.clone(), root.clone(), events.clone());
        tokio::spawn(async move {
            while let Some((event, payload)) = rx.recv().await {
                if event == "mcp://request" {
                    let id = payload["id"].as_u64().unwrap();
                    let result = handle(&hub, &buffers, &root, payload["op"].as_str().unwrap(), &payload["args"]);
                    hub.respond(id, result);
                } else {
                    events.lock().unwrap().push((event, payload));
                }
            }
        });
    }
    let (server_io, client_io) = tokio::io::duplex(64 * 1024);
    tokio::spawn(hub.clone().serve_connection(server_io));
    let client = ().serve(client_io).await.expect("client connects");
    Harness { hub, client, buffers, root, events }
}

impl Harness {
    async fn call(&self, tool: &'static str, args: Value) -> CallToolResult {
        let params = CallToolRequestParams::new(tool).with_arguments(args.as_object().cloned().unwrap_or_default());
        self.client.call_tool(params).await.expect("tool call")
    }

    async fn ok(&self, tool: &'static str, args: Value) -> Value {
        let r = self.call(tool, args).await;
        assert_ne!(r.is_error, Some(true), "{tool} failed: {:?}", r.content);
        r.structured_content.expect("structured content")
    }

    async fn err(&self, tool: &'static str, args: Value) -> String {
        let r = self.call(tool, args).await;
        assert_eq!(r.is_error, Some(true), "{tool} should fail: {:?}", r.structured_content);
        r.content.iter().filter_map(|c| c.as_text().map(|t| t.text.clone())).collect::<Vec<_>>().join("\n")
    }
}

impl Drop for Harness {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.root);
    }
}

#[tokio::test]
async fn lists_all_tools_with_schemas_and_hints() {
    let h = harness("tools", true, true).await;
    let tools = h.client.list_all_tools().await.unwrap();
    let names: Vec<_> = tools.iter().map(|t| t.name.to_string()).collect();
    assert_eq!(names, ["compile", "create_file", "edit_file", "get_diagnostics", "list_files", "read_file", "search"]);
    for t in &tools {
        assert!(t.description.as_ref().is_some_and(|d| !d.is_empty()), "{} has no description", t.name);
        assert!(t.output_schema.is_some(), "{} has no output schema", t.name);
    }
    let read_only = |name: &str| tools.iter().find(|t| t.name == name).unwrap().annotations.as_ref().unwrap().read_only_hint;
    assert_eq!(read_only("read_file"), Some(true));
    assert_eq!(read_only("edit_file"), Some(false));
    let info = h.client.peer_info().unwrap();
    assert_eq!(info.server_info.as_ref().unwrap().name, "kerntex");
    assert!(info.instructions.as_ref().unwrap().contains("edit_file"));
}

#[tokio::test]
async fn explains_how_to_turn_it_on_while_disabled() {
    let h = harness("disabled", false, true).await;
    let msg = h.err("list_files", json!({})).await;
    assert!(msg.contains("turned off"), "{msg}");
    // Nothing from the project leaked.
    assert!(!msg.contains("main.tex"));
}

#[tokio::test]
async fn lists_and_reads_files_including_unsaved_changes() {
    let h = harness("read", true, true).await;
    let list = h.ok("list_files", json!({})).await;
    assert_eq!(list["main_file"], "main.tex");
    let paths: Vec<_> = list["files"].as_array().unwrap().iter().map(|f| f["path"].as_str().unwrap().to_string()).collect();
    assert_eq!(paths, ["chapters/intro.tex", "main.tex"], "aux files are hidden by default");

    let disk = h.ok("read_file", json!({ "path": "main.tex" })).await;
    assert_eq!(disk["unsaved_changes"], false);
    assert!(disk["text"].as_str().unwrap().contains("Hello world."));
    assert_eq!(disk["total_lines"], 5);

    h.buffers.lock().unwrap().insert("main.tex".into(), "unsaved\ntext\n".into());
    let open = h.ok("read_file", json!({ "path": "./main.tex", "start_line": 2, "end_line": 2 })).await;
    assert_eq!(open["text"], "text\n");
    assert_eq!(open["unsaved_changes"], true);
    assert_ne!(open["revision"], disk["revision"]);

    let msg = h.err("read_file", json!({ "path": "../secret.txt" })).await;
    assert!(msg.contains("outside the project"), "{msg}");
}

#[tokio::test]
async fn edits_through_the_editor_and_detects_concurrent_changes() {
    let h = harness("edit", true, true).await;
    let read = h.ok("read_file", json!({ "path": "main.tex" })).await;
    let rev = read["revision"].as_str().unwrap().to_string();
    let out = h
        .ok("edit_file", json!({ "path": "main.tex", "expected_revision": rev, "edits": [{ "old_text": "Hello world.", "new_text": "Hello KernTeX." }] }))
        .await;
    assert_eq!(out["edits_applied"], 1);
    let buffer = h.buffers.lock().unwrap().get("main.tex").cloned().unwrap();
    assert!(buffer.contains("Hello KernTeX."));
    // The disk is untouched until the user (or autosave) saves.
    assert!(std::fs::read_to_string(h.root.join("main.tex")).unwrap().contains("Hello world."));

    // The old revision is stale now.
    let msg = h
        .err("edit_file", json!({ "path": "main.tex", "expected_revision": rev, "edits": [{ "old_text": "KernTeX", "new_text": "x" }] }))
        .await;
    assert!(msg.contains("changed since you read it"), "{msg}");
    let msg = h.err("edit_file", json!({ "path": "main.tex", "edits": [{ "old_text": "nope", "new_text": "x" }] })).await;
    assert!(msg.contains("wasn't found"), "{msg}");
}

#[tokio::test]
async fn creates_files_but_never_overwrites_or_touches_managed_folders() {
    let h = harness("create", true, true).await;
    let out = h.ok("create_file", json!({ "path": "chapters/methods.tex", "content": "Methods.\n" })).await;
    assert_eq!(out["path"], "chapters/methods.tex");
    assert_eq!(std::fs::read_to_string(h.root.join("chapters/methods.tex")).unwrap(), "Methods.\n");
    assert!(h.err("create_file", json!({ "path": "main.tex", "content": "" })).await.contains("already exists"));
    assert!(h.err("create_file", json!({ "path": ".git/config", "content": "" })).await.contains("manages"));
    assert!(h.err("create_file", json!({ "path": "/etc/x", "content": "" })).await.contains("outside the project"));
}

#[tokio::test]
async fn read_only_mode_allows_reading_but_not_writing() {
    let h = harness("readonly", true, false).await;
    h.ok("read_file", json!({ "path": "main.tex" })).await;
    let msg = h.err("edit_file", json!({ "path": "main.tex", "edits": [{ "old_text": "Hello", "new_text": "Bye" }] })).await;
    assert!(msg.contains("read-only"), "{msg}");
    assert!(h.err("create_file", json!({ "path": "new.tex" })).await.contains("read-only"));
}

#[tokio::test]
async fn searches_disk_and_unsaved_buffers() {
    let h = harness("search", true, true).await;
    h.buffers.lock().unwrap().insert("chapters/intro.tex".into(), "An unsaved INTRODUCTION.\n".into());
    let out = h.ok("search", json!({ "query": "introduction" })).await;
    let hits = out["hits"].as_array().unwrap();
    assert_eq!(hits.len(), 1);
    assert_eq!(hits[0]["path"], "chapters/intro.tex");
    assert_eq!(hits[0]["text"], "An unsaved INTRODUCTION.");
    let out = h.ok("search", json!({ "query": "document", "max_results": 1 })).await;
    assert_eq!(out["truncated"], true);
}

#[tokio::test]
async fn compiles_and_reports_diagnostics_errors_first() {
    let h = harness("compile", true, true).await;
    assert!(h.err("get_diagnostics", json!({})).await.contains("compile first"));
    let report = h.ok("compile", json!({})).await;
    assert_eq!(report["errors"], 1);
    assert_eq!(report["warnings"], 1);
    assert_eq!(report["diagnostics"][0]["severity"], "error");
    assert_eq!(report["diagnostics"][0]["hint"], "Check the spelling of the command.");
    assert_eq!(report["pdf"], ".kerntex/out/main.pdf");
    let again = h.ok("get_diagnostics", json!({})).await;
    assert_eq!(again, report);
}

#[tokio::test]
async fn reports_clients_and_activity_to_the_ui_and_disconnects() {
    let h = harness("activity", true, true).await;
    h.ok("read_file", json!({ "path": "main.tex" })).await;
    assert_eq!(h.hub.clients().len(), 1);
    {
        let events = h.events.lock().unwrap();
        assert!(events.iter().any(|(e, _)| e == "mcp://clients"));
        let activity = events.iter().find(|(e, _)| e == "mcp://activity").expect("activity event");
        assert_eq!(activity.1["tool"], "read_file");
        assert_eq!(activity.1["detail"], "main.tex");
        assert_eq!(activity.1["ok"], true);
    }
    h.hub.disconnect_all();
    for _ in 0..50 {
        if h.hub.clients().is_empty() {
            break;
        }
        tokio::time::sleep(Duration::from_millis(20)).await;
    }
    assert!(h.hub.clients().is_empty(), "disconnect ends the connection");
}
