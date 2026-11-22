# KernTeX design principles

These rules are binding for every change to KernTeX. Reviewers check them; if a change can't follow one, say why in the pull request.

## 1. Every click responds immediately

Within 100 ms of any click or key press there is a visible reaction: a pressed state, a spinner, a status change. No button without feedback. Buttons in `src/app.css` (`.btn`, `.icon-btn`) already provide hover and pressed states — use them.

## 2. Long operations show where they are

Anything that may take longer than ~300 ms shows a concrete status text, e.g. “Typesetting (pass 2)”, “Downloading packages – amsmath.sty”, “Writing PDF”, plus the elapsed time; where possible it can be cancelled. Long work runs off the main thread and reports progress through events (`compile://progress`).

## 3. Error messages are unambiguous

Every message says:

- **where it came from** — LaTeX, a package name, Tectonic, latexmk, BibTeX/Biber, the file system, or KernTeX itself (`AppError.source`, `Diagnostic.source`);
- **where it happened** — file and line; one click jumps there;
- **how to fix it** — a hint in everyday language.

If the cause is unknown, say so honestly and point to the raw log. Never show a bare “Error” or a stack trace as the only information.

## 4. Features are easy to find

Every action is in the command palette (`Ctrl+K`, `src/lib/commands.svelte.ts`). Buttons have a label or a tooltip that includes the keyboard shortcut. The file tree has a context menu.

## 5. Dark and light mode

Both themes are designed with equal care — including editor syntax colours and the PDF background. The default follows the system setting; users can switch manually. All colours come from the tokens in `src/app.css`; never hard-code a colour in a component.

## 6. Modern and clear

Calm surfaces, clear hierarchy, few accent colours. UI font: system UI fonts (Inter / Segoe UI); monospace: JetBrains Mono / Cascadia / Consolas. Panels can be collapsed and resized.

## 7. Subtle animation, only where it orients

150–250 ms (`--dur`) for: collapsing panels, cross-fading the PDF after a compile (scroll position is kept), toasts, status changes. With the system's “reduce motion” setting, there are no animations.

## 8. Fast and lean

No unnecessary code; few dependencies, each justified in its pull request. Load heavy parts (pdf.js) only when needed and render PDF pages lazily. Targets: installer under ~15 MB without the Tectonic cache, start-up under 1 s, typing without noticeable latency in a 5,000-line file.

## 9. Data is safe

Atomic saves (temp file + rename), autosave, deleting only to the system trash, never overwriting files without asking. Build output goes to `<project>/.kerntex/out/`.

## Architecture rules

- `src-tauri/src/core/` imports nothing from `tauri`. Tauri commands in `commands.rs` are thin wrappers, so a CLI or the MCP server can reuse the core.
- Paths between UI and backend are project-relative with `/`. Only `project::resolve` turns them into absolute paths, and it refuses `..` and absolute paths.
- All file I/O goes through `core::project`, so a sync layer can be inserted later.
- Binary data (the PDF) is sent with `tauri::ipc::Response`, never as a JSON array.
