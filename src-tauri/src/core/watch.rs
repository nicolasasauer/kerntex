//! Notices files changed outside KernTeX (Git, another editor, sync tools).
//!
//! Watches the project folder recursively, ignores KernTeX's own build
//! output and temp files and Git's internals, and reports the changed
//! project-relative paths in batches once things have been quiet for a moment
//! (a `git checkout` touches many files at once). KernTeX's own saves are
//! reported too; the UI recognises them because the content on disk equals
//! what it saved.

use super::error::{AppError, ErrorSource, Result};
use super::project;
use notify::{RecursiveMode, Watcher as _};
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::sync::mpsc::{self, RecvTimeoutError};
use std::time::{Duration, Instant};

/// How long the folder must be quiet before a batch is reported.
const QUIET: Duration = Duration::from_millis(250);
/// Report at the latest after this long, even while changes keep coming.
const MAX_DELAY: Duration = Duration::from_secs(2);

/// Stops watching when dropped.
pub struct ProjectWatcher {
    _inner: notify::RecommendedWatcher,
}

/// Starts watching `root`. `on_change` gets sorted, de-duplicated,
/// project-relative paths (`/`-separated) from a background thread.
pub fn watch(root: &Path, on_change: impl Fn(Vec<String>) + Send + 'static) -> Result<ProjectWatcher> {
    let root = root.canonicalize().map_err(|e| AppError::io("watch", root, e))?;
    let (tx, rx) = mpsc::channel::<PathBuf>();
    let mut inner = notify::recommended_watcher(move |res: notify::Result<notify::Event>| {
        if let Ok(event) = res {
            if matches!(event.kind, notify::EventKind::Access(_)) {
                return;
            }
            for p in event.paths {
                let _ = tx.send(p);
            }
        }
    })
    .map_err(watch_error)?;
    inner.watch(&root, RecursiveMode::Recursive).map_err(watch_error)?;

    std::thread::Builder::new()
        .name("kerntex-watch".into())
        .spawn(move || batch(&root, rx, on_change))
        .map_err(|e| AppError::new(ErrorSource::App, "Couldn't start watching the project folder").detail(e.to_string()))?;
    Ok(ProjectWatcher { _inner: inner })
}

/// Collects paths until the folder is quiet, then reports them. Ends when
/// the watcher (the sender) is dropped.
fn batch(root: &Path, rx: mpsc::Receiver<PathBuf>, on_change: impl Fn(Vec<String>)) {
    let mut pending = BTreeSet::new();
    let mut since: Option<Instant> = None;
    loop {
        let timeout = match since {
            Some(t) => QUIET.min(MAX_DELAY.saturating_sub(t.elapsed())),
            None => Duration::from_secs(3600),
        };
        match rx.recv_timeout(timeout) {
            Ok(p) => {
                if let Some(rel) = relevant(root, &p) {
                    pending.insert(rel);
                    since.get_or_insert_with(Instant::now);
                }
            }
            Err(RecvTimeoutError::Timeout) => {
                if !pending.is_empty() {
                    on_change(std::mem::take(&mut pending).into_iter().collect());
                }
                since = None;
            }
            Err(RecvTimeoutError::Disconnected) => return,
        }
    }
}

/// Project-relative path of a change worth reporting.
fn relevant(root: &Path, abs: &Path) -> Option<String> {
    let rel = project::to_rel(root, abs)?;
    if rel.is_empty() {
        return None;
    }
    let first = rel.split('/').next().unwrap_or("");
    // Build output, settings written by KernTeX, Git's internals.
    if first == ".kerntex" || first == ".git" {
        return None;
    }
    // KernTeX's atomic saves write `<name>.<ext>.kerntex-tmp` first.
    if rel.ends_with(".kerntex-tmp") {
        return None;
    }
    Some(rel)
}

fn watch_error(e: notify::Error) -> AppError {
    AppError::new(ErrorSource::FileSystem, "KernTeX can't watch this folder for changes")
        .detail(e.to_string())
        .hint("Files changed by other programs (e.g. Git) won't show up by themselves. Reopen the project to see them. On Linux, raising fs.inotify.max_user_watches helps with very large folders.")
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{Arc, Mutex};

    fn temp(tag: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!("kerntex-watch-{tag}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(d.join(".kerntex/out")).unwrap();
        std::fs::create_dir_all(d.join("chapters")).unwrap();
        d
    }

    /// Waits until a batch arrives (or a few seconds pass).
    fn wait(seen: &Arc<Mutex<Vec<Vec<String>>>>) -> Vec<String> {
        for _ in 0..60 {
            std::thread::sleep(Duration::from_millis(100));
            let mut s = seen.lock().unwrap();
            if !s.is_empty() {
                return s.drain(..).flatten().collect();
            }
        }
        vec![]
    }

    #[test]
    fn reports_project_files_in_batches_and_skips_build_output() {
        let root = temp("batch");
        let seen = Arc::new(Mutex::new(Vec::new()));
        let s = seen.clone();
        let _w = watch(&root, move |paths| s.lock().unwrap().push(paths)).unwrap();

        std::fs::write(root.join(".kerntex/out/main.log"), "log").unwrap();
        std::fs::write(root.join("main.tex.kerntex-tmp"), "x").unwrap();
        std::fs::write(root.join("main.tex"), "a").unwrap();
        std::fs::write(root.join("chapters/intro.tex"), "b").unwrap();
        let mut got = wait(&seen);
        got.dedup();
        assert!(got.contains(&"main.tex".to_string()), "{got:?}");
        assert!(got.contains(&"chapters/intro.tex".to_string()), "{got:?}");
        assert!(got.iter().all(|p| !p.starts_with(".kerntex") && !p.ends_with(".kerntex-tmp")), "{got:?}");

        // A deletion is reported as well.
        std::fs::remove_file(root.join("chapters/intro.tex")).unwrap();
        assert!(wait(&seen).contains(&"chapters/intro.tex".to_string()));
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn stops_when_dropped() {
        let root = temp("drop");
        let seen = Arc::new(Mutex::new(Vec::new()));
        let s = seen.clone();
        drop(watch(&root, move |paths| s.lock().unwrap().push(paths)).unwrap());
        std::fs::write(root.join("main.tex"), "a").unwrap();
        assert!(wait(&seen).is_empty());
        let _ = std::fs::remove_dir_all(&root);
    }
}
