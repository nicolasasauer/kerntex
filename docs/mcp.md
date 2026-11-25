# AI assistants (MCP server)

KernTeX lets AI assistants — Claude Code, Claude Desktop, Cursor, VS Code and any other [Model Context Protocol](https://modelcontextprotocol.io) client — work on *the project that is open in KernTeX*: read files, make changes you see live and can undo, compile and fix errors. **It is off by default.**

Background, research and design decisions: [mcp-research.md](mcp-research.md).

## Setting it up

1. In KernTeX: click the **robot icon** in the toolbar, or **Command palette (`Ctrl+K`) → “AI assistants”**. The start screen links there too, and the example projects mention it.
2. Click **Connect** next to your assistant:
   - **Claude Code** — KernTeX runs `claude mcp add --scope user kerntex -- <kerntex-mcp>` for you (or copy the command).
   - **Cursor / VS Code** — opens the app's own “Install MCP server” dialog; confirm it there.
   - **Claude Desktop and others** — copy the configuration and paste it into the app's MCP settings (Claude: *Settings → Developer → Edit config*, then restart Claude).
3. Connecting turns on **Allow AI assistants**. That's all — it works with whatever project is open.

While an assistant is connected, the toolbar shows its name. Click it to see what it did, or to disconnect it.

## How it works

```
Agent ──stdio──▶ kerntex-mcp ──named pipe / Unix socket──▶ KernTeX app (rmcp server)
                                                             ├─ core: files on disk
                                                             └─ web UI: editor, undo, compile
```

- **`kerntex-mcp`** (`src-tauri/mcp-proxy`) is a tiny program installed next to KernTeX. Agents start it as a local (stdio) MCP server. It only forwards bytes to the running app, and starts KernTeX if it isn't running.
  - Its path stays stable across updates: next to the app (Windows installer, `.deb`), as the `kerntex-mcp.exe` execution alias (Microsoft Store), or copied to the app's data folder (AppImage).
- **The app** (`src-tauri/src/mcp`) listens on a named pipe (`\\.\pipe\kerntex-mcp-<user>`) or a Unix socket in `$XDG_RUNTIME_DIR` — no network port.
  - Each connection is served by the official Rust SDK, [`rmcp`](https://github.com/modelcontextprotocol/rust-sdk). It speaks both the stateless MCP spec 2026-07-28 and 2025-11-25, which still uses a handshake.
- Requests that must see what the user sees (unsaved text, undoable edits, compiling with live progress) go to the web UI as `mcp://request` events (`src/lib/mcp.svelte.ts`). Everything else uses `core` directly.

## Tools

| Tool | Arguments | Result |
| --- | --- | --- |
| `list_files` | `include_generated?` | Files of the project and the main file |
| `read_file` | `path`, `start_line?`, `end_line?` | Text as the editor shows it (unsaved changes included), `revision`, line count |
| `edit_file` | `path`, `edits: [{ old_text, new_text }]`, `expected_revision?` | Each `old_text` must occur exactly once. Applied in the editor as one undoable step; refused if the file changed since `expected_revision` |
| `create_file` | `path`, `content` | Creates a new file (never overwrites); missing folders are created |
| `search` | `query`, `case_sensitive?`, `max_results?` | Matching lines across text files, including unsaved changes |
| `compile` | — | Saves and compiles like the Compile button; sends progress notifications. Returns `ok`, duration, errors/warnings with file, line and hint |
| `get_diagnostics` | — | The result of the last compile |

- **Results:** every tool declares an `outputSchema` and returns `structuredContent`.
- **Errors:** failures come back as `isError: true` with a plain-language message saying what to do. If MCP is off, no project is open, or edits aren't allowed, the message says so.
- **Hints for clients:** read-only tools carry `readOnlyHint`.

## Safety

- **Off by default.** While off, every tool call only answers “turned off — ask the user to allow it”; no project data is shared.
- **Only the open project.** Every path goes through `project::resolve` (no absolute paths, no `..`), links that lead outside the project are refused, and `.git` and `.kerntex` can't be written.
- **Visible and undoable.** Agent edits appear live in the editor, show a short notice, and are undone with `Ctrl+Z`. They're written to disk with the next save, autosave or compile, like the user's own typing. The activity list in *AI assistants* shows every call.
- **Read-only mode.** *Allow changes to files* off: agents can read and compile, but not edit or create files.
- **No shell, no network.** The tools can't run commands or fetch URLs; `compile` runs the configured engine only. A rate limit caps tool calls per agent.
- **One click to disconnect** from the toolbar or the command palette.

## Building from source

`cargo build --workspace` (in `src-tauri`) builds the app and `kerntex-mcp` next to each other, which is enough for development. Release builds bundle it via `npm run build:mcp`, which puts it at `src-tauri/binaries/kerntex-mcp-<triple>` for Tauri's `externalBin`.

## Later

- `get_pdf_text` to check the result.
- MCP resources with change subscriptions, e.g. for diagnostics.
- MCP Apps: a PDF preview inside the assistant's chat.
