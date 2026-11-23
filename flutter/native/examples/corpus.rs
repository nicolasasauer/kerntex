//! Rendering corpus: compiles public LaTeX and Typst documents with the
//! KernTeX core and checks every page that hayro (the preview renderer)
//! draws against Poppler's `pdftoppm` as a reference.
//!
//! ```sh
//! cd flutter/native
//! cargo run --release --example corpus                  # everything
//! cargo run --release --example corpus -- --only typst  # ids containing "typst"
//! ```
//!
//! Sources are fetched with git at the commits pinned in `corpus/corpus.json`
//! into `target/corpus/src` (reused on later runs). Typst Universe packages
//! are installed from the `typst/packages` repository into a private package
//! cache, so the run works without access to packages.typst.org.
//! Output: `target/corpus/report.md`, `report.json`, and side-by-side images
//! (hayro | Poppler | differences) of each document's least similar page.

use image::{Rgb, RgbImage};
use kerntex_native::core::compile::{self, CancelHandle, CompileRequest, Engine, TexFlavor};
use kerntex_native::core::{settings, templates};
use kerntex_native::pdf;
use serde::Deserialize;
use serde_json::json;
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Instant;

#[derive(Deserialize)]
struct Manifest {
    sources: BTreeMap<String, Source>,
    documents: Vec<Doc>,
}

#[derive(Deserialize)]
struct Source {
    repo: String,
    commit: String,
    license: String,
    #[serde(default)]
    sparse: bool,
    #[allow(dead_code)]
    #[serde(default)]
    note: Option<String>,
}

#[derive(Deserialize, Clone)]
struct Doc {
    id: String,
    what: String,
    source: String,
    #[serde(default)]
    dir: String,
    #[serde(default)]
    main: String,
    #[serde(default)]
    glob: Option<String>,
    #[serde(default)]
    packages: Vec<String>,
    #[serde(default)]
    flavor: Option<TexFlavor>,
    /// Force an engine (e.g. latexmk for LuaLaTeX documents, which Tectonic can't build).
    #[serde(default)]
    engine: Option<Engine>,
    /// Why a document needs special handling (shown in the report).
    #[serde(default)]
    note: Option<String>,
}

/// A page is "different" where the grey levels differ by more than this.
const PIXEL_TOLERANCE: u8 = 80;
/// Share of inked pixels that may differ before a page is flagged.
const FLAG_SCORE: f64 = 0.05;

struct Opts {
    only: Option<String>,
    dpi: f32,
    out: PathBuf,
}

fn main() {
    let mut args = std::env::args().skip(1);
    let crate_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let mut opts = Opts { only: None, dpi: 100.0, out: crate_dir.join("target/corpus") };
    while let Some(a) = args.next() {
        match a.as_str() {
            "--only" => opts.only = args.next(),
            "--dpi" => opts.dpi = args.next().and_then(|d| d.parse().ok()).unwrap_or(100.0),
            "--out" => opts.out = args.next().map(PathBuf::from).unwrap_or(opts.out),
            _ => {
                eprintln!("usage: corpus [--only <id substring>] [--dpi 100] [--out <dir>]");
                std::process::exit(2);
            }
        }
    }
    let manifest: Manifest =
        serde_json::from_str(&fs::read_to_string(crate_dir.join("corpus/corpus.json")).expect("corpus.json")).expect("valid corpus.json");
    if Command::new("pdftoppm").arg("-v").output().is_err() {
        eprintln!("pdftoppm (Poppler) is needed as the reference renderer: apt install poppler-utils");
        std::process::exit(2);
    }
    let src_dir = opts.out.join("src");
    let work_dir = opts.out.join("work");
    let img_dir = opts.out.join("images");
    let _ = fs::remove_dir_all(&work_dir);
    let _ = fs::remove_dir_all(&img_dir);
    fs::create_dir_all(&img_dir).unwrap();

    // Typst looks for packages in the user cache; point it at ours.
    let cache = opts.out.join("cache");
    std::env::set_var("XDG_CACHE_HOME", &cache);
    if !cfg!(target_os = "linux") {
        eprintln!("note: on this OS Typst packages come from packages.typst.org, not the pinned repository");
    }

    // Expand globs and filter.
    let mut docs = Vec::new();
    for d in &manifest.documents {
        if let (Some(o), Some(_)) = (&opts.only, &d.glob) {
            let prefix = d.id.split('*').next().unwrap_or_default();
            if !d.id.contains(o.as_str()) && !o.starts_with(prefix) {
                continue;
            }
        }
        if let Some(g) = &d.glob {
            let root = checkout(&src_dir, &d.source, &manifest.sources[&d.source], &[]);
            for f in glob(&root, g) {
                let rel = f.strip_prefix(&root).unwrap();
                let dir = rel.parent().unwrap().to_string_lossy().replace('\\', "/");
                let name = rel.file_name().unwrap().to_string_lossy().into_owned();
                let id = d.id.replace('*', &dir);
                docs.push(Doc { id, dir, main: name, glob: None, ..d.clone() });
            }
        } else {
            docs.push(d.clone());
        }
    }
    docs.retain(|d| opts.only.as_ref().map_or(true, |o| d.id.contains(o.as_str())));

    let engine = if compile::find_tectonic().is_some() { Engine::Tectonic } else { Engine::Latexmk };
    println!("{} documents · LaTeX engine: {engine:?} · reference: pdftoppm · {} dpi\n", docs.len(), opts.dpi);

    let mut installed = BTreeSet::new();
    let mut results = Vec::new();
    for d in &docs {
        let work = work_dir.join(d.id.replace(['/', ' '], "_"));
        if d.source == "kerntex" {
            // KernTeX's own tours, created exactly as the app creates them.
            fs::create_dir_all(&work).unwrap();
            let made = match d.dir.as_str() {
                "typst-welcome" => templates::ensure_typst_welcome(&work, true),
                _ => templates::ensure_welcome(&work, true),
            }
            .expect("create tour");
            results.push(run(d, &made, engine, &opts, &img_dir));
            continue;
        }
        let root = match manifest.sources.get(&d.source) {
            Some(s) => checkout(&src_dir, &d.source, s, &[]),
            None => panic!("unknown source {}", d.source),
        };
        for p in &d.packages {
            install_template_package(&root.join(p), &cache, &mut installed);
        }
        if d.id.starts_with("typst") {
            install_dependencies(&root.join(&d.dir), &src_dir, &manifest, &cache, &mut installed);
        }
        copy_dir(&root.join(&d.dir), &work);
        results.push(run(d, &work, engine, &opts, &img_dir));
    }

    // Report.
    let mut md = String::from("# KernTeX rendering corpus\n\n");
    md += &format!("LaTeX engine: {engine:?} · reference renderer: Poppler `pdftoppm` · {} dpi · page flagged when more than {:.0} % of its inked pixels differ by more than {PIXEL_TOLERANCE}/255.\n\n", opts.dpi, FLAG_SCORE * 100.0);
    md += "| Document | Compile | Pages | hayro ms/page | Mean diff | Worst page | Status |\n|---|---|---|---|---|---|---|\n";
    let (mut ok, mut flagged, mut failed, mut pages) = (0, 0, 0, 0);
    for r in &results {
        pages += r.pages.len();
        let worst = r.pages.iter().max_by(|a, b| a.score.total_cmp(&b.score));
        let mean = if r.pages.is_empty() { 0.0 } else { r.pages.iter().map(|p| p.score).sum::<f64>() / r.pages.len() as f64 };
        let ms = if r.pages.is_empty() { 0.0 } else { r.pages.iter().map(|p| p.ms).sum::<f64>() / r.pages.len() as f64 };
        let status = if r.error.is_some() {
            failed += 1;
            "❌ compile"
        } else if r.pages.iter().any(|p| p.blank) {
            failed += 1;
            "❌ blank page"
        } else if worst.is_some_and(|w| w.score > FLAG_SCORE) {
            flagged += 1;
            "⚠️ differs"
        } else {
            ok += 1;
            "✅"
        };
        md += &format!(
            "| {} — {} | {} | {} | {:.0} | {:.2} % | {} | {} |\n",
            r.id,
            match &r.note {
                Some(n) => format!("{} *({:?}: {n})*", r.what, r.engine),
                None => r.what.clone(),
            },
            r.error.as_deref().map(|e| format!("failed: {}", e.replace('|', "/"))).unwrap_or(format!("{:.1} s", r.compile_ms / 1000.0)),
            r.pages.len(),
            ms,
            mean * 100.0,
            worst.map(|w| format!("p. {} ({:.2} %)", w.page + 1, w.score * 100.0)).unwrap_or_default(),
            status
        );
    }
    md += &format!("\n**{} documents, {pages} pages: {ok} ✅, {flagged} ⚠️, {failed} ❌**\n", results.len());
    md += "\nSources:\n\n";
    for (id, s) in &manifest.sources {
        md += &format!("- `{id}`: {} @ `{}` ({})\n", s.repo, &s.commit[..12], s.license);
    }
    fs::write(opts.out.join("report.md"), &md).unwrap();
    let json: Vec<_> = results
        .iter()
        .map(|r| json!({"id": r.id, "error": r.error, "compileMs": r.compile_ms, "pages": r.pages.iter().map(|p| json!({"score": p.score, "ms": p.ms, "blank": p.blank})).collect::<Vec<_>>()}))
        .collect();
    fs::write(opts.out.join("report.json"), serde_json::to_string_pretty(&json).unwrap()).unwrap();
    println!("\n{md}\nReport: {}", opts.out.join("report.md").display());
    if failed > 0 {
        std::process::exit(1);
    }
}

struct PageResult {
    page: usize,
    score: f64,
    ms: f64,
    blank: bool,
}

struct DocResult {
    id: String,
    what: String,
    engine: Engine,
    note: Option<String>,
    error: Option<String>,
    compile_ms: f64,
    pages: Vec<PageResult>,
}

fn run(d: &Doc, work: &Path, engine: Engine, opts: &Opts, img_dir: &Path) -> DocResult {
    let mut res = DocResult {
        id: d.id.clone(),
        what: d.what.clone(),
        engine: d.engine.unwrap_or(engine),
        note: d.note.clone(),
        error: None,
        compile_ms: 0.0,
        pages: Vec::new(),
    };
    print!("{:<40} ", d.id);
    let flavor = d.flavor.or_else(|| settings::build_hints(work, Some(&d.main)).flavor).unwrap_or(TexFlavor::Pdflatex);
    let req = CompileRequest { root: work.display().to_string(), main_file: d.main.clone(), engine: d.engine.unwrap_or(engine), flavor };
    let t = Instant::now();
    let out = compile::compile(&req, &CancelHandle::default(), |_| {});
    res.compile_ms = t.elapsed().as_secs_f64() * 1000.0;
    let pdf_path = match out {
        Ok(r) if r.pdf_path.is_some() => {
            if !r.ok {
                let first = r.diagnostics.iter().find(|d| format!("{:?}", d.severity) == "Error").map(|d| d.message.clone());
                println!("(with errors: {})", first.unwrap_or_default());
                print!("{:<40} ", "");
            }
            PathBuf::from(r.pdf_path.unwrap())
        }
        Ok(r) => {
            let first = r
                .diagnostics
                .iter()
                .find(|d| format!("{:?}", d.severity) == "Error")
                .map(|d| d.message.clone())
                .unwrap_or_else(|| "no PDF".into());
            println!("compile failed: {first}");
            res.error = Some(first);
            return res;
        }
        Err(e) => {
            println!("compile failed: {}", e.title);
            res.error = Some(e.title);
            return res;
        }
    };

    // Reference: Poppler, grey, same resolution.
    let ref_prefix = work.join(".kerntex/ref");
    let st = Command::new("pdftoppm").args(["-r", &opts.dpi.to_string(), "-gray"]).arg(&pdf_path).arg(&ref_prefix).status();
    if !st.is_ok_and(|s| s.success()) {
        res.error = Some("pdftoppm failed".into());
        println!("pdftoppm failed");
        return res;
    }
    let mut refs: Vec<PathBuf> = fs::read_dir(work.join(".kerntex"))
        .unwrap()
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| p.extension().is_some_and(|e| e == "pgm"))
        .collect();
    refs.sort_by_key(|p| p.file_stem().unwrap().to_string_lossy().rsplit('-').next().unwrap().parse::<usize>().unwrap_or(0));

    let n = pdf::info(&pdf_path).map(|i| i.page_count()).unwrap_or(0);
    let scale = opts.dpi / 72.0;
    let threads = std::thread::available_parallelism().map(|n| n.get()).unwrap_or(4).min(8);
    let mut pages: Vec<(PageResult, Option<(Gray, Gray)>)> = Vec::new();
    std::thread::scope(|s| {
        let handles: Vec<_> = (0..threads)
            .map(|t| {
                let (refs, pdf_path) = (&refs, &pdf_path);
                s.spawn(move || {
                    let mut out = Vec::new();
                    for i in (t..n).step_by(threads) {
                        let t0 = Instant::now();
                        let Some((w, h, rgba)) = pdf::render(pdf_path, i, scale) else {
                            out.push((PageResult { page: i, score: 1.0, ms: 0.0, blank: true }, None));
                            continue;
                        };
                        let ms = t0.elapsed().as_secs_f64() * 1000.0;
                        let ours = Gray::from_rgba(w, h, &rgba);
                        let Some(theirs) = refs.get(i).and_then(|p| Gray::read_pgm(p)) else { continue };
                        let (score, ours_ink, theirs_ink) = compare(&ours, &theirs);
                        let blank = ours_ink < 0.0001 && theirs_ink > 0.001;
                        out.push((PageResult { page: i, score, ms, blank }, Some((ours, theirs))));
                    }
                    out
                })
            })
            .collect();
        for h in handles {
            pages.extend(h.join().unwrap());
        }
    });
    pages.sort_by_key(|p| p.0.page);
    let worst = pages.iter().enumerate().max_by(|a, b| a.1 .0.score.total_cmp(&b.1 .0.score)).map(|(i, _)| i);
    if let Some(i) = worst {
        if let Some((a, b)) = &pages[i].1 {
            side_by_side(a, b).save(img_dir.join(format!("{}-p{}.png", d.id.replace(['/', ' '], "_"), pages[i].0.page + 1))).ok();
        }
    }
    res.pages = pages.into_iter().map(|p| p.0).collect();
    let worst = res.pages.iter().map(|p| p.score).fold(0.0, f64::max);
    println!(
        "{:>5.1} s  {:>3} pages  worst diff {:>5.2} %{}",
        res.compile_ms / 1000.0,
        res.pages.len(),
        worst * 100.0,
        if res.pages.iter().any(|p| p.blank) { "  BLANK PAGE" } else { "" }
    );
    res
}

// ── Images ───────────────────────────────────────────────────────────────

struct Gray {
    w: u32,
    h: u32,
    px: Vec<u8>,
}

impl Gray {
    fn from_rgba(w: u32, h: u32, rgba: &[u8]) -> Self {
        let px = rgba.chunks_exact(4).map(|c| ((c[0] as u32 * 299 + c[1] as u32 * 587 + c[2] as u32 * 114) / 1000) as u8).collect();
        Gray { w, h, px }
    }

    /// Binary PGM (P5) as written by pdftoppm -gray.
    fn read_pgm(path: &Path) -> Option<Self> {
        let data = fs::read(path).ok()?;
        let mut fields = Vec::new();
        let mut i = 0;
        while fields.len() < 4 && i < data.len() {
            while i < data.len() && data[i].is_ascii_whitespace() {
                i += 1;
            }
            if data.get(i) == Some(&b'#') {
                while i < data.len() && data[i] != b'\n' {
                    i += 1;
                }
                continue;
            }
            let s = i;
            while i < data.len() && !data[i].is_ascii_whitespace() {
                i += 1;
            }
            fields.push(String::from_utf8_lossy(&data[s..i]).into_owned());
        }
        if fields.first()? != "P5" {
            return None;
        }
        let (w, h): (u32, u32) = (fields[1].parse().ok()?, fields[2].parse().ok()?);
        let px = data.get(i + 1..i + 1 + (w * h) as usize)?.to_vec();
        Some(Gray { w, h, px })
    }

    fn at(&self, x: u32, y: u32) -> u8 {
        self.px[(y * self.w + x) as usize]
    }
}

/// Share of inked pixels that differ, plus both pages' ink coverage.
/// Compared on 2×2 averaged blocks (anti-aliasing doesn't count), and a
/// block only counts as different if no block within one step of it in the
/// other image matches — so sub-pixel placement differences don't count,
/// missing or wrong content does.
fn compare(a: &Gray, b: &Gray) -> (f64, f64, f64) {
    let (w, h) = (a.w.min(b.w) / 2, a.h.min(b.h) / 2);
    let avg = |g: &Gray, x: u32, y: u32| -> u8 {
        ((g.at(2 * x, 2 * y) as u32 + g.at(2 * x + 1, 2 * y) as u32 + g.at(2 * x, 2 * y + 1) as u32 + g.at(2 * x + 1, 2 * y + 1) as u32)
            / 4) as u8
    };
    let da: Vec<u8> = (0..h).flat_map(|y| (0..w).map(move |x| (x, y))).map(|(x, y)| avg(a, x, y)).collect();
    let db: Vec<u8> = (0..h).flat_map(|y| (0..w).map(move |x| (x, y))).map(|(x, y)| avg(b, x, y)).collect();
    let near = |img: &[u8], x: u32, y: u32, v: u8| -> bool {
        for yy in y.saturating_sub(1)..=(y + 1).min(h - 1) {
            for xx in x.saturating_sub(1)..=(x + 1).min(w - 1) {
                if img[(yy * w + xx) as usize].abs_diff(v) <= PIXEL_TOLERANCE {
                    return true;
                }
            }
        }
        false
    };
    let (mut ink, mut bad, mut ink_a, mut ink_b) = (0u64, 0u64, 0u64, 0u64);
    for y in 0..h {
        for x in 0..w {
            let i = (y * w + x) as usize;
            let (p, q) = (da[i], db[i]);
            let (ia, ib) = (p < 224, q < 224);
            ink_a += ia as u64;
            ink_b += ib as u64;
            if ia || ib {
                ink += 1;
                if p.abs_diff(q) > PIXEL_TOLERANCE && (!near(&db, x, y, p) || !near(&da, x, y, q)) {
                    bad += 1;
                }
            }
        }
    }
    let total = (w as f64 * h as f64).max(1.0);
    (bad as f64 / ink.max(1) as f64, ink_a as f64 / total, ink_b as f64 / total)
}

/// hayro | Poppler | differences (red), for looking at what differs.
fn side_by_side(a: &Gray, b: &Gray) -> RgbImage {
    let (w, h) = (a.w.min(b.w), a.h.min(b.h));
    let mut img = RgbImage::from_pixel(w * 3 + 20, h, Rgb([200, 200, 200]));
    for y in 0..h {
        for x in 0..w {
            let (p, q) = (a.at(x, y), b.at(x, y));
            img.put_pixel(x, y, Rgb([p, p, p]));
            img.put_pixel(w + 10 + x, y, Rgb([q, q, q]));
            let d = p.abs_diff(q);
            let c = if d > PIXEL_TOLERANCE {
                Rgb([220, 30, 60])
            } else {
                let v = 255 - (255 - p.min(q)) / 4;
                Rgb([v, v, v])
            };
            img.put_pixel(2 * w + 20 + x, y, c);
        }
    }
    img
}

// ── Fetching ─────────────────────────────────────────────────────────────

fn git(dir: &Path, args: &[&str]) {
    let st = Command::new("git").arg("-C").arg(dir).args(args).status().expect("git is needed to fetch the corpus");
    assert!(st.success(), "git {args:?} failed in {}", dir.display());
}

/// Checks out `src` at its pinned commit (blobless; sparse if requested).
fn checkout(dir: &Path, id: &str, src: &Source, sparse_add: &[String]) -> PathBuf {
    let d = dir.join(id);
    let marker = d.join(".git/kerntex-commit");
    if !d.join(".git").exists() {
        fs::create_dir_all(&d).unwrap();
        git(&d, &["init", "-q"]);
        git(&d, &["remote", "add", "origin", &src.repo]);
        if src.sparse {
            git(&d, &["sparse-checkout", "set", "--cone", "typst.toml"]);
        }
    }
    if !sparse_add.is_empty() {
        let mut a = vec!["sparse-checkout", "add"];
        a.extend(sparse_add.iter().map(String::as_str));
        git(&d, &a);
    }
    if fs::read_to_string(&marker).ok().as_deref() != Some(src.commit.as_str()) {
        println!("fetching {id} @ {}", &src.commit[..12]);
        git(&d, &["fetch", "-q", "--depth", "1", "--filter=blob:none", "origin", &src.commit]);
        git(&d, &["checkout", "-q", "--force", "FETCH_HEAD"]);
        fs::write(&marker, &src.commit).unwrap();
    } else if !sparse_add.is_empty() {
        git(&d, &["checkout", "-q", "--force", "FETCH_HEAD"]);
    }
    d
}

fn toml_field(text: &str, key: &str) -> Option<String> {
    text.lines().find_map(|l| {
        let l = l.trim();
        let rest = l.strip_prefix(key)?.trim_start().strip_prefix('=')?;
        Some(rest.trim().trim_matches('"').to_string())
    })
}

/// Installs a package directory (with typst.toml) as @preview/name:version.
fn install_template_package(dir: &Path, cache: &Path, installed: &mut BTreeSet<String>) {
    let toml = fs::read_to_string(dir.join("typst.toml")).expect("typst.toml");
    let (name, version) = (toml_field(&toml, "name").unwrap(), toml_field(&toml, "version").unwrap());
    copy_dir(dir, &cache.join(format!("typst/packages/preview/{name}/{version}")));
    installed.insert(format!("{name}:{version}"));
}

/// `@preview/name:1.2.3` imports in all .typ files below `dir`.
fn preview_imports(dir: &Path) -> BTreeSet<String> {
    let mut out = BTreeSet::new();
    for f in glob(dir, "**/*.typ") {
        let t = fs::read_to_string(f).unwrap_or_default();
        let mut rest = t.as_str();
        while let Some(i) = rest.find("@preview/") {
            rest = &rest[i + 9..];
            let spec: String = rest.chars().take_while(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | ':' | '.')).collect();
            let spec = spec.trim_end_matches('.');
            // name:1.2.3 only (skips prose such as "@preview/name:version").
            if let Some((_, v)) = spec.split_once(':') {
                if v.split('.').count() == 3 && v.split('.').all(|p| !p.is_empty() && p.bytes().all(|b| b.is_ascii_digit())) {
                    out.insert(spec.to_string());
                }
            }
        }
    }
    out
}

/// Installs every Universe package a document needs, transitively, from the
/// pinned typst/packages repository.
fn install_dependencies(doc_dir: &Path, src_dir: &Path, m: &Manifest, cache: &Path, installed: &mut BTreeSet<String>) {
    let mut todo: Vec<String> = preview_imports(doc_dir).into_iter().collect();
    for spec in installed.clone() {
        let (n, v) = spec.split_once(':').unwrap();
        todo.extend(preview_imports(&cache.join(format!("typst/packages/preview/{n}/{v}"))));
    }
    while let Some(spec) = todo.pop() {
        if installed.contains(&spec) {
            continue;
        }
        let (n, v) = spec.split_once(':').unwrap();
        let path = format!("packages/preview/{n}/{v}");
        let repo = checkout(src_dir, "typst-packages", &m.sources["typst-packages"], std::slice::from_ref(&path));
        let dir = repo.join(&path);
        assert!(dir.is_dir(), "package {spec} not found in typst/packages");
        copy_dir(&dir, &cache.join(format!("typst/packages/preview/{n}/{v}")));
        installed.insert(spec.clone());
        todo.extend(preview_imports(&dir));
    }
}

fn copy_dir(from: &Path, to: &Path) {
    fs::create_dir_all(to).unwrap();
    for e in fs::read_dir(from).unwrap_or_else(|e| panic!("{}: {e}", from.display())) {
        let e = e.unwrap();
        let name = e.file_name();
        if name == ".git" || name == ".kerntex" {
            continue;
        }
        let p = e.path();
        if p.is_dir() {
            copy_dir(&p, &to.join(&name));
        } else {
            fs::copy(&p, to.join(&name)).unwrap();
        }
    }
}

/// Minimal glob: `*` within one path segment, `**` for any depth.
fn glob(root: &Path, pattern: &str) -> Vec<PathBuf> {
    fn matches(seg: &str, pat: &str) -> bool {
        match pat.split_once('*') {
            None => seg == pat,
            Some((pre, post)) => seg.len() >= pre.len() + post.len() && seg.starts_with(pre) && seg.ends_with(post),
        }
    }
    fn walk(dir: &Path, parts: &[&str], out: &mut Vec<PathBuf>) {
        let Some((first, rest)) = parts.split_first() else { return };
        let Ok(rd) = fs::read_dir(dir) else { return };
        let mut entries: Vec<_> = rd.filter_map(|e| e.ok()).collect();
        entries.sort_by_key(|e| e.file_name());
        for e in entries {
            let name = e.file_name().to_string_lossy().into_owned();
            if name.starts_with('.') {
                continue;
            }
            let p = e.path();
            if *first == "**" {
                if p.is_dir() {
                    walk(&p, parts, out);
                } else if rest.len() == 1 && matches(&name, rest[0]) {
                    out.push(p.clone());
                }
                continue;
            }
            if !matches(&name, first) {
                continue;
            }
            if rest.is_empty() {
                if p.is_file() {
                    out.push(p);
                }
            } else if p.is_dir() {
                walk(&p, rest, out);
            }
        }
    }
    let parts: Vec<&str> = pattern.split('/').collect();
    let mut out = Vec::new();
    walk(root, &parts, &mut out);
    out
}
