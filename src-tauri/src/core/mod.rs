//! Editor-agnostic core. Nothing in here depends on Tauri, so the same
//! functions can back the desktop UI, a CLI, or a future MCP server
//! (see docs/mcp.md).

pub mod archive;
pub mod buildinfo;
pub mod compile;
pub mod diagnostics;
pub mod error;
pub mod project;
pub mod refs;
pub mod settings;
pub mod templates;
