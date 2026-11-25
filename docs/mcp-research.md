# MCP server: research and implementation plan (November 2026)

Research notes for the MCP server, collected on 2026-11-25. They record the state of the protocol and tooling and the design decisions. **Implemented** — see [docs/mcp.md](mcp.md) for what was built; this file stays as the background.

## Starting point

- Nothing of the MCP server is implemented yet.
- The architecture is ready for it: `src-tauri/src/core/` has no Tauri dependency, `project::resolve` rejects paths outside the project, and all planned tools map onto existing functions (`project::read_tree`, `project::read_text`, `compile::compile`, `diagnostics`).
- `read_file` must return unsaved editor content, `edit_file` must be undoable with `Ctrl+Z`, and `compile` must show progress and refresh the PDF in the UI. These requests therefore have to go through the frontend, not straight to `core`.

## What changed in MCP (spec 2026-07-28)

Verified against the changelog in the official spec repository.

- **Stateless core.** The `initialize` / `notifications/initialized` handshake and protocol-level sessions are gone. Every request carries its protocol version and client capabilities in `_meta`. Servers **must** implement the new `server/discover` method.
- **State through handles.** A server that needs state across calls returns its own handle, which the client passes back as an ordinary tool argument (SEP-2567). This matches the planned `revision` on `read_file` / `edit_file`.
- **New required result fields:** `resultType` on all results; `ttlMs` and `cacheScope` on list results. `tools/list` should be returned in a deterministic order.
- **Multi Round-Trip Requests (MRTR)** replace server-initiated requests (elicitation, sampling, roots). The server returns `resultType: "input_required"`, and the client retries with the answers.
- **Tasks** for long-running work moved into an official extension (`io.modelcontextprotocol/tasks`). Progress notifications on the response stream of the running request remain allowed.
- **Change notifications** now go through `subscriptions/listen` instead of `resources/subscribe`.
- **Deprecated:** Roots, Sampling, Logging (on stdio, log to stderr), the HTTP+SSE transport.
- **Tools:** `inputSchema` / `outputSchema` accept any JSON Schema 2020-12; `structuredContent` may be any JSON value. Execution errors are returned as `isError: true` with readable text so the model can correct itself. Servers **must** validate inputs, implement access control, rate-limit tool calls and sanitize outputs.

**Client support is unverified.** We could not confirm whether Claude Desktop, Claude Code or Cursor already speak 2026-07-28. The server must therefore support **both** 2025-11-25 (with handshake) and 2026-07-28 (stateless).

## Decisions

### 1. Use the official Rust SDK `rmcp` instead of writing the protocol ourselves

- `rmcp` 3.11.0 (2026-11-17), maintained by the MCP project and promoted to Tier 1. It supports both spec versions.
- Its dependencies (tokio, serde, futures, tracing) are mostly already pulled in by Tauri, so it adds little weight.
- Tool input and output schemas are generated with `schemars` from the same Rust types the UI already uses, which keeps "same JSON shapes as the app" automatically true.
- Supporting two spec versions by hand would be error-prone; this replaces the earlier idea of a hand-written JSON-RPC layer.

### 2. Architecture: thin stdio proxy, server inside the app

```
Agent ──stdio──▶ kerntex-mcp (small console binary, forwards bytes only)
                    │  interprocess: named pipe (Windows) / Unix socket (Linux)
                    ▼
                 KernTeX app: rmcp server ──▶ frontend (editor, undo) / core
```

- **stdio** is the recommended transport for local servers. JetBrains IDEs follow the same model: the IDE itself serves MCP for the open project.
- **No localhost port**, so no DNS-rebinding risk and no token handling. The pipe / socket is only accessible to the current user. Crate: `interprocess` 2.4 (tokio feature).
- **A separate small binary `kerntex-mcp`** instead of a `kerntex mcp` subcommand:
  - it is a console-subsystem binary, so stdio works on Windows (the app is built with `windows_subsystem = "windows"`);
  - it starts fast without loading the WebView;
  - it can be shipped as a Tauri sidecar (`externalBin`), as an MSIX `AppExecutionAlias` and inside an `.mcpb` bundle.
- The app only opens the socket while *Settings → Enable MCP server* is on.
- If the app is not running, no project is open or MCP is disabled, `kerntex-mcp` returns a clear message saying so (design rule 3).

### 3. Frontend bridge

- Rust emits `mcp://request` with a request id; the frontend handles it and answers through a `mcp_respond` command. A map of pending requests with a timeout sits in between.
- `workspace.compile()` returns its result so `compile` can report it.
- Agent edits to a file that is not open open it as a tab, so every change is visible and undoable.

### 4. Tools

| Tool | Notes |
| --- | --- |
| `list_files` | `readOnlyHint` |
| `read_file` | `readOnlyHint`; returns text and `revision` |
| `edit_file` | `edits: [{ old_text, new_text }]`, optional `expected_revision`; each `old_text` must match exactly once; concurrent changes by the user are detected instead of overwritten |
| `create_file` | new; agents expect it |
| `search` | new; text search across the project |
| `compile` | progress notifications ("Typesetting (pass 2)"); returns `ok`, duration, diagnostics |
| `get_diagnostics` | `readOnlyHint` |
| `get_pdf_text` | later |

- All tools declare `outputSchema` and return `structuredContent`.
- Errors are `isError: true` with source, location and hint, like the rest of KernTeX.
- Use the tasks extension for `compile` only once clients support it. The first Tectonic run can take minutes, so client timeouts need testing.

### 5. Safety

- Every path goes through `project::resolve`; no shell, no network access; `compile` runs the configured engine only.
- A simple rate limit on tool calls.
- A *read-only* switch: the agent may read and compile, but not edit.
- A visible **agent activity log** in the app ("Agent edited `main.tex`"), plus the toolbar indicator with a one-click disconnect.
- Tool annotations are hints for clients, not a security boundary; file contents are untrusted input to the agent (prompt injection through a `.tex` file is possible) and must never be treated as instructions by KernTeX.
- Reference: OWASP MCP Security Cheat Sheet, CSA Agentic MCP Security Best Practices.

### 6. Setup

- In Settings, a button "Connect to Claude Code / Claude Desktop / Cursor" copies the matching configuration, e.g. `claude mcp add kerntex -- kerntex-mcp`.
- Optional: an `.mcpb` bundle for one-click install in Claude Desktop. The official directory no longer accepts MCPB submissions, so it would be distributed by us (website / releases).

### Later, deliberately not in the first version

- MCP resources with change subscriptions (`subscriptions/listen`), e.g. for diagnostics.
- **MCP Apps** (`ui://` resources, extension `io.modelcontextprotocol/ui`): a PDF preview inside the agent's chat.

## Prerequisite: file watcher

Agents such as Claude Code often edit files directly on disk with their own tools, not through KernTeX. Today the editor then keeps showing the old text, and saving would overwrite the agent's change. With MCP this becomes a real data-loss risk, so the file watcher (roadmap v0.2) should land **before or together with** the MCP server.

- Crates: `notify` 9.0 and `notify-debouncer-full` 0.8, both released 2026-11-21. Check the changelog before adopting the new major version, or fall back to the previous one.
- The debouncer merges "write temp file + rename" into a single event — the same pattern our own atomic saves produce.

## Effort

Roughly **5–7 working days**, about 1,200–1,800 lines.

| Part | Effort |
| --- | --- |
| `rmcp` server in the app, tools, schemas | ~1–1.5 days |
| `kerntex-mcp` proxy and local socket / named pipe | ~1–1.5 days |
| Frontend bridge (requests, edits through the editor, compile result) | 1–2 days |
| UI: setting, indicator, activity log, read-only switch, connect button | ~1 day |
| Tests, packaging (sidecar, MSIX alias, AppImage path), docs, PRIVACY.md | 1–2 days |

Most of the time goes into testing with real agents on Windows and Linux and into packaging, not into the code itself.

### Pitfalls to check early

- stdio of the proxy on Windows under MSIX; the AppImage binary path.
- MCP client timeouts during long compiles.
- Concurrent edits by the user and the agent (covered by `expected_revision`).

## Suggested order

1. File watcher
2. MCP server (with `create_file` and `search`)
3. SyncTeX

## What follows afterwards (from the roadmap)

- **Still open from v0.2:** SyncTeX (the engine already writes `.synctex.gz`), file watcher, spell checking.
- **v0.3:** Git panel, alongside MCP; then `get_pdf_text`.
- **v0.4:** device sync and real-time collaboration via a self-hostable `kerntex-sync`.
- **Typst stage 2:** completion from Typst's IDE support, live preview, click in the PDF to jump to the source (#16).
- **Later:** Android / tablet, macOS, code signing, auto-update.
- **Smaller open points:** calmer auto-compile, line numbers for missing packages, overall download progress on the first compile, native menu bar, UI translations.

## Sources

- [MCP spec 2026-07-28 – Key changes](https://modelcontextprotocol.io/specification/2026-07-28/changelog) (read from the [spec repository](https://github.com/modelcontextprotocol/modelcontextprotocol))
- [MCP spec 2025-11-25 – Key changes](https://modelcontextprotocol.io/specification/2025-11-25/changelog)
- [SEP-1686: Tasks](https://modelcontextprotocol.io/seps/1686-tasks)
- [WorkOS: MCP went stateless](https://workos.com/blog/mcp-stateless-spec-2026-07-28)
- [AAIF: Migrate to stateless requests](https://aaif.io/blog/migrate-sessions-to-stateless-requests-with-mcp-2026-07-28)
- [Google Developers Blog: MCP stateless updates](https://developers.googleblog.com/scaling-ai-agent-infrastructure-with-the-mcp-stateless-updates/)
- [Cloudflare: The next generation of MCP](https://blog.cloudflare.com/mcp-v2)
- [modelcontextprotocol/rust-sdk (rmcp)](https://github.com/modelcontextprotocol/rust-sdk), [roadmap](https://github.com/modelcontextprotocol/rust-sdk/blob/main/ROADMAP.md)
- [Rust SDK promoted to Tier 1](https://www.digitalapplied.com/blog/mcp-sdk-conformance-tiers-what-tier-1-means)
- [MCP transports: stdio vs Streamable HTTP](https://restfulapi.net/mcp-transports-stdio-vs-streamable-http/)
- [Security guide: Streamable HTTP vs stdio](https://nhimg.org/streamable-http-vs-stdio-mcp-transport-guide)
- [MCPB (GitHub)](https://github.com/modelcontextprotocol/mcpb), [Build a desktop extension with MCPB](https://claude.com/docs/connectors/building/mcpb), [Anthropic: Desktop Extensions](https://www.anthropic.com/engineering/desktop-extensions)
- [MCP Apps – SEP-1865](https://modelcontextprotocol.io/seps/1865-mcp-apps-interactive-user-interfaces-for-mcp), [MCP Apps blog post (January 2026)](https://blog.modelcontextprotocol.io/posts/2026-01-26-mcp-apps/)
- [OWASP MCP Security Cheat Sheet](https://cheatsheetseries.owasp.org/cheatsheets/MCP%5FSecurity%5FCheat%5FSheet.html)
- [CSA Agentic MCP Security Best Practices](https://labs.cloudsecurityalliance.org/research/agentic-mcp-security-best-practices-v1/)
- [interprocess](https://docs.rs/interprocess), [notify-debouncer-full](https://docs.rs/notify-debouncer-full)
- [hypothesi/mcp-server-tauri](https://github.com/hypothesi/mcp-server-tauri)
- [JetBrains MCP Server](https://heyclau.de/entry/mcp/jetbrains-mcp-server), [Zed discussion: MCP server mode](https://github.com/zed-industries/zed/discussions/46388)
