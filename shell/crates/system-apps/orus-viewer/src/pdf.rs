//! PDF pages as PNG files, for the canvas to draw like any other image.
//!
//! Pages are rendered by pdfium when its library is found (`ORUS_PDFIUM`, then
//! `/usr/lib/orus/libpdfium.so`, then beside the program, then the system's), and by
//! `hayro` (pure Rust, CPU) when it is not. hayro cannot open password-protected files
//! and has known gaps; pdfium has neither. The web canvas pdf.js needs is not in Detir
//! yet, so this stands in for it.

use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::path::{Path, PathBuf};
use std::sync::{Arc, OnceLock};

use hayro::hayro_interpret::InterpreterSettings;
use hayro::hayro_syntax::Pdf;
use hayro::vello_cpu::color::palette::css::WHITE;
use hayro::{PixmapSettings, RenderCache, RenderSettings, render};
use pdfium_render::prelude::*;

/// Smallest and largest scale from PDF points to pixels. A page far outside this is
/// either unreadable or too big to hold as a bitmap.
const MIN_SCALE: f32 = 0.05;
const MAX_SCALE: f32 = 4.0;

/// Where the Debian package puts the library.
const INSTALLED_PDFIUM: &str = "/usr/lib/orus/libpdfium.so";

/// A rendered page and the length of its document.
pub struct Rendered {
    pub png: PathBuf,
    pub page_count: usize,
}

/// Where rendered pages are kept: `~/.orus/cache/viewer`.
pub fn cache_dir() -> PathBuf {
    std::env::var_os("HOME")
        .map(PathBuf::from)
        .unwrap_or_default()
        .join(".orus/cache/viewer")
}

/// The cache file for one page at one size. The name carries the file's size and
/// modified time, so an edited PDF is rendered again.
fn cache_file(dir: &Path, source: &Path, page: usize, box_px: u32) -> std::io::Result<PathBuf> {
    let meta = std::fs::metadata(source)?;
    let modified = meta
        .modified()?
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let mut hasher = DefaultHasher::new();
    source.hash(&mut hasher);
    (meta.len(), modified).hash(&mut hasher);
    Ok(dir.join(format!("{:016x}-p{page}-{box_px}.png", hasher.finish())))
}

/// Renders pages with pdfium or, without it, hayro. pdfium-render serialises calls into
/// the library, so the render threads share one.
pub struct Renderer {
    pdfium: Option<Pdfium>,
}

static SHARED: OnceLock<Renderer> = OnceLock::new();

impl Renderer {
    /// The renderer of this process. pdfium-render binds the library once per process,
    /// so every caller shares this one.
    pub fn shared() -> &'static Renderer {
        SHARED.get_or_init(Renderer::load)
    }

    /// Look for the pdfium library, and use hayro if there is none.
    fn load() -> Self {
        let from_env = std::env::var_os("ORUS_PDFIUM").map(PathBuf::from);
        let beside = std::env::current_exe()
            .ok()
            .and_then(|exe| exe.parent().map(|dir| dir.join("libpdfium.so")));
        let candidates = from_env
            .into_iter()
            .chain([PathBuf::from(INSTALLED_PDFIUM)])
            .chain(beside);
        for path in candidates {
            if !path.is_file() {
                continue;
            }
            match Pdfium::bind_to_library(&path) {
                Ok(bindings) => {
                    return Self {
                        pdfium: Some(Pdfium::new(bindings)),
                    };
                }
                Err(e) => {
                    log::warn!("cannot load pdfium from {}: {e}", path.display())
                }
            }
        }
        let pdfium = Pdfium::bind_to_system_library().ok().map(Pdfium::new);
        Self { pdfium }
    }

    /// Which library draws the pages.
    pub fn engine(&self) -> &'static str {
        if self.pdfium.is_some() {
            "pdfium"
        } else {
            "hayro"
        }
    }

    /// Render page `page` (0-based) of `source` to fit a square of `box_px` pixels, and
    /// give the number of pages. A page already in `dir` is reused (the count is read
    /// each time).
    pub fn render_page(
        &self,
        source: &Path,
        page: usize,
        box_px: u32,
        dir: &Path,
    ) -> Result<Rendered, String> {
        let png = cache_file(dir, source, page, box_px)
            .map_err(|e| format!("cannot read the file: {e}"))?;
        let (bytes, page_count) = match &self.pdfium {
            Some(pdfium) => draw_with_pdfium(pdfium, source, page, box_px, &png)?,
            None => draw_with_hayro(source, page, box_px, &png)?,
        };
        if let Some(bytes) = bytes {
            std::fs::create_dir_all(dir)
                .map_err(|e| format!("cannot write {}: {e}", dir.display()))?;
            // Write beside the final name, then rename, so the canvas never reads half a file.
            let partial = png.with_extension("png.partial");
            std::fs::write(&partial, bytes).map_err(|e| e.to_string())?;
            std::fs::rename(&partial, &png).map_err(|e| e.to_string())?;
        }
        Ok(Rendered { png, page_count })
    }
}

fn out_of_range(page: usize, count: usize) -> String {
    format!("the PDF has {count} pages, not {}", page + 1)
}

/// The PNG bytes of the page, or `None` when `png` is already there, and the page count.
fn draw_with_pdfium(
    pdfium: &Pdfium,
    source: &Path,
    page: usize,
    box_px: u32,
    png: &Path,
) -> Result<(Option<Vec<u8>>, usize), String> {
    let document = pdfium
        .load_pdf_from_file(source, None)
        .map_err(|e| match e {
            PdfiumError::PdfiumLibraryInternalError(PdfiumInternalError::PasswordError) => {
                "this PDF needs a password, which the Viewer cannot ask for yet".to_string()
            }
            other => format!("cannot open this PDF: {other}"),
        })?;
    let pages = document.pages();
    let count = pages.len() as usize;
    if png.is_file() {
        return Ok((None, count));
    }
    let index = i32::try_from(page).map_err(|_| out_of_range(page, count))?;
    let target = pages.get(index).map_err(|_| out_of_range(page, count))?;
    let (width, height) = (target.width().value, target.height().value);
    let scale = (box_px as f32 / width.max(height)).clamp(MIN_SCALE, MAX_SCALE);
    let bitmap = target
        .render_with_config(
            &PdfRenderConfig::new()
                .set_target_width((width * scale).round().max(1.0) as i32)
                .set_maximum_height((height * scale).round().max(1.0) as i32),
        )
        .map_err(|e| format!("cannot draw the page: {e}"))?;
    let mut bytes = Vec::new();
    bitmap
        .as_image()
        .map_err(|e| format!("cannot read the page's pixels: {e}"))?
        .write_to(
            &mut std::io::Cursor::new(&mut bytes),
            image::ImageFormat::Png,
        )
        .map_err(|e| format!("cannot encode the page: {e}"))?;
    Ok((Some(bytes), count))
}

fn draw_with_hayro(
    source: &Path,
    page: usize,
    box_px: u32,
    png: &Path,
) -> Result<(Option<Vec<u8>>, usize), String> {
    let data = std::fs::read(source).map_err(|e| format!("cannot read the file: {e}"))?;
    let pdf = Pdf::new(Arc::new(data)).map_err(|e| format!("cannot open this PDF: {e:?}"))?;
    let pages = pdf.pages();
    let count = pages.len();
    let target = pages.get(page).ok_or_else(|| out_of_range(page, count))?;
    if png.is_file() {
        return Ok((None, count));
    }
    let (width, height) = target.render_dimensions();
    let scale = (box_px as f32 / width.max(height)).clamp(MIN_SCALE, MAX_SCALE);
    let pixmap = render(
        target,
        &RenderCache::new(),
        &InterpreterSettings::default(),
        &RenderSettings::default(),
        &PixmapSettings {
            x_scale: scale,
            y_scale: scale,
            bg_color: WHITE,
        },
    );
    let bytes = pixmap
        .into_png()
        .map_err(|e| format!("cannot encode the page: {e:?}"))?;
    Ok((Some(bytes), count))
}
