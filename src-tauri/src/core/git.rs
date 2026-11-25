//! Git integration through the system `git` binary.
//!
//! Calling the installed Git (instead of embedding a Git library) means the
//! user's own configuration and sign-in keep working: the Git Credential
//! Manager on Windows, SSH keys, `includeIf`, hooks. KernTeX never sees a
//! password or token.
//!
//! The project folder may be the repository root or a subfolder of a larger
//! repository; all paths exchanged with the UI are relative to the *project*
//! (like everywhere else in KernTeX), and commits only include the project
//! folder.

use super::compile::{find_in_path, hide_window, kill_tree};
use super::error::{AppError, ErrorSource, Result};
use super::project::{self, BUILD_DIR};
use serde::Serialize;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Output, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

/// Build output never goes into a commit, even if the repository's
/// `.gitignore` doesn't exclude it.
const EXCLUDE_BUILD: &str = ":(exclude).kerntex/out";

/// Upper bound for a diff sent to the UI.
const MAX_DIFF_BYTES: usize = 512 * 1024;

#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum ChangeKind {
    Modified,
    Added,
    Deleted,
    Renamed,
    Untracked,
    Conflicted,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct GitChange {
    /// Project-relative path with `/`.
    pub path: String,
    /// Previous path of a rename.
    pub old_path: Option<String>,
    pub kind: ChangeKind,
}

#[derive(Debug, Clone, Serialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct CommitInfo {
    pub hash: String,
    pub subject: String,
    /// Unix seconds.
    pub time: i64,
}

#[derive(Debug, Clone, Serialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct GitStatus {
    /// `git --version` output; `None` = Git isn't installed (or not found).
    pub git_version: Option<String>,
    /// The project folder is inside a Git work tree.
    pub repo: bool,
    /// Current branch; `None` when detached.
    pub branch: Option<String>,
    /// e.g. `origin/main`.
    pub upstream: Option<String>,
    pub ahead: u32,
    pub behind: u32,
    /// Remote that push/pull use (upstream's remote, else `origin`, else the first one).
    pub remote: Option<String>,
    pub remote_url: Option<String>,
    pub has_commits: bool,
    pub last_commit: Option<CommitInfo>,
    /// A merge stopped with conflicts and is waiting for a commit.
    pub merging: bool,
    /// `user.name` / `user.email` are configured (needed to commit).
    pub identity: Option<Identity>,
    pub changes: Vec<GitChange>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct Identity {
    pub name: String,
    pub email: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PullResult {
    /// HEAD moved: files on disk may have changed.
    pub updated: bool,
    /// Files with merge conflicts (project-relative); empty = clean.
    pub conflicts: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GitProgress {
    /// "pull" or "push".
    pub op: String,
    /// Latest progress line from Git, e.g. "Writing objects: 45% (9/20)".
    pub line: String,
}

/// Lets the UI cancel a running pull/push (kills the `git` process tree).
#[derive(Clone, Default)]
pub struct GitCancel {
    child: Arc<Mutex<Option<Child>>>,
    canceled: Arc<AtomicBool>,
}

impl GitCancel {
    pub fn cancel(&self) -> bool {
        if let Some(child) = self.child.lock().unwrap().as_mut() {
            self.canceled.store(true, Ordering::SeqCst);
            kill_tree(child);
            return true;
        }
        false
    }
}

// ---------- Running git ----------

fn git_bin() -> Option<PathBuf> {
    if let Some(p) = std::env::var_os("KERNTEX_GIT") {
        return Some(PathBuf::from(p));
    }
    if let Some(p) = find_in_path("git") {
        return Some(p);
    }
    // Apps started from the Start menu sometimes miss PATH changes made by a
    // Git installation that happened after login.
    if cfg!(windows) {
        let candidates = [std::env::var_os("ProgramFiles"), std::env::var_os("ProgramW6432"), std::env::var_os("LOCALAPPDATA")];
        for base in candidates.into_iter().flatten() {
            let base = PathBuf::from(base);
            for rel in ["Git/cmd/git.exe", "Programs/Git/cmd/git.exe"] {
                let p = base.join(rel);
                if p.is_file() {
                    return Some(p);
                }
            }
        }
    }
    None
}

fn not_installed() -> AppError {
    AppError::new(ErrorSource::Git, "Git isn't installed")
        .hint("Install Git from git-scm.com (on Windows: “Git for Windows”, which also brings the Git Credential Manager for signing in to GitHub). Restart KernTeX afterwards.")
}

fn command(root: &Path) -> Result<Command> {
    let bin = git_bin().ok_or_else(not_installed)?;
    let mut c = Command::new(bin);
    c.current_dir(root)
        // English messages so errors can be recognised and explained.
        .env("LC_ALL", "C")
        .env("LANGUAGE", "C")
        // Never wait for input on a terminal nobody can see. Credential
        // helpers with their own window (Git Credential Manager) still work.
        .env("GIT_TERMINAL_PROMPT", "0")
        // Status checks must not take the index lock away from a real command.
        .env("GIT_OPTIONAL_LOCKS", "0")
        .args(["-c", "core.quotepath=false", "-c", "color.ui=false"])
        .stdin(Stdio::null());
    hide_window(&mut c);
    Ok(c)
}

fn output(root: &Path, args: &[&str]) -> Result<Output> {
    command(root)?.args(args).output().map_err(|e| {
        if e.kind() == std::io::ErrorKind::NotFound {
            not_installed()
        } else {
            AppError::new(ErrorSource::Git, "Couldn't start Git").detail(e.to_string())
        }
    })
}

/// Runs git and returns stdout; a non-zero exit becomes an explained error.
fn run(root: &Path, action: &str, args: &[&str]) -> Result<String> {
    let out = output(root, args)?;
    if out.status.success() {
        Ok(String::from_utf8_lossy(&out.stdout).into_owned())
    } else {
        Err(explain(action, &String::from_utf8_lossy(&out.stderr), &String::from_utf8_lossy(&out.stdout)))
    }
}

/// Like [`run`], but returns `None` instead of an error.
fn try_run(root: &Path, args: &[&str]) -> Option<String> {
    let out = output(root, args).ok()?;
    out.status.success().then(|| String::from_utf8_lossy(&out.stdout).into_owned())
}

/// Turns Git's message into: what happened, and how to fix it.
fn explain(action: &str, stderr: &str, stdout: &str) -> AppError {
    let all = format!("{stderr}\n{stdout}");
    let low = all.to_lowercase();
    let detail = tail(all.trim(), 1200);
    let e = |title: &str, hint: &str| AppError::new(ErrorSource::Git, title).detail(detail.clone()).hint(hint);
    if low.contains("please tell me who you are") || low.contains("empty ident") || low.contains("unable to auto-detect email") {
        return e(
            "Git needs your name and email",
            "Enter them in the Git panel. They are stored in your Git settings and written into every commit.",
        );
    }
    if low.contains("authentication failed")
        || low.contains("could not read username")
        || low.contains("could not read password")
        || low.contains("terminal prompts disabled")
        || low.contains("permission denied (publickey")
        || low.contains("invalid username or password")
        || low.contains("the requested url returned error: 403")
        || low.contains("the requested url returned error: 401")
    {
        return e(
            "Sign-in to the remote failed",
            "On Windows, install “Git for Windows” — its Git Credential Manager opens your browser to sign in to GitHub. For Overleaf, use the Git token from your Overleaf account settings. Then try again.",
        );
    }
    if low.contains("repository not found") || low.contains("does not appear to be a git repository") || low.contains("returned error: 404")
    {
        return e(
            "The remote repository wasn't found",
            "Check the remote URL in the Git panel, and that your account has access to the repository.",
        );
    }
    if low.contains("could not resolve host")
        || low.contains("failed to connect")
        || low.contains("connection timed out")
        || low.contains("network is unreachable")
    {
        return e("Couldn't reach the remote", "Check your internet connection (and VPN/proxy), then try again.");
    }
    if low.contains("[rejected]") || low.contains("non-fast-forward") || low.contains("fetch first") {
        return e("The remote has changes you don't have yet", "Pull first, then push again. “Commit & Push” does this automatically.");
    }
    if low.contains("would be overwritten by merge") || low.contains("commit your changes or stash them") {
        return e("Your uncommitted changes would be overwritten", "Commit your changes first (or use “Commit & Push”), then pull again.");
    }
    if low.contains("nothing to commit") || low.contains("no changes added to commit") {
        return e("Nothing to commit", "All changes are already committed.");
    }
    if low.contains("index.lock") {
        return e(
            "Another Git program is working on this repository",
            "Wait for it to finish (or close it) and try again. If no other Git program is running, delete the file .git/index.lock.",
        );
    }
    if low.contains("refusing to merge unrelated histories") {
        return e("The remote has an unrelated history", "The remote repository was created separately (e.g. with its own README). Merge it once in a terminal with `git pull --allow-unrelated-histories`, or push to a new, empty repository.");
    }
    if low.contains("no configured push destination") || low.contains("no such remote") {
        return e(
            "No remote configured",
            "Add a remote URL in the Git panel (e.g. the URL of a new, empty GitHub repository or your Overleaf Git URL).",
        );
    }
    if low.contains("not a git repository") {
        return e("This project isn't a Git repository", "Use “Set up Git for this project” in the Git panel.");
    }
    if low.contains("you have not concluded your merge") || low.contains("unmerged files") || low.contains("fix conflicts") {
        return e("A merge is waiting for conflict resolution", "Open the files marked as conflicted, keep the right text between the <<<<<<< and >>>>>>> markers, save, then commit. Or abort the merge in the Git panel.");
    }
    AppError::new(ErrorSource::Git, format!("Git couldn't {action}"))
        .detail(detail)
        .hint("See Git's message above. If it doesn't help, the same command in a terminal may tell you more.")
}

fn tail(s: &str, max: usize) -> String {
    if s.len() <= max {
        return s.to_string();
    }
    let mut start = s.len() - max;
    while !s.is_char_boundary(start) {
        start += 1;
    }
    format!("…{}", &s[start..])
}

// ---------- Status ----------

pub fn version() -> Option<String> {
    let mut c = Command::new(git_bin()?);
    c.arg("--version").stdin(Stdio::null()).stderr(Stdio::null());
    hide_window(&mut c);
    let out = c.output().ok()?;
    out.status.success().then(|| String::from_utf8_lossy(&out.stdout).trim().to_string())
}

/// Path of the project folder inside the repository, e.g. `thesis/` (empty at the root).
fn prefix(root: &Path) -> Result<Option<String>> {
    let out = output(root, &["rev-parse", "--is-inside-work-tree", "--show-prefix"])?;
    if !out.status.success() {
        return Ok(None);
    }
    let text = String::from_utf8_lossy(&out.stdout);
    let mut lines = text.lines();
    if lines.next() != Some("true") {
        return Ok(None);
    }
    Ok(Some(lines.next().unwrap_or("").trim().to_string()))
}

fn strip(prefix: &str, path: &str) -> String {
    path.strip_prefix(prefix).unwrap_or(path).to_string()
}

pub fn status(root: &Path) -> Result<GitStatus> {
    let Some(git_version) = version() else {
        return Ok(GitStatus::default());
    };
    let mut st = GitStatus { git_version: Some(git_version), ..Default::default() };
    let Some(prefix) = prefix(root)? else {
        return Ok(st);
    };
    st.repo = true;

    let raw =
        run(root, "read the status", &["status", "--porcelain=v2", "-z", "--branch", "--untracked-files=all", "--", ".", EXCLUDE_BUILD])?;
    parse_status(&raw, &prefix, &mut st);

    let remotes = try_run(root, &["remote", "-v"]).unwrap_or_default();
    let remotes = parse_remotes(&remotes);
    let remote = st
        .upstream
        .as_deref()
        .and_then(|u| remotes.iter().filter(|(n, _)| u.starts_with(&format!("{n}/"))).max_by_key(|(n, _)| n.len()))
        .or_else(|| remotes.iter().find(|(n, _)| n == "origin"))
        .or_else(|| remotes.first());
    if let Some((name, url)) = remote {
        st.remote = Some(name.clone());
        st.remote_url = Some(url.clone());
    }

    if st.has_commits {
        if let Some(log) = try_run(root, &["log", "-1", "--format=%h%x00%s%x00%ct"]) {
            let mut parts = log.trim_end().splitn(3, '\0');
            if let (Some(h), Some(s), Some(t)) = (parts.next(), parts.next(), parts.next()) {
                st.last_commit = Some(CommitInfo { hash: h.into(), subject: s.into(), time: t.trim().parse().unwrap_or(0) });
            }
        }
    }

    if let Some(p) = try_run(root, &["rev-parse", "--git-path", "MERGE_HEAD"]) {
        st.merging = root.join(p.trim()).is_file();
    }

    st.identity = identity(root);
    Ok(st)
}

fn identity(root: &Path) -> Option<Identity> {
    let cfg = try_run(root, &["config", "--get-regexp", r"^user\.(name|email)$"])?;
    let (mut name, mut email) = (String::new(), String::new());
    for line in cfg.lines() {
        if let Some(v) = line.strip_prefix("user.name ") {
            name = v.trim().into();
        } else if let Some(v) = line.strip_prefix("user.email ") {
            email = v.trim().into();
        }
    }
    (!name.is_empty() && !email.is_empty()).then_some(Identity { name, email })
}

/// `name\turl (fetch)` lines → unique (name, url), in order.
fn parse_remotes(text: &str) -> Vec<(String, String)> {
    let mut out: Vec<(String, String)> = Vec::new();
    for line in text.lines() {
        let mut it = line.split_whitespace();
        if let (Some(name), Some(url)) = (it.next(), it.next()) {
            if !out.iter().any(|(n, _)| n == name) {
                out.push((name.into(), url.into()));
            }
        }
    }
    out
}

/// Parses `git status --porcelain=v2 -z --branch`.
fn parse_status(raw: &str, prefix: &str, st: &mut GitStatus) {
    let mut entries = raw.split('\0').filter(|s| !s.is_empty());
    while let Some(e) = entries.next() {
        if let Some(h) = e.strip_prefix("# ") {
            if let Some(v) = h.strip_prefix("branch.oid ") {
                st.has_commits = v != "(initial)";
            } else if let Some(v) = h.strip_prefix("branch.head ") {
                st.branch = (v != "(detached)").then(|| v.to_string());
            } else if let Some(v) = h.strip_prefix("branch.upstream ") {
                st.upstream = Some(v.to_string());
            } else if let Some(v) = h.strip_prefix("branch.ab ") {
                for part in v.split_whitespace() {
                    if let Some(n) = part.strip_prefix('+') {
                        st.ahead = n.parse().unwrap_or(0);
                    } else if let Some(n) = part.strip_prefix('-') {
                        st.behind = n.parse().unwrap_or(0);
                    }
                }
            }
            continue;
        }
        let (tag, rest) = e.split_at(1);
        let rest = rest.trim_start();
        match tag {
            "1" => {
                // XY sub mH mI mW hH hI path
                let f: Vec<&str> = rest.splitn(8, ' ').collect();
                if f.len() == 8 {
                    st.changes.push(GitChange { path: strip(prefix, f[7]), old_path: None, kind: kind_of(f[0]) });
                }
            }
            "2" => {
                // XY sub mH mI mW hH hI Xscore path \0 origPath
                let f: Vec<&str> = rest.splitn(9, ' ').collect();
                let orig = entries.next();
                if f.len() == 9 {
                    st.changes.push(GitChange {
                        path: strip(prefix, f[8]),
                        old_path: orig.map(|o| strip(prefix, o)),
                        kind: ChangeKind::Renamed,
                    });
                }
            }
            "u" => {
                // XY sub m1 m2 m3 mW h1 h2 h3 path
                let f: Vec<&str> = rest.splitn(10, ' ').collect();
                if f.len() == 10 {
                    st.changes.push(GitChange { path: strip(prefix, f[9]), old_path: None, kind: ChangeKind::Conflicted });
                }
            }
            "?" => st.changes.push(GitChange { path: strip(prefix, rest), old_path: None, kind: ChangeKind::Untracked }),
            _ => {}
        }
    }
    st.changes.sort_by_key(|c| c.path.to_lowercase());
}

fn kind_of(xy: &str) -> ChangeKind {
    if xy.contains('D') {
        ChangeKind::Deleted
    } else if xy.starts_with('A') {
        ChangeKind::Added
    } else {
        ChangeKind::Modified
    }
}

// ---------- Diff ----------

/// Unified diff of one file against the last commit (untracked: whole file as added).
pub fn diff(root: &Path, path: &str, old_path: Option<&str>) -> Result<String> {
    project::resolve(root, path)?;
    if let Some(o) = old_path {
        project::resolve(root, o)?;
    }
    let has_head = try_run(root, &["rev-parse", "--verify", "-q", "HEAD"]).is_some();
    let tracked = has_head && try_run(root, &["cat-file", "-e", &format!("HEAD:./{}", old_path.unwrap_or(path))]).is_some();
    let text = if tracked {
        let mut args = vec!["diff", "-M", "HEAD", "--"];
        if let Some(o) = old_path {
            args.push(o);
        }
        args.push(path);
        run(root, "show the changes", &args)?
    } else {
        // Untracked or newly added: compare with nothing. Exit code 1 = "there are differences".
        let out = output(root, &["diff", "--no-index", "--", "/dev/null", path])?;
        if out.status.code().is_some_and(|c| c > 1) {
            return Err(explain("show the changes", &String::from_utf8_lossy(&out.stderr), ""));
        }
        String::from_utf8_lossy(&out.stdout).into_owned()
    };
    Ok(if text.len() > MAX_DIFF_BYTES {
        let mut end = MAX_DIFF_BYTES;
        while !text.is_char_boundary(end) {
            end -= 1;
        }
        format!("{}\n… (diff shortened — it is larger than 512 KB)\n", &text[..end])
    } else {
        text
    })
}

// ---------- Commit message suggestion ----------

/// A short, rule-based commit message from what changed, e.g.
/// "Edit “Introduction” and “Methods”, add plot.png".
pub fn suggest_message(root: &Path) -> Result<String> {
    let st = status(root)?;
    if !st.has_commits {
        return Ok("Initial commit".into());
    }
    let changes: Vec<&GitChange> = st.changes.iter().filter(|c| c.kind != ChangeKind::Conflicted).collect();
    if changes.is_empty() {
        return Ok(if st.merging { "Merge remote changes".into() } else { String::new() });
    }
    let name = |p: &str| p.rsplit('/').next().unwrap_or(p).to_string();
    let modified: Vec<&GitChange> = changes.iter().copied().filter(|c| c.kind == ChangeKind::Modified).collect();
    let added: Vec<String> =
        changes.iter().filter(|c| matches!(c.kind, ChangeKind::Added | ChangeKind::Untracked)).map(|c| name(&c.path)).collect();
    let deleted: Vec<String> = changes.iter().filter(|c| c.kind == ChangeKind::Deleted).map(|c| name(&c.path)).collect();
    let renamed: Vec<&GitChange> = changes.iter().copied().filter(|c| c.kind == ChangeKind::Renamed).collect();

    // Section titles around the changed lines of modified documents.
    let mut headings: Vec<String> = Vec::new();
    for c in modified.iter().filter(|c| is_document(&c.path)) {
        for h in changed_headings(root, &c.path) {
            if !headings.contains(&h) {
                headings.push(h);
            }
        }
    }

    let mut parts: Vec<String> = Vec::new();
    if !headings.is_empty() {
        parts.push(format!("edit {}", list(&headings.iter().map(|h| format!("“{h}”")).collect::<Vec<_>>(), 2, "sections")));
        let other: Vec<String> = modified.iter().filter(|c| !is_document(&c.path)).map(|c| name(&c.path)).collect();
        if !other.is_empty() {
            parts.push(format!("update {}", list(&other, 2, "files")));
        }
    } else if !modified.is_empty() {
        parts.push(format!("update {}", list(&modified.iter().map(|c| name(&c.path)).collect::<Vec<_>>(), 2, "files")));
    }
    if !added.is_empty() {
        parts.push(format!("add {}", list(&added, 2, "files")));
    }
    if !deleted.is_empty() {
        parts.push(format!("remove {}", list(&deleted, 2, "files")));
    }
    if renamed.len() == 1 {
        let r = renamed[0];
        parts.push(format!("rename {} to {}", name(r.old_path.as_deref().unwrap_or("?")), name(&r.path)));
    } else if renamed.len() > 1 {
        parts.push(format!("rename {} files", renamed.len()));
    }
    let mut msg = parts.join(", ");
    if let Some(first) = msg.get(0..1) {
        msg = first.to_uppercase() + &msg[1..];
    }
    Ok(shorten(&msg, 72))
}

fn is_document(path: &str) -> bool {
    let p = path.to_ascii_lowercase();
    p.ends_with(".tex") || p.ends_with(".typ")
}

/// "a", "a and b", "a, b and 3 more files".
fn list(items: &[String], max: usize, noun: &str) -> String {
    match items.len() {
        0 => String::new(),
        1 => items[0].clone(),
        n if n <= max => format!("{} and {}", items[..n - 1].join(", "), items[n - 1]),
        n if max == 0 => format!("{n} {noun}"),
        n => format!("{} and {} more", items[..max].join(", "), n - max),
    }
}

fn shorten(s: &str, max: usize) -> String {
    if s.chars().count() <= max {
        return s.to_string();
    }
    let cut: String = s.chars().take(max - 1).collect();
    // Prefer cutting at a word boundary.
    let cut = match cut.rfind([' ', ',']) {
        Some(i) if i > max / 2 => cut[..i].trim_end_matches([',', ' ']).to_string(),
        _ => cut,
    };
    format!("{cut}…")
}

/// Titles of the sections that contain the changed lines of a document.
fn changed_headings(root: &Path, path: &str) -> Vec<String> {
    let Some(diff) = try_run(root, &["diff", "-U0", "HEAD", "--", path]) else {
        return Vec::new();
    };
    let Ok(text) = project::read_text(root, path) else {
        return Vec::new();
    };
    let lines: Vec<&str> = text.lines().collect();
    let typst = path.to_ascii_lowercase().ends_with(".typ");
    let mut out = Vec::new();
    for start in hunk_starts(&diff) {
        // Nearest heading at or above the changed line (1-based → index).
        let from = start.saturating_sub(1).min(lines.len().saturating_sub(1));
        if let Some(h) = (0..=from).rev().find_map(|i| heading(lines.get(i).copied().unwrap_or(""), typst)) {
            if !out.contains(&h) {
                out.push(h);
            }
        }
    }
    out
}

/// New-side start lines of `@@ -a,b +c,d @@` hunks.
fn hunk_starts(diff: &str) -> Vec<usize> {
    diff.lines()
        .filter_map(|l| l.strip_prefix("@@ "))
        .filter_map(|l| l.split_whitespace().find(|p| p.starts_with('+')))
        .filter_map(|p| p[1..].split(',').next()?.parse().ok())
        .collect()
}

fn heading(line: &str, typst: bool) -> Option<String> {
    let t = line.trim_start();
    if typst {
        let rest = t.strip_prefix('=')?.trim_start_matches('=');
        let title = rest.trim();
        return (!title.is_empty() && rest.starts_with(' ')).then(|| shorten(title, 40));
    }
    for cmd in ["\\chapter", "\\section", "\\subsection", "\\part"] {
        if let Some(rest) = t.strip_prefix(cmd) {
            let rest = rest.strip_prefix('*').unwrap_or(rest);
            // Skip an optional short title: \section[short]{long}
            let rest = if rest.starts_with('[') { rest.split_once(']').map(|(_, r)| r).unwrap_or(rest) } else { rest };
            let body = rest.strip_prefix('{')?;
            let mut depth = 1;
            let mut end = body.len();
            for (i, ch) in body.char_indices() {
                match ch {
                    '{' => depth += 1,
                    '}' => {
                        depth -= 1;
                        if depth == 0 {
                            end = i;
                            break;
                        }
                    }
                    _ => {}
                }
            }
            let title: String = body[..end].replace(['{', '}'], "").split_whitespace().collect::<Vec<_>>().join(" ");
            return (!title.is_empty()).then(|| shorten(&title, 40));
        }
    }
    None
}

// ---------- Actions ----------

/// `git init`, branch `main` (unless the user configured a default), and a
/// `.gitignore` that keeps build output out of the repository.
pub fn init(root: &Path) -> Result<()> {
    if prefix(root)?.is_some() {
        return Err(AppError::new(ErrorSource::Git, "This project is already in a Git repository"));
    }
    run(root, "set up the repository", &["init"])?;
    if try_run(root, &["config", "--get", "init.defaultBranch"]).is_none() {
        run(root, "name the branch", &["symbolic-ref", "HEAD", "refs/heads/main"])?;
    }
    let gi = root.join(".gitignore");
    let wanted = super::templates::gitignore();
    let entry = wanted.trim();
    match std::fs::read_to_string(&gi) {
        Ok(existing) => {
            if !existing.lines().any(|l| matches!(l.trim(), e if e == entry || e == format!("/{entry}") || e == format!("{BUILD_DIR}/"))) {
                let sep = if existing.is_empty() || existing.ends_with('\n') { "" } else { "\n" };
                project::write_text(root, ".gitignore", &format!("{existing}{sep}{wanted}"))?;
            }
        }
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => project::write_text(root, ".gitignore", &wanted)?,
        Err(e) => return Err(AppError::io("read", &gi, e)),
    }
    Ok(())
}

/// Commits every change in the project folder; returns the new commit.
pub fn commit(root: &Path, message: &str) -> Result<CommitInfo> {
    let message = message.trim();
    if message.is_empty() {
        return Err(AppError::new(ErrorSource::Git, "The commit message is empty").hint("Describe in a few words what you changed."));
    }
    let prefix = prefix(root)?.ok_or_else(|| explain("commit", "not a git repository", ""))?;
    // Excluding a path that is already ignored makes `git add` fail, so only
    // exclude the build output when the repository doesn't ignore it itself.
    let ignored = try_run(root, &["check-ignore", "-q", ".kerntex/out/"]).is_some();
    let mut add = vec!["add", "-A", "--", "."];
    if !ignored {
        add.push(EXCLUDE_BUILD);
    }
    run(root, "stage the changes", &add)?;
    let merging = try_run(root, &["rev-parse", "--git-path", "MERGE_HEAD"]).is_some_and(|p| root.join(p.trim()).is_file());
    let mut args = vec!["commit", "-m", message];
    // In a subfolder of a larger repository, commit only the project folder.
    // (Not possible while a merge is in progress: Git commits it as a whole.)
    if !prefix.is_empty() && !merging {
        args.extend(["--", "."]);
    }
    run(root, "commit", &args)?;
    let log = run(root, "read the new commit", &["log", "-1", "--format=%h%x00%s%x00%ct"])?;
    let mut parts = log.trim_end().splitn(3, '\0');
    Ok(CommitInfo {
        hash: parts.next().unwrap_or("").into(),
        subject: parts.next().unwrap_or("").into(),
        time: parts.next().and_then(|t| t.trim().parse().ok()).unwrap_or(0),
    })
}

/// Fetches and merges the upstream branch. Conflicts are reported, not an error.
pub fn pull(root: &Path, cancel: &GitCancel, progress: impl FnMut(GitProgress)) -> Result<PullResult> {
    let st = status(root)?;
    if !st.repo {
        return Err(explain("pull", "not a git repository", ""));
    }
    if st.upstream.is_none() {
        return Err(AppError::new(ErrorSource::Git, "Nothing to pull from yet").hint(if st.remote.is_some() {
            "This branch isn't on the remote yet. Push once; afterwards pull works."
        } else {
            "Add a remote URL in the Git panel first."
        }));
    }
    let before = try_run(root, &["rev-parse", "HEAD"]);
    let res = run_network(root, "pull", &["pull", "--no-rebase", "--no-edit", "--progress"], cancel, progress);
    let after = try_run(root, &["rev-parse", "HEAD"]);
    let conflicts: Vec<String> = status(root)?.changes.into_iter().filter(|c| c.kind == ChangeKind::Conflicted).map(|c| c.path).collect();
    match res {
        Ok(()) => Ok(PullResult { updated: before != after, conflicts }),
        // A merge that stopped on conflicts: not a failure, the UI guides through it.
        Err(_) if !conflicts.is_empty() => Ok(PullResult { updated: true, conflicts }),
        Err(e) => Err(e),
    }
}

/// Pushes the current branch; the first push sets the upstream.
pub fn push(root: &Path, cancel: &GitCancel, progress: impl FnMut(GitProgress)) -> Result<()> {
    let st = status(root)?;
    if !st.repo {
        return Err(explain("push", "not a git repository", ""));
    }
    if !st.has_commits {
        return Err(AppError::new(ErrorSource::Git, "Nothing to push yet").hint("Make a first commit, then push."));
    }
    let Some(remote) = st.remote.clone() else {
        return Err(explain("push", "no configured push destination", ""));
    };
    if st.branch.is_none() {
        return Err(AppError::new(ErrorSource::Git, "No branch is checked out")
            .hint("The repository is in “detached HEAD” state. Check out a branch in a terminal (`git switch main`), then push."));
    }
    if st.upstream.is_some() {
        run_network(root, "push", &["push", "--progress"], cancel, progress)
    } else {
        run_network(root, "push", &["push", "--progress", "--set-upstream", &remote, "HEAD"], cancel, progress)
    }
}

/// Adds `origin` (or changes its URL).
pub fn set_remote(root: &Path, url: &str) -> Result<()> {
    let url = url.trim();
    if url.is_empty() || url.contains(char::is_whitespace) || url.starts_with('-') {
        return Err(AppError::new(ErrorSource::Git, "That doesn't look like a repository URL")
            .hint("Paste the URL of the remote repository, e.g. https://github.com/you/thesis.git or https://git.overleaf.com/…"));
    }
    let remotes = parse_remotes(&try_run(root, &["remote", "-v"]).unwrap_or_default());
    if remotes.iter().any(|(n, _)| n == "origin") {
        run(root, "change the remote URL", &["remote", "set-url", "origin", url])?;
    } else {
        run(root, "add the remote", &["remote", "add", "origin", url])?;
    }
    Ok(())
}

/// Stores name and email in the user's global Git configuration.
pub fn set_identity(root: &Path, name: &str, email: &str) -> Result<()> {
    let (name, email) = (name.trim(), email.trim());
    if name.is_empty() || !email.contains('@') || email.contains(char::is_whitespace) {
        return Err(AppError::new(ErrorSource::Git, "Name and email are needed").hint("Enter your name and a valid email address."));
    }
    run(root, "save your name", &["config", "--global", "user.name", name])?;
    run(root, "save your email", &["config", "--global", "user.email", email])?;
    Ok(())
}

pub fn abort_merge(root: &Path) -> Result<()> {
    run(root, "abort the merge", &["merge", "--abort"]).map(|_| ())
}

/// Runs a pull/push with progress lines and cancellation.
fn run_network(root: &Path, op: &str, args: &[&str], cancel: &GitCancel, mut progress: impl FnMut(GitProgress)) -> Result<()> {
    let mut c = command(root)?;
    c.args(args).stdout(Stdio::piped()).stderr(Stdio::piped());
    let (mut stdout, mut stderr) = {
        let mut slot = cancel.child.lock().unwrap();
        cancel.canceled.store(false, Ordering::SeqCst);
        let mut child = c.spawn().map_err(|e| AppError::new(ErrorSource::Git, "Couldn't start Git").detail(e.to_string()))?;
        let io = (child.stdout.take().unwrap(), child.stderr.take().unwrap());
        *slot = Some(child);
        io
    };
    let t_out = std::thread::spawn(move || {
        let mut s = String::new();
        let _ = stdout.read_to_string(&mut s);
        s
    });
    // Git rewrites progress lines with \r; report each update.
    let mut err_all = Vec::new();
    let mut line = Vec::new();
    let mut buf = [0u8; 4096];
    while let Ok(n) = stderr.read(&mut buf) {
        if n == 0 {
            break;
        }
        for &b in &buf[..n] {
            err_all.push(b);
            if b == b'\r' || b == b'\n' {
                let l = String::from_utf8_lossy(&line).trim().to_string();
                if !l.is_empty() {
                    progress(GitProgress { op: op.into(), line: l.trim_start_matches("remote: ").to_string() });
                }
                line.clear();
            } else {
                line.push(b);
            }
        }
    }
    let out = t_out.join().unwrap_or_default();
    let status = cancel.child.lock().unwrap().take().map(|mut ch| ch.wait());
    if cancel.canceled.swap(false, Ordering::SeqCst) {
        return Err(AppError::new(ErrorSource::Git, format!("{} canceled", if op == "pull" { "Pull" } else { "Push" }))
            .hint("Nothing was lost. Try again whenever you like."));
    }
    match status {
        Some(Ok(s)) if s.success() => Ok(()),
        _ => Err(explain(op, &String::from_utf8_lossy(&err_all), &out)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn tmp() -> PathBuf {
        let d = std::env::temp_dir().join(format!("kerntex-git-{}-{}", std::process::id(), rand_suffix()));
        fs::create_dir_all(&d).unwrap();
        d
    }

    fn rand_suffix() -> u128 {
        std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos()
    }

    fn git(dir: &Path, args: &[&str]) {
        let ok = Command::new("git").current_dir(dir).args(args).stdout(Stdio::null()).stderr(Stdio::null()).status().unwrap().success();
        assert!(ok, "git {args:?} failed");
    }

    fn local_identity(dir: &Path) {
        git(dir, &["config", "user.name", "Test"]);
        git(dir, &["config", "user.email", "test@example.com"]);
    }

    #[test]
    fn parses_porcelain_v2() {
        let raw = "# branch.oid abc\0# branch.head main\0# branch.upstream origin/main\0# branch.ab +2 -1\0\
1 .M N... 100644 100644 100644 a b thesis/my chapter.tex\0\
2 R. N... 100644 100644 100644 a b R100 thesis/new.tex\0thesis/old.tex\0\
u UU N... 100644 100644 100644 100644 a b c thesis/main.tex\0\
? thesis/fig/plot.png\0\
1 D. N... 100644 000000 000000 a b thesis/gone.bib\0";
        let mut st = GitStatus::default();
        parse_status(raw, "thesis/", &mut st);
        assert_eq!(st.branch.as_deref(), Some("main"));
        assert_eq!(st.upstream.as_deref(), Some("origin/main"));
        assert_eq!((st.ahead, st.behind), (2, 1));
        assert!(st.has_commits);
        let by = |p: &str| st.changes.iter().find(|c| c.path == p).map(|c| c.kind);
        assert_eq!(by("my chapter.tex"), Some(ChangeKind::Modified));
        assert_eq!(by("new.tex"), Some(ChangeKind::Renamed));
        assert_eq!(st.changes.iter().find(|c| c.path == "new.tex").unwrap().old_path.as_deref(), Some("old.tex"));
        assert_eq!(by("main.tex"), Some(ChangeKind::Conflicted));
        assert_eq!(by("fig/plot.png"), Some(ChangeKind::Untracked));
        assert_eq!(by("gone.bib"), Some(ChangeKind::Deleted));
    }

    #[test]
    fn finds_headings() {
        assert_eq!(heading("\\section{Introduction}", false).as_deref(), Some("Introduction"));
        assert_eq!(heading("  \\subsection*[Short]{A {\\em long} title}", false).as_deref(), Some("A \\em long title"));
        assert_eq!(heading("== Results", true).as_deref(), Some("Results"));
        assert_eq!(heading("=not a heading", true), None);
        assert_eq!(heading("text \\section{x}", false), None);
        assert_eq!(hunk_starts("@@ -3,0 +4,2 @@ x\n+a\n@@ -10 +12 @@\n"), vec![4, 12]);
    }

    #[test]
    fn lists_and_shortens() {
        let v = |s: &[&str]| s.iter().map(|x| x.to_string()).collect::<Vec<_>>();
        assert_eq!(list(&v(&["a"]), 2, "files"), "a");
        assert_eq!(list(&v(&["a", "b"]), 2, "files"), "a and b");
        assert_eq!(list(&v(&["a", "b", "c", "d"]), 2, "files"), "a, b and 2 more");
        let s = shorten(&"word ".repeat(30), 72);
        assert!(s.chars().count() <= 72 && s.ends_with('…'));
    }

    #[test]
    fn explains_common_errors() {
        assert_eq!(
            explain("push", "fatal: Authentication failed for 'https://github.com/x/y.git/'", "").title,
            "Sign-in to the remote failed"
        );
        assert_eq!(
            explain("push", " ! [rejected]        main -> main (fetch first)", "").title,
            "The remote has changes you don't have yet"
        );
        assert_eq!(explain("commit", "", "nothing to commit, working tree clean").title, "Nothing to commit");
        assert_eq!(explain("x", "weird", "").title, "Git couldn't x");
    }

    /// End to end against the real `git`: init → commit → push to a bare
    /// remote → change on the remote → pull with a conflict → abort.
    #[test]
    fn init_commit_push_pull() {
        if version().is_none() {
            eprintln!("git not installed; skipping");
            return;
        }
        let base = tmp();
        let root = base.join("thesis");
        fs::create_dir_all(root.join(".kerntex/out")).unwrap();
        fs::write(root.join("main.tex"), "\\documentclass{article}\n\\begin{document}\n\\section{Intro}\nHello\n\\end{document}\n")
            .unwrap();
        fs::write(root.join(".kerntex/out/main.pdf"), "pdf").unwrap();

        assert!(!status(&root).unwrap().repo);
        // `init` keeps a configured default branch (some Windows setups use
        // `master`) and only picks `main` when none is configured.
        let branch =
            try_run(&root, &["config", "--get", "init.defaultBranch"]).map(|b| b.trim().to_string()).unwrap_or_else(|| "main".into());
        init(&root).unwrap();
        local_identity(&root);
        assert_eq!(fs::read_to_string(root.join(".gitignore")).unwrap(), ".kerntex/out/\n");
        let st = status(&root).unwrap();
        assert!(st.repo && !st.has_commits);
        assert_eq!(st.branch.as_deref(), Some(branch.as_str()));
        assert!(st.changes.iter().all(|c| !c.path.starts_with(".kerntex/out")), "{:?}", st.changes);
        assert_eq!(suggest_message(&root).unwrap(), "Initial commit");
        assert!(diff(&root, "main.tex", None).unwrap().contains("+Hello"));

        let c = commit(&root, "Initial commit").unwrap();
        assert_eq!(c.subject, "Initial commit");
        assert!(status(&root).unwrap().changes.is_empty());
        assert_eq!(commit(&root, "again").unwrap_err().title, "Nothing to commit");

        // Edit inside a section → message names it.
        fs::write(root.join("main.tex"), "\\documentclass{article}\n\\begin{document}\n\\section{Intro}\nHello world\n\\end{document}\n")
            .unwrap();
        fs::write(root.join("plot.png"), "png").unwrap();
        assert_eq!(suggest_message(&root).unwrap(), "Edit “Intro”, add plot.png");
        assert!(diff(&root, "main.tex", None).unwrap().contains("+Hello world"));

        // Push to a bare remote.
        let remote = base.join("remote.git");
        git(&base, &["init", "-q", "--bare", "-b", &branch, "remote.git"]);
        assert!(push(&root, &GitCancel::default(), |_| {}).is_err(), "no remote yet");
        set_remote(&root, remote.to_str().unwrap()).unwrap();
        commit(&root, "Edit intro").unwrap();
        push(&root, &GitCancel::default(), |_| {}).unwrap();
        let st = status(&root).unwrap();
        assert_eq!(st.upstream.as_deref(), Some(format!("origin/{branch}").as_str()));
        assert_eq!(st.remote.as_deref(), Some("origin"));

        // Someone else changes the same line.
        git(&base, &["clone", "-q", "remote.git", "other"]);
        let other = base.join("other");
        local_identity(&other);
        fs::write(other.join("main.tex"), "\\documentclass{article}\n\\begin{document}\n\\section{Intro}\nHi there\n\\end{document}\n")
            .unwrap();
        git(&other, &["commit", "-qam", "Other edit"]);
        git(&other, &["push", "-q"]);

        fs::write(root.join("main.tex"), "\\documentclass{article}\n\\begin{document}\n\\section{Intro}\nHello KernTeX\n\\end{document}\n")
            .unwrap();
        commit(&root, "My edit").unwrap();
        let r = pull(&root, &GitCancel::default(), |_| {}).unwrap();
        assert_eq!(r.conflicts, vec!["main.tex".to_string()]);
        assert!(status(&root).unwrap().merging);
        abort_merge(&root).unwrap();
        assert!(!status(&root).unwrap().merging);

        fs::remove_dir_all(base).ok();
    }

    #[test]
    fn subfolder_project_commits_only_its_folder() {
        if version().is_none() {
            return;
        }
        let base = tmp();
        git(&base, &["init", "-q"]);
        local_identity(&base);
        let root = base.join("paper");
        fs::create_dir_all(&root).unwrap();
        fs::write(root.join("main.tex"), "x").unwrap();
        fs::write(base.join("outside.txt"), "y").unwrap();
        let st = status(&root).unwrap();
        assert_eq!(st.changes.iter().map(|c| c.path.as_str()).collect::<Vec<_>>(), vec!["main.tex"]);
        commit(&root, "Paper").unwrap();
        let out = Command::new("git").current_dir(&base).args(["status", "--porcelain"]).output().unwrap();
        assert_eq!(String::from_utf8_lossy(&out.stdout).trim(), "?? outside.txt");
        fs::remove_dir_all(base).ok();
    }
}
