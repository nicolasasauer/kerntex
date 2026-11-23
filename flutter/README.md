# KernTeX — Flutter UI (preview)

An alternative front end for KernTeX, written in Flutter, on **the same Rust
core** as the Tauri desktop app. It exists to answer a concrete question with
numbers instead of opinions: *is Flutter leaner or faster than Tauri for
KernTeX?* (Results [below](#measurements).)

![KernTeX Flutter UI: the Typst tour with project-wide @ completion, a precise error underline and the PDF rendered by the Rust core](screenshot.png)

Nothing in `src-tauri/src/core` was changed for it; both apps compile the very
same files.

## How it fits together

```
flutter/
├── lib/                Dart UI (no state-management or editor packages)
│   ├── main.dart
│   └── src/
│       ├── native.dart        FFI bindings + two worker isolates (build, render)
│       ├── state.dart         one ChangeNotifier: project, tabs, build, settings
│       ├── editor/            KernTeX's own code editor (see below)
│       ├── pdf_view.dart      preview: pages rasterised by Rust, on-demand
│       ├── workspace.dart     toolbar, file tree, tabs, problems, status bar
│       ├── start_screen.dart  and dialogs.dart, theme.dart
├── native/             Rust cdylib: a ~250-line C ABI over src-tauri/src/core
└── hook/build.dart     Dart build hook: runs cargo, bundles the library
```

* **No code generator.** The bridge is four C functions: `kt_call(method, json)`
  returns `{"ok": …}` or `{"err": AppError}`; `kt_cancel` and `kt_progress` are
  cheap and callable while a build runs; `kt_pdf_render` returns an RGBA buffer.
  The JSON is exactly what the Tauri commands exchange, so both UIs speak the
  same protocol.
* **No CMake glue.** `hook/build.dart` is a Dart build hook (`package:hooks`
  2.x / `code_assets` 2.x, stable since Dart 3.10, current in Dart 3.14):
  `flutter run` and `flutter build` compile the Rust crate with cargo for the
  target platform and bundle `kerntex_native` automatically (Linux, Windows;
  macOS/Android targets are mapped but untested). `code_assets` checks that the
  library was built for the target architecture. Hooks run with a filtered
  environment, so options come from `pubspec.yaml` *user-defines*:
  `cargo_profile: release` (LTO, what ships) or `fast` (no LTO, about half the
  build time for local work). `CARGO_TARGET_DIR` passes through, which lets CI
  cache the Rust build.
* **The UI thread never blocks.** Two long-lived isolates call into Rust: one
  for builds and file-system work, one for PDF pages, so a running build never
  stalls scrolling.
* **PDF without a PDF engine in the UI.** Pages are rasterised in Rust by
  [hayro](https://github.com/LaurenzV/hayro) at exactly the on-screen
  resolution. hayro is already part of Typst's dependency tree, so this adds
  no code to the binary. One page renders at a time, the visible page first;
  only ~10 bitmaps stay in memory.
* **Its own editor**, built for LaTeX and Typst rather than wrapped around a
  generic component — see [The editor](#the-editor).

Dependencies beyond Flutter itself: `ffi`, `file_selector` (native file
dialogs), `hooks`/`code_assets` (the build hook) — all published by the Dart and
Flutter teams.

## The editor

`lib/src/editor/` — about 2 800 lines of Dart (plus generated vocabulary), no editor package.

| File | What it does |
|---|---|
| `document.dart` | The text as a list of lines, selection, grouped undo/redo, diagnostics that move with the text, word and bracket logic |
| `highlight.dart` | Line-by-line lexers for LaTeX, Typst and BibTeX with carried state (math mode, block comments, raw blocks, verbatim) |
| `editor_view.dart` | Virtualised rendering, text input, keyboard and mouse |
| `completion.dart` | Context-aware completion |
| `find_bar.dart` | Find / replace |
| `vocabulary.dart` | Commands, environments, Typst functions and packages — generated from the Tauri editor's data by `tool/gen_vocabulary.mjs` |

**Why it is fast**

* *Only the screen is laid out.* Each line caches its own `ui.Paragraph`; a
  frame draws the ~40 visible ones. Scrolling never re-lays-out text, typing
  re-lays-out one line. The cache is capped (800 lines), so paging through a
  huge file keeps memory flat.
* *Edits are local.* The document is a list of lines; typing changes one
  string, Enter inserts one list entry. Nothing re-joins or re-scans the file.
* *Highlighting is incremental.* After an edit only the changed lines are
  re-lexed, plus following lines until one ends in the same state as before
  (e.g. typing `$` re-lexes until the math mode settles).

Measured (`flutter test`, 100 000-line LaTeX file): opening and highlighting
≈ 100 ms; **≈ 11 µs per keystroke** including undo bookkeeping and
re-highlighting.

**Text input** goes through Flutter's own `TextInputClient` — the interface
Flutter's text fields use — so keyboard layouts, dead keys, compose sequences
and IMEs (Chinese, Japanese, emoji) work as in any Flutter app. The platform
only ever sees the current line; the editor diffs what comes back and applies
it as an edit. Tested in the real app: umlauts, ß, €, typographic quotes,
漢字 and emoji.

**What it does**

* Selection with mouse (click, drag with auto-scroll, double-click word,
  triple-click line, Shift+click) and keyboard (Shift+arrows, Ctrl+Shift+arrows,
  Home/End, Page Up/Down, Ctrl+A, Ctrl+L)
* Smart Home (indentation first), word-wise motion and deletion, soft tabs
* Enter keeps the indentation, opens a block between brackets, and after
  `\begin{env}` adds the matching `\end{env}`
* Auto-closing `( [ {` (and `$` in Typst), typing over the closing bracket,
  wrapping a selection, deleting an empty pair at once; matching bracket
  highlighted
* Tab / Shift+Tab indent and outdent, Ctrl+/ comment toggle, Alt+↑/↓ move
  lines, Ctrl+Shift+K delete line, Ctrl+X/C on an empty selection cut/copy the
  line
* Undo/redo grouped like typing feels (a burst of typing is one step,
  including inside auto-closed braces); "unsaved" is exact: undoing back to the
  saved text clears it
* Other occurrences of the selected word highlighted
* Diagnostics: wavy underline at the exact span (Typst) or line (TeX), error
  background, gutter dot — and they stay on their text while you edit above
  them
* **Completion** (as you type, or Ctrl+Space): LaTeX commands and your own
  `\newcommand`s, environments with bodies (`\begin{figure}` → full figure
  skeleton), `\ref`/`\cref`/… labels and `\cite`/`\parencite`/… keys from the
  whole project, `\usepackage` names with descriptions, file names for
  `\includegraphics`, `\input`, `\include`, `\addbibresource`; in Typst `#`
  functions, `@` labels and citations, and file names for `#image`/`#include`/…
* **Find / replace** (Ctrl+F, Ctrl+H, F3): match case, whole word, regular
  expressions with `$1` groups in the replacement, "n of m", all matches
  highlighted, replace all as one undo step
* Right-click menu, IME candidate window placed at the caret

## Run it

Needs Flutter 3.50+ and Rust (stable). On Linux also `ninja-build libgtk-3-dev clang cmake pkg-config`.

```sh
cd flutter
flutter run -d linux        # or: -d windows
flutter build linux --release
```

The first build compiles the Rust core with LTO and takes a few minutes.
Tectonic is found like in the Tauri app: `KERNTEX_TECTONIC`, then next to the
executable, then `PATH`; without it, LaTeX projects use latexmk. Typst needs
nothing.

Checks: `flutter analyze`, `flutter test`, and in `native/`
`cargo clippy --all-targets -- -D warnings` and `cargo test`. CI:
`.github/workflows/flutter.yml` builds Linux and Windows bundles (with
Tectonic) and uploads them as artifacts.

## What works

Start screen (new project from all LaTeX and Typst templates, open folder,
import Overleaf ZIP, LaTeX example, Typst tour, recent projects) · file tree
(new file/folder, add files, rename, trash, set main file, reveal) · tabs with
unsaved markers · the editor above · compile with progress and Stop (Tectonic,
latexmk, built-in Typst) · Problems with hints and jump-to-source · Build
details (engine, document class, packages) · raw log (virtualised) · PDF
preview with zoom (buttons, Ctrl+wheel) and page indicator · image and PDF file
previews · export PDF · settings (theme, font size, compile on save, engines) ·
About · light and dark theme · keyboard shortcuts as in the Tauri app.

Not (yet) ported from the Tauri app: command palette, drag & drop from the file
manager, export ZIP, the missing-package card with one-click fixes. Not yet in
the Flutter editor (CodeMirror has them): soft wrap (long lines scroll
horizontally here), code folding, multiple cursors.

## Rendering corpus

Does the preview draw real documents correctly? `native/examples/corpus.rs`
compiles public LaTeX and Typst documents with the KernTeX core and compares
every page hayro draws with Poppler (`pdftoppm`), the renderer behind most
Linux PDF viewers.

```sh
cd flutter/native
cargo run --release --example corpus                 # all documents
cargo run --release --example corpus -- --only typst # a subset
```

The documents are listed in `native/corpus/corpus.json` and fetched with git at
pinned commits (nothing third-party is stored in this repository): the official
Typst templates (IEEE paper, AMS article, book, newsletter with CeTZ drawings,
letter), LaTeX's own documents (sample, user guide, LaTeX News, the amsmath
guide), two Beamer talks, *The Not So Short Introduction to LaTeX*, 64 TikZ
figures, and KernTeX's own two tours. Typst Universe packages are installed
from the `typst/packages` repository, so no access to packages.typst.org is
needed. A page counts as different where more than 5 % of its inked pixels
differ by more than 80/255, ignoring anti-aliasing and one-pixel shifts. Output:
`target/corpus/report.md` and side-by-side images (hayro | Poppler | red
differences) of each document's least similar page. CI:
`.github/workflows/corpus.yml` runs it with Tectonic, weekly and whenever the
core or renderer changes, and shows the report as the job summary.

Last local run (TeX Live 2023 via latexmk, 100 dpi): **79 documents, 221
pages — 76 identical within tolerance, 2 flagged, 1 not compiled.**

* All Typst documents and all LaTeX documents (sample2e, usrguide, LaTeX News,
  amsldoc, both Beamer talks, the tours) match Poppler: mean difference ≤ 0.4 %.
* The two flagged TikZ figures have the same content, but hayro draws
  *hairlines* (strokes thinner than a pixel) darker than Poppler, which
  thins them — visible in figures with hundreds of 0.1 pt lines. Both are
  valid; noted for a possible renderer setting.
* *lshort* needs LuaLaTeX, whose font database is incomplete in this
  container's TeX Live; CI installs it properly.
* Speed: hayro renders a page in 1–26 ms at 100 dpi (median ≈ 7 ms).

## Measurements

Same machine, same document (the Typst tour, compiled, first PDF page
visible), Linux, software rendering (Xvfb, no GPU), Rust core built with the same
release profile. Memory is **PSS** (shared pages split fairly) summed over all
of the app's processes, measured 10 s after start.

| | Tauri (WebKitGTK) | Flutter |
|---|---|---|
| Processes | 3 (app, WebKit web, WebKit network) | 1 |
| Memory, fresh start | **377 MB** (98 + 258 + 21) | **312 MB** |
| Memory after the LaTeX example, then the Typst tour | 636 MB in the WebKit web process alone | 342 MB |
| Typst rebuild after an edit | ~0.0–0.1 s | ~0.0–0.1 s (same core) |
| **100 000-line `.tex` (7 MB): open, page through 6 000 lines, type** | **+214 MB → 823 MB**, line numbers stopped rendering | **+30 MB → 362 MB** |
| Linux download (without Tectonic) | 20.5 MB `.deb` (uses the system's WebKitGTK) | 28.5 MB `.tar.gz` (61 MB unpacked, engine included) |

Of Flutter's 312 MB, about 46 MB is the software OpenGL stack (llvmpipe,
`libLLVM`) that a real GPU driver replaces; the Rust core (Typst fonts and
caches) is the same in both apps.

**Reading the numbers:** Flutter uses roughly 15–20 % less memory on a fresh
start, and it stays flat when you switch projects, whereas WebKit's web
process grows. Compile speed is identical because both apps run the same Rust
code. Where they differ is the install size: Tauri borrows the system web
view, while Flutter ships its own engine, which makes the Flutter download
larger: about 8 MB more on Linux. On Windows the gap should be similar
(Tauri's v0.1.3 installer was 15–23 MB before Typst); the CI artifacts show the
Flutter size.

**The editor.** The first Flutter preview used a stock text field that laid
out the whole file on every keystroke — the honest trade-off then was "CodeMirror
is more mature". The editor here is virtualised like CodeMirror, covers the
everyday editing of the Tauri editor (plus environment bodies, file-name
completion and regex groups in replace, which that one lacks), and stays flat
on huge files where the web view grows by hundreds of megabytes. Still to do:
soft wrap, folding, multiple cursors.
