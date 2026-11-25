//! Editor-agnostic core. Nothing in here depends on Tauri, so the same
//! functions back the desktop UI and the MCP server (see `crate::mcp` and
//! docs/mcp.md).

pub mod archive;
pub mod buildinfo;
pub mod compile;
pub mod diagnostics;
pub mod edits;
pub mod error;
pub mod project;
pub mod refs;
pub mod settings;
pub mod templates;
pub mod typst_engine;
