//! Shared by the KernTeX app and the `kerntex-mcp` proxy: where the app
//! listens for MCP connections.
//!
//! - **Windows:** a named pipe, `\\.\pipe\kerntex-mcp-<user>`.
//! - **Linux / Unix:** a Unix socket in `$XDG_RUNTIME_DIR` (a folder only the
//!   user can access), or in a private `kerntex-<user>` folder in the temp
//!   directory when that isn't set.
//!
//! No network port is opened, so web pages can't reach it.

use interprocess::local_socket::Name;
use std::io;
use std::path::PathBuf;

/// File next to an installed copy of `kerntex-mcp` that holds the path of the
/// KernTeX app, so the proxy can start the app (used for AppImages, whose
/// files live in a temporary mount).
pub const APP_PATH_FILE: &str = "kerntex-app-path";

/// Where the KernTeX app listens for MCP connections.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Endpoint {
    /// Unix socket file.
    Path(PathBuf),
    /// Windows named pipe name (without the `\\.\pipe\` prefix).
    Pipe(String),
}

impl Endpoint {
    /// The endpoint for the current user.
    pub fn for_current_user() -> io::Result<Endpoint> {
        if cfg!(windows) {
            Ok(Endpoint::Pipe(format!("kerntex-mcp-{}", sanitize(&user_name()))))
        } else {
            let dir = match std::env::var_os("XDG_RUNTIME_DIR").map(PathBuf::from) {
                Some(d) if d.is_dir() => d,
                _ => private_temp_dir()?,
            };
            Ok(Endpoint::Path(dir.join("kerntex-mcp.sock")))
        }
    }

    pub fn name(&self) -> io::Result<Name<'static>> {
        use interprocess::local_socket::prelude::*;
        match self {
            Endpoint::Path(p) => Ok(p.clone().to_fs_name::<interprocess::local_socket::GenericFilePath>()?.into_owned()),
            Endpoint::Pipe(n) => Ok(n.clone().to_ns_name::<interprocess::local_socket::GenericNamespaced>()?.into_owned()),
        }
    }

    /// Human-readable form for messages.
    pub fn display(&self) -> String {
        match self {
            Endpoint::Path(p) => p.display().to_string(),
            Endpoint::Pipe(n) => format!(r"\\.\pipe\{n}"),
        }
    }
}

fn user_name() -> String {
    std::env::var("USERNAME").or_else(|_| std::env::var("USER")).unwrap_or_else(|_| "user".into())
}

/// Letters, digits, `-` and `_` only, so the name is valid everywhere.
fn sanitize(s: &str) -> String {
    let out: String = s.chars().map(|c| if c.is_ascii_alphanumeric() || c == '-' || c == '_' { c } else { '_' }).collect();
    if out.is_empty() {
        "user".into()
    } else {
        out
    }
}

/// `<temp>/kerntex-<user>`, created with permissions for the owner only.
fn private_temp_dir() -> io::Result<PathBuf> {
    let dir = std::env::temp_dir().join(format!("kerntex-{}", sanitize(&user_name())));
    #[cfg(unix)]
    {
        use std::os::unix::fs::{DirBuilderExt, MetadataExt, PermissionsExt};
        match std::fs::DirBuilder::new().mode(0o700).create(&dir) {
            Ok(()) => {}
            Err(e) if e.kind() == io::ErrorKind::AlreadyExists => {}
            Err(e) => return Err(e),
        }
        // Refuse a folder someone else created or opened up.
        let meta = std::fs::symlink_metadata(&dir)?;
        if !meta.is_dir() || current_uid().is_some_and(|u| meta.uid() != u) {
            return Err(io::Error::new(io::ErrorKind::PermissionDenied, format!("{} isn't a private folder of this user", dir.display())));
        }
        if meta.permissions().mode() & 0o077 != 0 {
            std::fs::set_permissions(&dir, std::fs::Permissions::from_mode(0o700))?;
        }
    }
    #[cfg(not(unix))]
    std::fs::create_dir_all(&dir)?;
    Ok(dir)
}

/// The current user's id, read from a file we just created (no libc needed).
#[cfg(unix)]
fn current_uid() -> Option<u32> {
    use std::os::unix::fs::MetadataExt;
    let probe = std::env::temp_dir().join(format!("kerntex-uid-probe-{}", std::process::id()));
    std::fs::write(&probe, b"").ok()?;
    let uid = std::fs::metadata(&probe).ok().map(|m| m.uid());
    let _ = std::fs::remove_file(&probe);
    uid
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sanitizes_user_names() {
        assert_eq!(sanitize("Jane Doe"), "Jane_Doe");
        assert_eq!(sanitize("jörg"), "j_rg");
        assert_eq!(sanitize(""), "user");
    }

    #[test]
    fn endpoint_is_stable_and_valid() {
        let a = Endpoint::for_current_user().unwrap();
        let b = Endpoint::for_current_user().unwrap();
        assert_eq!(a, b);
        a.name().unwrap();
        if cfg!(windows) {
            assert!(a.display().starts_with(r"\\.\pipe\kerntex-mcp-"));
        } else {
            assert!(a.display().ends_with("kerntex-mcp.sock"));
        }
    }
}
