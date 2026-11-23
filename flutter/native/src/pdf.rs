//! PDF page rasterisation with hayro (pure Rust, already part of Typst's
//! dependency tree, so it adds no new code to the binary).
//!
//! The parsed document is cached per path + modification time, so scrolling
//! and zooming only re-render pages; a rebuild invalidates the cache.

use hayro::hayro_interpret::InterpreterSettings;
use hayro::hayro_syntax::Pdf;
use hayro::vello_cpu::color::palette::css::WHITE;
use hayro::{RenderCache, RenderSettings};
use serde::Serialize;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::SystemTime;

struct Cached {
    path: PathBuf,
    mtime: SystemTime,
    pdf: Arc<Pdf>,
}

static CACHE: Mutex<Option<Cached>> = Mutex::new(None);

/// Largest page bitmap edge in pixels (keeps a deep zoom from allocating gigabytes).
const MAX_EDGE: f32 = 8192.0;

fn load(path: &Path) -> Option<Arc<Pdf>> {
    let mtime = std::fs::metadata(path).and_then(|m| m.modified()).ok()?;
    let mut cache = CACHE.lock().unwrap();
    if let Some(c) = cache.as_ref().filter(|c| c.path == path && c.mtime == mtime) {
        return Some(c.pdf.clone());
    }
    let pdf = Arc::new(Pdf::new(std::fs::read(path).ok()?).ok()?);
    *cache = Some(Cached { path: path.to_path_buf(), mtime, pdf: pdf.clone() });
    Some(pdf)
}

/// Drops the cached document (after a rebuild wrote a new file).
pub fn forget(path: &Path) {
    let mut cache = CACHE.lock().unwrap();
    if cache.as_ref().is_some_and(|c| c.path == path) {
        *cache = None;
    }
}

#[derive(Serialize)]
pub struct Info {
    /// Page sizes in PDF points, `[width, height]`.
    pages: Vec<[f32; 2]>,
}

pub fn info(path: &Path) -> Option<Info> {
    let pdf = load(path)?;
    Some(Info { pages: pdf.pages().iter().map(|p| p.render_dimensions().into()).collect() })
}

pub fn render(path: &Path, page: usize, scale: f32) -> Option<(u32, u32, Vec<u8>)> {
    let pdf = load(path)?;
    let page = pdf.pages().get(page)?;
    let (w, h) = page.render_dimensions();
    let scale = scale.clamp(0.05, MAX_EDGE / w.max(h).max(1.0));
    let settings = RenderSettings { x_scale: scale, y_scale: scale, bg_color: WHITE, ..Default::default() };
    let pixmap = hayro::render(page, &RenderCache::new(), &InterpreterSettings::default(), &settings);
    // White background: every pixel is opaque, so premultiplied == straight RGBA.
    Some((pixmap.width() as u32, pixmap.height() as u32, pixmap.data_as_u8_slice().to_vec()))
}
