# MCP interface (planned for v0.3)

KernTeX will optionally expose the open project to AI agents through the [Model Context Protocol](https://modelcontextprotocol.io). **It is off by default** and nothing in v0.1 implements it yet; this document fixes the interface so the architecture keeps room for it.

Research and the current implementation plan (November 2026): [mcp-research.md](mcp-research.md).

## Enabling

- **In the app:** *Settings → Enable MCP server*.
- **For agents:** `kerntex mcp` starts a stdio MCP server that connects to the running KernTeX app (local IPC) and serves *the project that is currently open*.

While an agent is connected, the toolbar shows a visible indicator, and it can be disconnected with one click.

## Tools

| Tool | Arguments | Result |
| --- | --- | --- |
| `list_files` | — | File tree of the open project (project-relative paths, `generated` flag for aux files) |
| `read_file` | `path` | UTF-8 text of a file |
| `edit_file` | `path`, `edits: [{ old_text, new_text }]` | Applies targeted replacements; each `old_text` must match exactly once. Returns the new revision. No whole-file overwrites. |
| `compile` | — | Same as the Compile button; returns `ok`, duration and diagnostics |
| `get_diagnostics` | — | Diagnostics of the last compile: severity, source, file, line, message, hint |
| `get_pdf_text` *(later)* | `page?` | Extracted text of the PDF, for checking the result |

All results use the same JSON shapes as the app's Tauri commands (`src/lib/types.ts`).

## Safety

- **Only the open project.** Every path goes through the same `project::resolve` check as the UI: no absolute paths, no `..`, nothing outside the project folder.
- **Edits are visible and undoable.** Agent edits are applied through the editor, appear live, and can be undone with `Ctrl+Z` like the user's own typing. Unsaved editor content is what the agent reads, so the user and the agent never overwrite each other.
- **No shell, no network.** The tools can't run commands or fetch URLs; `compile` runs the configured engine only.

## Implementation notes

The tools map directly onto `core` (`project::read_tree`, `project::read_text`, `compile::compile`, `diagnostics`), which has no Tauri dependency. `edit_file` is routed through the frontend so the editor stays the single source of truth for open files.
