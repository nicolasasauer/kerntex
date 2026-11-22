//! ZIP import (e.g. Overleaf "Download → Source") and export.

use super::error::{AppError, ErrorSource, Result};
use super::project::{self, BUILD_DIR};
use std::fs::{self, File};
use std::io;
use std::path::{Path, PathBuf};

fn zip_err(action: &str, path: &Path, e: impl std::fmt::Display) -> AppError {
    AppError::new(ErrorSource::Archive, format!("Couldn't {action} “{}”", path.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default()))
        .detail(e.to_string())
        .hint("Make sure it's a regular, complete .zip file (re-download it if unsure). Password-protected or split archives aren't supported.")
}

/// Extracts `zip_path` into a new folder inside `dest_parent` and returns it.
/// If the archive wraps everything in one top-level folder (GitHub style),
/// that folder is unwrapped.
pub fn import_zip(zip_path: &Path, dest_parent: &Path) -> Result<PathBuf> {
    let file = File::open(zip_path).map_err(|e| AppError::io("open", zip_path, e))?;
    let mut archive = zip::ZipArchive::new(file).map_err(|e| zip_err("read", zip_path, e))?;

    let names: Vec<PathBuf> = (0..archive.len())
        .filter_map(|i| archive.by_index(i).ok()?.enclosed_name())
        .collect();
    let strip = common_root(&names);

    let stem = zip_path.file_stem().map(|s| s.to_string_lossy().into_owned()).unwrap_or_else(|| "project".into());
    let dest = unique_dir(dest_parent, &stem);
    fs::create_dir_all(&dest).map_err(|e| AppError::io("create", &dest, e))?;

    for i in 0..archive.len() {
        let mut entry = archive.by_index(i).map_err(|e| zip_err("read", zip_path, e))?;
        // `enclosed_name` rejects absolute paths and `..` (zip-slip protection).
        let Some(name) = entry.enclosed_name() else { continue };
        let rel = match &strip {
            Some(prefix) => match name.strip_prefix(prefix) {
                Ok(r) if !r.as_os_str().is_empty() => r.to_path_buf(),
                _ => continue,
            },
            None => name,
        };
        if rel.components().any(|c| c.as_os_str() == "__MACOSX") {
            continue;
        }
        let out = dest.join(&rel);
        if entry.is_dir() {
            fs::create_dir_all(&out).map_err(|e| AppError::io("create", &out, e))?;
        } else {
            if let Some(p) = out.parent() {
                fs::create_dir_all(p).map_err(|e| AppError::io("create", p, e))?;
            }
            let mut f = File::create(&out).map_err(|e| AppError::io("write", &out, e))?;
            io::copy(&mut entry, &mut f).map_err(|e| AppError::io("write", &out, e))?;
        }
    }
    Ok(dest)
}

fn common_root(names: &[PathBuf]) -> Option<PathBuf> {
    let first = names.first()?.components().next()?.as_os_str().to_owned();
    let all_under = names.iter().all(|n| n.components().next().is_some_and(|c| c.as_os_str() == first));
    let has_nested = names.iter().any(|n| n.components().count() > 1);
    (all_under && has_nested).then(|| PathBuf::from(first))
}

fn unique_dir(parent: &Path, stem: &str) -> PathBuf {
    let mut candidate = parent.join(stem);
    let mut n = 2;
    while candidate.exists() {
        candidate = parent.join(format!("{stem}-{n}"));
        n += 1;
    }
    candidate
}

/// Writes the project to `dest`. With `clean`, LaTeX by-products are left out
/// (the `.bbl` is kept — arXiv and many journals need it).
pub fn export_zip(root: &Path, dest: &Path, clean: bool) -> Result<usize> {
    let file = File::create(dest).map_err(|e| AppError::io("create", dest, e))?;
    let mut zip = zip::ZipWriter::new(file);
    let opts = zip::write::SimpleFileOptions::default().compression_method(zip::CompressionMethod::Deflated);
    let mut count = 0;
    let mut stack = vec![root.to_path_buf()];
    while let Some(dir) = stack.pop() {
        let entries = fs::read_dir(&dir).map_err(|e| AppError::io("read", &dir, e))?;
        for entry in entries.flatten() {
            let path = entry.path();
            let name = entry.file_name().to_string_lossy().into_owned();
            if path == dest {
                continue;
            }
            if path.is_dir() {
                if [".git", BUILD_DIR, "node_modules"].contains(&name.as_str()) {
                    continue;
                }
                stack.push(path);
                continue;
            }
            if clean && (project::is_generated(&name) || name.ends_with(".kerntex-tmp")) {
                continue;
            }
            let Some(rel) = project::to_rel(root, &path) else { continue };
            zip.start_file(rel, opts).map_err(|e| zip_err("write", dest, e))?;
            let mut f = File::open(&path).map_err(|e| AppError::io("read", &path, e))?;
            io::copy(&mut f, &mut zip).map_err(|e| AppError::io("write", dest, e))?;
            count += 1;
        }
    }
    zip.finish().map_err(|e| zip_err("write", dest, e))?;
    Ok(count)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roundtrip_with_clean_and_unwrap() {
        let base = std::env::temp_dir().join(format!("kerntex-zip-{}", std::process::id()));
        let _ = fs::remove_dir_all(&base);
        let src = base.join("src");
        project::write_text(&src, "main.tex", "x").unwrap();
        project::write_text(&src, "main.aux", "x").unwrap();
        project::write_text(&src, "main.bbl", "x").unwrap();
        project::write_text(&src, "img/a.txt", "x").unwrap();
        project::write_text(&src, ".kerntex/out/main.pdf", "x").unwrap();
        let z = base.join("proj.zip");
        assert_eq!(export_zip(&src, &z, true).unwrap(), 3);

        let dest = import_zip(&z, &base).unwrap();
        assert_eq!(dest.file_name().unwrap(), "proj");
        assert!(dest.join("main.tex").is_file() && dest.join("img/a.txt").is_file() && dest.join("main.bbl").is_file());
        assert!(!dest.join("main.aux").exists());
        // Importing again must not overwrite.
        assert_eq!(import_zip(&z, &base).unwrap().file_name().unwrap(), "proj-2");
        fs::remove_dir_all(base).ok();
    }

    #[test]
    fn common_root_detection() {
        let p = |v: &[&str]| v.iter().map(PathBuf::from).collect::<Vec<_>>();
        assert_eq!(common_root(&p(&["repo/", "repo/main.tex", "repo/a/b.tex"])), Some(PathBuf::from("repo")));
        assert_eq!(common_root(&p(&["main.tex", "a/b.tex"])), None);
    }
}
