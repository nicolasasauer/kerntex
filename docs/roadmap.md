# Roadmap & open points

What KernTeX doesn't do yet, roughly in order of how much it would help. The architecture already leaves room for each item (see [DESIGN.md](../DESIGN.md)).

## Usability gaps (known, not yet built)

| Item | Why it matters | Planned |
| --- | --- | --- |
| **SyncTeX** — double-click in the PDF jumps to the source line, Ctrl+click in the source jumps to the PDF | One of the most-used features in Overleaf; finding a spot in a long document is tedious without it. The engine already writes `.synctex.gz`. | v0.2 |
| **File watcher** — notice files changed outside KernTeX (Git pull, another editor, sync tools) | Today the editor keeps showing the old text until the file is reopened; saving could then overwrite the external change. | v0.2 |
| **Spell checking** (German / English) | Expected by anyone writing prose; e.g. LTeX / LanguageTool, optional. | v0.2 |
| **Calmer auto-compile** | Autosave and "compile on save" are both on by default, so every typing pause of 1.5 s compiles. Feels live, but can be restless for large theses. Idea: compile on autosave only when the last build was fast, or a separate "auto-compile" switch. | open |
| **Line numbers for missing packages** | TeX reports a missing `.sty` one line *after* the `\usepackage` line. KernTeX could map it back to the `\usepackage` line. | open |
| **Download progress on the very first compile** | The first Tectonic run shows which package is being downloaded, but no overall progress (%). | open |
| **Native menu bar** (File / Edit / View) | Some users look there first; today everything is in the toolbar, command palette and context menus. | open |
| **UI translations** | The UI is English; strings should be collected so German and other languages can be added. | open |

## Later phases

See the roadmap in the requirements: v0.3 MCP server ([docs/mcp.md](mcp.md)) and Git panel; v0.4 device sync and real-time collaboration; later macOS, Typst, code signing, auto-update.

## Done since v0.1 planning

- Add files by drag & drop onto the file tree (into the folder under the pointer) or via *Add files…*; name clashes become `name-2.ext`, nothing is overwritten.
- Preview for images and PDF figures, with "Insert figure" and "Copy `\includegraphics`".
- *Build details* tab: engine and version, TeX program, LaTeX format, document class, every package with version and date (the ones your document loads, and on request their dependencies), links to the output folder, the full log and the Tectonic package cache.
