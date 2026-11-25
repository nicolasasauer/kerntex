//! `kerntex-mcp`: the command AI agents start to talk to KernTeX.
//!
//! It speaks MCP over stdio by forwarding bytes between stdin/stdout and the
//! running KernTeX app (see `lib.rs` for where the app listens). All MCP logic
//! lives in the app; this program only connects, and starts KernTeX if it
//! isn't running yet. Log messages go to stderr, as MCP requires.

use interprocess::local_socket::{prelude::*, Stream};
use kerntex_mcp::{Endpoint, APP_PATH_FILE};
use std::io::{self, Read, Write};
use std::path::PathBuf;
use std::process::ExitCode;
use std::time::{Duration, Instant};

const HELP: &str = "kerntex-mcp — connects AI assistants (MCP clients) to the KernTeX app

Usage: kerntex-mcp [--no-launch]

Add this command as a local (stdio) MCP server in your AI assistant, e.g.
  claude mcp add --scope user kerntex -- kerntex-mcp
KernTeX → Command palette → “AI assistants” shows the exact setup for your system.

Options:
  --no-launch   Don't start KernTeX if it isn't running
  --version     Print the version
  --help        Print this help";

/// How long to wait for a freshly started KernTeX to accept connections.
const LAUNCH_WAIT: Duration = Duration::from_secs(30);

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.iter().any(|a| a == "--help" || a == "-h") {
        println!("{HELP}");
        return ExitCode::SUCCESS;
    }
    if args.iter().any(|a| a == "--version" || a == "-V") {
        println!("kerntex-mcp {}", env!("CARGO_PKG_VERSION"));
        return ExitCode::SUCCESS;
    }
    let launch = !args.iter().any(|a| a == "--no-launch");

    let endpoint = match Endpoint::for_current_user() {
        Ok(e) => e,
        Err(e) => return fail(&format!("Couldn't determine where KernTeX listens: {e}")),
    };
    let stream = match connect(&endpoint, launch) {
        Ok(s) => s,
        Err(msg) => return fail(&msg),
    };
    pipe(stream);
    ExitCode::SUCCESS
}

fn fail(msg: &str) -> ExitCode {
    eprintln!("kerntex-mcp: {msg}");
    ExitCode::FAILURE
}

fn try_connect(endpoint: &Endpoint) -> io::Result<Stream> {
    Stream::connect(endpoint.name()?)
}

fn connect(endpoint: &Endpoint, launch: bool) -> Result<Stream, String> {
    if let Ok(s) = try_connect(endpoint) {
        return Ok(s);
    }
    let not_running = "KernTeX isn't running. Open KernTeX, then try again.";
    if !launch {
        return Err(not_running.into());
    }
    let Some(app) = find_app() else {
        return Err(format!("{not_running} (The KernTeX app wasn't found next to kerntex-mcp, so it can't be started automatically.)"));
    };
    eprintln!("kerntex-mcp: starting KernTeX ({})…", app.display());
    if let Err(e) = start_detached(&app) {
        return Err(format!("{not_running} (Starting {} failed: {e})", app.display()));
    }
    let deadline = Instant::now() + LAUNCH_WAIT;
    while Instant::now() < deadline {
        std::thread::sleep(Duration::from_millis(300));
        if let Ok(s) = try_connect(endpoint) {
            return Ok(s);
        }
    }
    Err(format!(
        "KernTeX was started but didn't answer within {} s (listening at {}). Check that it opened, then try again.",
        LAUNCH_WAIT.as_secs(),
        endpoint.display()
    ))
}

/// The KernTeX app: next to this program, or named in `kerntex-app-path`.
fn find_app() -> Option<PathBuf> {
    let me = std::env::current_exe().ok()?;
    let dir = me.parent()?;
    let sibling = dir.join(format!("kerntex{}", std::env::consts::EXE_SUFFIX));
    if sibling.is_file() {
        return Some(sibling);
    }
    let named = std::fs::read_to_string(dir.join(APP_PATH_FILE)).ok()?;
    let p = PathBuf::from(named.trim());
    p.is_file().then_some(p)
}

/// Starts the app so it keeps running after the agent ends this process.
fn start_detached(app: &PathBuf) -> io::Result<()> {
    let mut cmd = std::process::Command::new(app);
    cmd.stdin(std::process::Stdio::null()).stdout(std::process::Stdio::null()).stderr(std::process::Stdio::null());
    #[cfg(unix)]
    {
        use std::os::unix::process::CommandExt;
        cmd.process_group(0);
    }
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        const DETACHED_PROCESS: u32 = 0x0000_0008;
        const CREATE_NEW_PROCESS_GROUP: u32 = 0x0000_0200;
        cmd.creation_flags(DETACHED_PROCESS | CREATE_NEW_PROCESS_GROUP);
    }
    cmd.spawn().map(|_| ())
}

/// Copies stdin → app and app → stdout until either side closes.
fn pipe(stream: Stream) {
    // `&Stream` reads and writes independently, so both directions can share it.
    let stream = std::sync::Arc::new(stream);
    let to_app = stream.clone();
    std::thread::spawn(move || {
        let _ = copy_flushing(&mut io::stdin().lock(), &mut &*to_app);
        // The agent closed stdin: it's done with us.
        std::process::exit(0);
    });
    let _ = copy_flushing(&mut &*stream, &mut io::stdout().lock());
    // KernTeX closed the connection (quit, or the user disconnected the agent).
}

/// Like `io::copy`, but flushes after every read so messages aren't held back.
fn copy_flushing(from: &mut impl Read, to: &mut impl Write) -> io::Result<()> {
    let mut buf = [0u8; 64 * 1024];
    loop {
        let n = match from.read(&mut buf) {
            Ok(0) => return Ok(()),
            Ok(n) => n,
            Err(e) if e.kind() == io::ErrorKind::Interrupted => continue,
            Err(e) => return Err(e),
        };
        to.write_all(&buf[..n])?;
        to.flush()?;
    }
}
