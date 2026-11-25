//! Where `kerntex-mcp` lives, so the UI can hand agents a command that keeps
//! working across updates.

use serde::Serialize;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct McpSetup {
    /// Absolute path of the `kerntex-mcp` command, if it was found.
    pub command: Option<String>,
    /// Why `command` is missing, in plain language.
    pub problem: Option<String>,
    /// Where the app listens (for troubleshooting).
    pub endpoint: String,
    /// The Claude Code command line, if installed (enables one-click setup).
    pub claude_code: Option<String>,
}

const PROXY: &str = "kerntex-mcp";

pub fn setup(local_data_dir: Option<&Path>) -> McpSetup {
    let endpoint = kerntex_mcp::Endpoint::for_current_user().map(|e| e.display()).unwrap_or_default();
    let claude_code = find_claude_code().map(|p| p.display().to_string());
    match command_path(local_data_dir) {
        Ok(p) => McpSetup { command: Some(p.display().to_string()), problem: None, endpoint, claude_code },
        Err(problem) => McpSetup { command: None, problem: Some(problem), endpoint, claude_code },
    }
}

/// Name agents show for this server.
pub const SERVER_NAME: &str = "kerntex";

/// One-click install link for an editor that supports them, built here (not in
/// the UI) so the window can't be made to open arbitrary links.
pub fn install_link(target: &str, command: &str) -> Option<String> {
    match target {
        // https://cursor.com/docs/mcp/install-links
        "cursor" => {
            let config = serde_json::json!({ "command": command, "args": [] }).to_string();
            Some(format!("cursor://anysphere.cursor-deeplink/mcp/install?name={SERVER_NAME}&config={}", base64(config.as_bytes())))
        }
        // VS Code's `vscode:mcp/install` handler takes the server as URL-encoded JSON.
        "vscode" => {
            let config = serde_json::json!({ "name": SERVER_NAME, "type": "stdio", "command": command, "args": [] }).to_string();
            Some(format!("vscode:mcp/install?{}", percent_encode(&config)))
        }
        _ => None,
    }
}

/// Registers `command` with Claude Code for all projects of this user
/// (`claude mcp add --scope user`). Replaces an older KernTeX entry.
pub fn add_to_claude_code(command: &str) -> Result<String, String> {
    let claude = find_claude_code().ok_or(
        "Claude Code wasn't found on this computer. Install it from claude.com/claude-code, or copy the command and run it yourself.",
    )?;
    // An existing entry (e.g. from an older install path) would make `add` fail.
    let _ = run(&claude, &["mcp", "remove", "--scope", "user", SERVER_NAME]);
    let out = run(&claude, &["mcp", "add", "--scope", "user", SERVER_NAME, "--", command])
        .map_err(|e| format!("Couldn't run Claude Code ({}): {e}", claude.display()))?;
    if out.status.success() {
        Ok(String::from_utf8_lossy(&out.stdout).trim().to_string())
    } else {
        let msg = String::from_utf8_lossy(&out.stderr).trim().to_string();
        Err(format!(
            "Claude Code refused the setup: {}",
            if msg.is_empty() { String::from_utf8_lossy(&out.stdout).trim().to_string() } else { msg }
        ))
    }
}

fn run(program: &Path, args: &[&str]) -> std::io::Result<std::process::Output> {
    let mut cmd = std::process::Command::new(program);
    cmd.args(args).stdin(std::process::Stdio::null());
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
        cmd.creation_flags(CREATE_NO_WINDOW);
    }
    cmd.output()
}

/// `claude` on PATH, or where its installers put it (apps started from the
/// desktop often don't see the shell's PATH).
fn find_claude_code() -> Option<PathBuf> {
    if let Some(p) = crate::core::compile::find_in_path("claude") {
        return Some(p);
    }
    let home = std::env::var_os("HOME").or_else(|| std::env::var_os("USERPROFILE")).map(PathBuf::from)?;
    let name = format!("claude{}", std::env::consts::EXE_SUFFIX);
    [home.join(".local").join("bin").join(&name), home.join(".claude").join("local").join(&name)].into_iter().find(|p| p.is_file())
}

fn base64(bytes: &[u8]) -> String {
    const ABC: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::with_capacity(bytes.len().div_ceil(3) * 4);
    for chunk in bytes.chunks(3) {
        let n = (u32::from(chunk[0]) << 16) | (u32::from(*chunk.get(1).unwrap_or(&0)) << 8) | u32::from(*chunk.get(2).unwrap_or(&0));
        for i in 0..4 {
            if i <= chunk.len() {
                out.push(ABC[(n >> (18 - 6 * i) & 63) as usize] as char);
            } else {
                out.push('=');
            }
        }
    }
    out
}

fn percent_encode(s: &str) -> String {
    s.bytes()
        .map(|b| match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => (b as char).to_string(),
            _ => format!("%{b:02X}"),
        })
        .collect()
}

fn exe_name() -> String {
    format!("{PROXY}{}", std::env::consts::EXE_SUFFIX)
}

fn command_path(local_data_dir: Option<&Path>) -> Result<PathBuf, String> {
    let exe = std::env::current_exe().map_err(|e| format!("KernTeX couldn't find its own location ({e})."))?;
    let dir = exe.parent().ok_or("KernTeX couldn't find its own folder.")?;
    let bundled = dir.join(exe_name());
    if !bundled.is_file() {
        return Err(format!(
            "kerntex-mcp wasn't found next to KernTeX ({}). Reinstall KernTeX; when building from source, run `cargo build -p kerntex-mcp`.",
            dir.display()
        ));
    }

    // Microsoft Store: the install folder changes with every update and isn't
    // directly runnable; the package's execution alias is stable.
    if cfg!(windows) && exe.to_string_lossy().contains(r"\WindowsApps\") {
        if let Some(local) = std::env::var_os("LOCALAPPDATA") {
            return Ok(PathBuf::from(local).join("Microsoft").join("WindowsApps").join(exe_name()));
        }
    }

    // AppImage: our files live in a temporary mount that disappears when the
    // app quits, so keep a copy in a stable place (refreshed on every call),
    // together with the path of the AppImage so it can be started.
    if let Some(appimage) = std::env::var_os("APPIMAGE") {
        let Some(data) = local_data_dir else {
            return Err("No folder for app data was found, so kerntex-mcp can't be installed for this AppImage.".into());
        };
        let bin = data.join("bin");
        let target = bin.join(exe_name());
        let copy = || -> std::io::Result<()> {
            std::fs::create_dir_all(&bin)?;
            let fresh = std::fs::read(&bundled)?;
            if std::fs::read(&target).ok().as_deref() != Some(fresh.as_slice()) {
                let tmp = bin.join(format!("{PROXY}.tmp"));
                std::fs::write(&tmp, &fresh)?;
                #[cfg(unix)]
                {
                    use std::os::unix::fs::PermissionsExt;
                    std::fs::set_permissions(&tmp, std::fs::Permissions::from_mode(0o755))?;
                }
                std::fs::rename(&tmp, &target)?;
            }
            std::fs::write(bin.join(kerntex_mcp::APP_PATH_FILE), appimage.to_string_lossy().as_bytes())
        };
        copy().map_err(|e| format!("Couldn't install kerntex-mcp to {} ({e}).", bin.display()))?;
        return Ok(target);
    }

    Ok(bundled)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn base64_matches_the_standard_alphabet() {
        assert_eq!(base64(b""), "");
        assert_eq!(base64(b"f"), "Zg==");
        assert_eq!(base64(b"fo"), "Zm8=");
        assert_eq!(base64(b"foo"), "Zm9v");
        assert_eq!(base64(b"{\"a\":1}"), "eyJhIjoxfQ==");
    }

    #[test]
    fn builds_install_links() {
        let cursor = install_link("cursor", "/opt/kerntex/kerntex-mcp").unwrap();
        assert!(cursor.starts_with("cursor://anysphere.cursor-deeplink/mcp/install?name=kerntex&config="));
        assert!(cursor.ends_with(&base64(br#"{"args":[],"command":"/opt/kerntex/kerntex-mcp"}"#)));
        let vscode = install_link("vscode", r"C:\Program Files\KernTeX\kerntex-mcp.exe").unwrap();
        assert!(vscode.starts_with("vscode:mcp/install?%7B"));
        assert!(!vscode.contains(' ') && !vscode.contains('"'));
        assert!(install_link("https://evil.example", "x").is_none());
    }
}
