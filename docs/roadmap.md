# Roadmap & open points

What KernTeX doesn't do yet, roughly in order of how much it would help. The architecture already leaves room for each item (see [DESIGN.md](../DESIGN.md)).

## Usability gaps (known, not yet built)

| Item | Why it matters | Planned |
| --- | --- | --- |
| **SyncTeX** — double-click in the PDF jumps to the source line, Ctrl+click in the source jumps to the PDF | One of the most-used features in Overleaf; finding a spot in a long document is tedious without it. The engine already writes `.synctex.gz`. | v0.2 |
| **File watcher** — notice files changed outside KernTeX (Git pull, another editor, sync tools) | Today the editor keeps showing the old text until the file is reopened; saving could then overwrite the external change. | v0.2 |
| **Spell checking** (German / English) | Expected by anyone writing prose; e.g. LTeX / LanguageTool, optional. | v0.2 |
| **Download progress on the very first compile** | The first Tectonic run shows which package is being downloaded, but no overall progress (%). | open |
| **Native menu bar** (File / Edit / View) | Some users (especially on macOS) look there first; today there is a File button in the toolbar, plus the command palette and context menus. | open |
| **UI translations** | The UI is English; strings should be collected so German and other languages can be added. | open |

## Later phases

All tracked as [GitHub issues](https://github.com/nicolasasauer/kerntex/issues), labelled by version.

- **v0.2** — SyncTeX, file watcher, spell checking
- **v0.3** — Git panel (#13; first step done, see below). Next: sign-in prompt for tokens (`GIT_ASKPASS`, #33), clone from URL, "Sign in with GitHub" / "Publish to GitHub"; later optional local-LLM commit messages (#26). The MCP server for AI assistants is done: [docs/mcp.md](mcp.md)
- **v0.4** — device sync and real-time collaboration via a self-hostable server (`kerntex-sync`)
- **Microsoft Store** — MSIX package is built by the release workflow; see [docs/microsoft-store.md](microsoft-store.md)
- **Android / tablet** — same app on Android tablets and phones (Tauri 2 supports Android). Goal: **compile on the device**, so no server is required. A self-hosted server (the v0.4 relay) is an optional extra for syncing and for devices that can't compile locally — never a requirement, and never a server you don't control.
- **Typst** — stage 1 done (built-in compiler, templates, Typst tour, highlighting, exact error positions, `@`/`#` completion). Stage 2: completion and tooltips from Typst's own IDE support, live preview while typing, click in the PDF to jump to the source (#16)
- **Later** — macOS, code signing, auto-update

## Done since v0.1 planning

- **Calmer auto-compile** (#4): autosave compiles only while builds take under 5 s; longer documents compile on `Ctrl+S` or *Compile*. Setting *Compile on autosave*: only for quick documents / always / never.
- **Missing packages point at `\usepackage`** (#5): the error jumps to the package name in the `\usepackage` / `\documentclass` line, not the line after it.
- **Git panel, step 1** (#13): *Git* tab in the sidebar with branch, ahead/behind, changed files with a diff view, rule-based commit message suggestion (names the changed `\section`s), *Commit & Push* (commit → pull → push) and *Commit*, pull, set up Git for a project (with `.gitignore` for `.kerntex/out/`), add a remote URL, set name/email, merge conflicts listed with *Abort merge*. Uses the system `git`; pull/push show progress and can be cancelled.
- MCP server: AI assistants (Claude Code, Claude Desktop, Cursor, VS Code …) can read, edit (undoably), search and compile the open project; one-click setup, activity list, read-only mode. See [docs/mcp.md](mcp.md).
- Add files by drag & drop onto the file tree (into the folder under the pointer) or via *Add files…*; name clashes become `name-2.ext`, nothing is overwritten.
- Preview for images and PDF figures, with "Insert figure" and "Copy `\includegraphics`".
- *Build details* tab: engine and version, TeX program, LaTeX format, document class, every package with version and date (the ones your document loads, and on request their dependencies), links to the output folder, the full log and the Tectonic package cache.
