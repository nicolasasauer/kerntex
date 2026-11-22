# Contributing to KernTeX

Thanks for helping! Bug reports, ideas and pull requests are all welcome.

## Setup

See *Build from source* in the [README](README.md). In short:

```sh
npm install
npm run setup:tectonic
npm run tauri dev
```

For UI-only work, `npm run dev` serves the interface in a normal browser at http://localhost:1420 with an in-memory mock backend (`src/lib/mock/`). Add `?demo` to the URL to open the example project right away.

## Project layout

| Path | What |
| --- | --- |
| `src-tauri/src/core/` | All domain logic in plain Rust (no Tauri): project files, compiling, log parsing, ZIP, settings, templates |
| `src-tauri/src/commands.rs` | Thin Tauri command wrappers |
| `src-tauri/templates/` | New-project templates and the welcome project |
| `src/lib/` | Frontend state (`workspace.svelte.ts`), API, commands/shortcuts, CodeMirror setup, LaTeX knowledge base |
| `src/components/` | Svelte 5 components |
| `scripts/` | Build helpers (Tectonic download) |

## Before you open a pull request

All of these must pass (CI runs them on Ubuntu and Windows):

```sh
npm run check          # svelte-check / TypeScript
npm run build
npm run test:scripts
cd src-tauri
cargo fmt --check
cargo clippy --all-targets -- -D warnings
cargo test             # end-to-end tests run where latexmk / Tectonic are available
```

Set `KERNTEX_TECTONIC=/path/to/tectonic` to run the Tectonic end-to-end test locally.

- Follow [DESIGN.md](DESIGN.md) — especially feedback within 100 ms, clear error messages, and both themes.
- For UI changes, check the result in light and dark mode (a screenshot in the PR helps).
- New dependencies only when they clearly save work; justify each one in the PR description.
- Log-parser changes need a test with a real log excerpt.

## Style

- English in code, comments, commits and docs.
- Small, focused commits with an imperative subject line (“Add clean ZIP export”), body explaining *why* when it isn't obvious.
- Rust: `cargo fmt` (see `src-tauri/rustfmt.toml`). TypeScript/Svelte: 2-space indent, keep components small.
