//! PDFium-backed import, text, rendering and export.
//!
//! PDFium is not thread-safe, so one engine thread owns it and every parsed document.
//! [`Engine`] is a cheap, cloneable handle whose calls block until that thread answers;
//! UI code calls it from a background executor.

mod draw;
mod frame;
mod sample;

use std::collections::HashMap;
use std::io::Write as _;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::mpsc;

use pdfium_render::prelude::*;
use refr_core::workspace_json::{self, WorkspaceError};
use refr_core::{PdfPageState, PdfSource, PdfWorkspace, RectD, Uuid, layout};

pub use frame::{Matrix, PageFrame};

/// Largest rendered bitmap, in pixels. Callers lower the scale for bigger requests.
pub const MAX_BITMAP_PIXELS: u64 = 48_000_000;
pub const MAX_PNG_SIDE: f64 = 4096.0;
pub const MAX_SPLIT_PAGES: usize = 300;
pub const MAX_ARCHIVE_BYTES: usize = 256 * 1024 * 1024;

#[derive(Debug, thiserror::Error)]
pub enum PdfError {
    #[error("PDFium could not be loaded from {path}: {message}")]
    Library { path: PathBuf, message: String },
    #[error("PDFium: {0}")]
    Pdfium(String),
    #[error(transparent)]
    Workspace(#[from] WorkspaceError),
    #[error("{0}")]
    Invalid(String),
    #[error("The PDF engine has stopped.")]
    Stopped,
}

impl From<PdfiumError> for PdfError {
    fn from(error: PdfiumError) -> Self {
        match error {
            PdfiumError::PdfiumLibraryInternalError(PdfiumInternalError::PasswordError) => {
                PdfError::Invalid("This PDF is password protected. Password-protected documents are not supported yet.".into())
            }
            PdfiumError::PdfiumLibraryInternalError(PdfiumInternalError::FormatError) => {
                PdfError::Invalid("This file is not a readable PDF document.".into())
            }
            other => PdfError::Pdfium(format!("{other:?}")),
        }
    }
}

fn invalid<T>(message: impl Into<String>) -> Result<T, PdfError> {
    Err(PdfError::Invalid(message.into()))
}

/// A rendered page in PDFium's native BGRA order, which is also what GPUI's images expect.
#[derive(Clone)]
pub struct Bitmap {
    pub width: u32,
    pub height: u32,
    pub bgra: Vec<u8>,
}

impl Bitmap {
    pub fn to_png(&self) -> Result<Vec<u8>, PdfError> {
        let mut rgba = self.bgra.clone();
        for px in rgba.chunks_exact_mut(4) {
            px.swap(0, 2);
        }
        let image = image::RgbaImage::from_raw(self.width, self.height, rgba).ok_or_else(|| PdfError::Invalid("Invalid bitmap".into()))?;
        let mut out = std::io::Cursor::new(Vec::new());
        image.write_to(&mut out, image::ImageFormat::Png).map_err(|e| PdfError::Invalid(e.to_string()))?;
        Ok(out.into_inner())
    }
}

/// A word and its bounds in logical page coordinates.
#[derive(Clone, Debug, PartialEq)]
pub struct PdfWord {
    pub text: String,
    pub bounds: RectD,
}

#[derive(Clone, Debug, PartialEq)]
pub struct SearchResult {
    pub page_index: usize,
    pub excerpt: String,
    pub bounds: Vec<RectD>,
    /// Set when the match is in an annotation's text rather than the page content.
    pub annotation: Option<Uuid>,
}

type Job = Box<dyn FnOnce(&mut State) + Send>;

#[derive(Clone)]
pub struct Engine {
    jobs: mpsc::Sender<Job>,
}

struct Loaded {
    // Field order matters: the document borrows `_bytes` and must drop first.
    document: PdfDocument<'static>,
    _bytes: Arc<[u8]>,
    last_used: u64,
}

pub struct State {
    pdfium: &'static Pdfium,
    loaded: HashMap<(Uuid, usize), Loaded>,
    clock: u64,
}

const OPEN_DOCUMENT_LIMIT: usize = 8;

fn source_key(source: &PdfSource) -> (Uuid, usize) {
    (source.id, source.bytes.as_ptr() as usize)
}

impl State {
    fn ensure(&mut self, source: &PdfSource) -> Result<(), PdfError> {
        self.clock += 1;
        let key = source_key(source);
        if let Some(loaded) = self.loaded.get_mut(&key) {
            loaded.last_used = self.clock;
            return Ok(());
        }
        // SAFETY: the slice points into `bytes`, an immutable shared buffer kept alive in the
        // same `Loaded` entry and dropped only after the document that reads it.
        let slice: &'static [u8] = unsafe { std::slice::from_raw_parts(source.bytes.as_ptr(), source.bytes.len()) };
        let document = self.pdfium.load_pdf_from_byte_slice(slice, None)?;
        self.loaded.insert(key, Loaded { document, _bytes: source.bytes.clone(), last_used: self.clock });
        Ok(())
    }

    fn document(&self, source: &PdfSource) -> Result<&PdfDocument<'static>, PdfError> {
        self.loaded.get(&source_key(source)).map(|l| &l.document).ok_or(PdfError::Stopped)
    }

    fn trim(&mut self) {
        while self.loaded.len() > OPEN_DOCUMENT_LIMIT {
            let oldest = *self.loaded.iter().min_by_key(|(_, l)| l.last_used).map(|(k, _)| k).expect("non-empty");
            self.loaded.remove(&oldest);
        }
    }

    fn source<'w>(workspace: &'w PdfWorkspace, page: &PdfPageState) -> Result<Option<&'w PdfSource>, PdfError> {
        match page.source_id {
            None => Ok(None),
            Some(id) => workspace.source(id).map(Some).ok_or_else(|| PdfError::Invalid("Missing source document.".into())),
        }
    }

    fn source_page<'s>(&'s self, source: &PdfSource, page_number: u32) -> Result<PdfPage<'static>, PdfError> {
        let document = self.document(source)?;
        let count = document.pages().len() as u32;
        if page_number < 1 || page_number > count {
            return invalid("A workspace page refers to a source page that does not exist.");
        }
        Ok(document.pages().get((page_number - 1) as PdfPageIndex)?)
    }

    fn open(&mut self, bytes: Arc<[u8]>, name: &str) -> Result<PdfWorkspace, PdfError> {
        if bytes.len() > workspace_json::MAXIMUM_SOURCE_BYTES {
            return invalid("PDF files are limited to 64 MB in this version.");
        }
        if !workspace_json::is_pdf(&bytes) {
            return invalid("This file is not a PDF document.");
        }
        let file_name = Path::new(name).file_name().and_then(|n| n.to_str()).filter(|n| !n.trim().is_empty()).unwrap_or("Untitled.pdf").to_string();
        let source = PdfSource { id: Uuid::new_v4(), name: file_name.clone(), bytes };
        self.ensure(&source)?;
        let document = self.document(&source)?;
        let count = document.pages().len() as usize;
        if count == 0 || count > workspace_json::MAXIMUM_PAGES {
            return invalid("PDF documents must contain 1–4096 pages.");
        }
        let mut pages = Vec::with_capacity(count);
        for (i, page) in document.pages().iter().enumerate() {
            let frame = PageFrame::of(&page);
            pages.push(PdfPageState {
                source_id: Some(source.id),
                source_page: i as u32 + 1,
                width: frame.logical_width().max(1.0),
                height: frame.logical_height().max(1.0),
                ..Default::default()
            });
        }
        let author = document.metadata().get(PdfDocumentMetadataTagType::Author).map(|t| t.value().to_string()).unwrap_or_default();
        let workspace = PdfWorkspace { title: file_name, author, sources: vec![source], pages, ..Default::default() };
        workspace_json::validate(&workspace)?;
        Ok(workspace)
    }

    fn render(&mut self, source: &PdfSource, page: &PdfPageState, scale: f64) -> Result<Bitmap, PdfError> {
        self.ensure(source)?;
        let pdf_page = self.source_page(source, page.source_page)?;
        let rotation = page.rotation % 360;
        let (full_w, full_h) = if rotation % 180 == 0 { (page.width, page.height) } else { (page.height, page.width) };
        let uncropped = PdfPageState { crop: None, ..page.clone() };
        let visible = layout::display_bounds(&uncropped, page.visible_box());
        let x0 = (visible.x * scale).floor();
        let y0 = (visible.y * scale).floor();
        let width = ((visible.right() * scale).ceil() - x0).max(1.0);
        let height = ((visible.bottom() * scale).ceil() - y0).max(1.0);
        if (width as u64) * (height as u64) > MAX_BITMAP_PIXELS {
            return invalid("The requested page image is too large.");
        }
        let mut bitmap = PdfBitmap::empty(width as i32, height as i32, PdfBitmapFormat::BGRA)?;
        let config = PdfRenderConfig::new()
            .set_fixed_size((full_w * scale).round() as i32, (full_h * scale).round() as i32)
            .rotate(frame::rotation_from_degrees(rotation), false)
            .set_origin(-x0 as i32, -y0 as i32)
            .render_annotations(true)
            .render_form_data(true)
            .set_reverse_byte_order(false);
        pdf_page.render_into_bitmap_with_config(&mut bitmap, &config)?;
        let mut bgra = bitmap.as_raw_bytes();
        bgra.truncate(width as usize * height as usize * 4);
        Ok(Bitmap { width: width as u32, height: height as u32, bgra })
    }

    fn words(&mut self, source: &PdfSource, page_number: u32) -> Result<Vec<PdfWord>, PdfError> {
        self.ensure(source)?;
        let page = self.source_page(source, page_number)?;
        let frame = PageFrame::of(&page);
        let text = page.text()?;
        // Group characters in user space, where text normally runs left to right, then map
        // each word's box into logical space. Grouping after rotation would split words
        // on pages with an intrinsic /Rotate.
        let mut words = Vec::new();
        let mut current = String::new();
        let mut bounds: Option<(f32, f32, f32, f32)> = None; // left, bottom, right, top
        let mut flush = |current: &mut String, bounds: &mut Option<(f32, f32, f32, f32)>| {
            if let Some((l, b, r, t)) = bounds.take()
                && !current.is_empty()
            {
                let rect = PdfRect::new_from_values(b, l, t, r);
                words.push(PdfWord { text: std::mem::take(current), bounds: frame.rect_to_logical(rect) });
            }
            current.clear();
        };
        for ch in text.chars().iter() {
            let Some(c) = ch.unicode_char() else { continue };
            if c.is_whitespace() || c == '\u{0}' {
                flush(&mut current, &mut bounds);
                continue;
            }
            let Ok(rect) = ch.loose_bounds().or_else(|_| ch.tight_bounds()) else { continue };
            let (l, b, r, t) = (rect.left().value, rect.bottom().value, rect.right().value, rect.top().value);
            if let Some((bl, bb, br, bt)) = bounds {
                // A jump to another line or a wide gap ends the word even without a space.
                let height = (t - b).abs().max(1.0);
                let center = (b + t) / 2.0;
                let same_line = center >= bb.min(bt) && center <= bb.max(bt);
                let gap = l - br;
                if !same_line || gap > height * 0.6 || l < bl - height * 2.0 {
                    flush(&mut current, &mut bounds);
                }
            }
            current.push(c);
            bounds = Some(match bounds {
                Some((bl, bb, br, bt)) => (bl.min(l), bb.min(b), br.max(r), bt.max(t)),
                None => (l, b, r, t),
            });
        }
        flush(&mut current, &mut bounds);
        Ok(words)
    }

    fn workspace_words(&mut self, workspace: &PdfWorkspace, page: &PdfPageState) -> Result<Vec<PdfWord>, PdfError> {
        match Self::source(workspace, page)? {
            Some(source) => self.words(source, page.source_page),
            None => Ok(Vec::new()),
        }
    }

    fn find(&mut self, workspace: &PdfWorkspace, query: &str, match_case: bool) -> Result<Vec<SearchResult>, PdfError> {
        let query = query.trim();
        if query.is_empty() {
            return Ok(Vec::new());
        }
        let fold = |c: char| if match_case { c } else { c.to_lowercase().next().unwrap_or(c) };
        let needle: Vec<char> = query.chars().map(fold).collect();
        let mut results = Vec::new();
        for (page_index, page) in workspace.pages.iter().enumerate() {
            let words = self.workspace_words(workspace, page)?;
            // Joined page text with each word's starting char offset.
            let mut hay: Vec<char> = Vec::new();
            let mut starts = Vec::with_capacity(words.len());
            for word in &words {
                starts.push(hay.len());
                hay.extend(word.text.chars());
                hay.push(' ');
            }
            let folded: Vec<char> = hay.iter().copied().map(fold).collect();
            let mut start = 0;
            while start + needle.len() <= folded.len() {
                let Some(offset) = folded[start..].windows(needle.len()).position(|w| w == needle.as_slice()) else { break };
                let index = start + offset;
                let end = index + needle.len();
                let bounds: Vec<RectD> = words
                    .iter()
                    .zip(&starts)
                    .filter(|(w, s)| **s < end && **s + w.text.chars().count() > index)
                    .map(|(w, _)| w.bounds)
                    .collect();
                if !bounds.is_empty() {
                    let from = index.saturating_sub(28);
                    let to = (end + 60).min(hay.len());
                    results.push(SearchResult { page_index, excerpt: hay[from..to].iter().collect::<String>().trim().to_string(), bounds, annotation: None });
                }
                start = end.max(index + 1);
            }
            for annotation in &page.annotations {
                let texts = std::iter::once(annotation.text.as_str()).chain(annotation.replies.iter().map(|r| r.text.as_str()));
                for text in texts {
                    let folded: String = text.chars().map(fold).collect();
                    if folded.contains(&needle.iter().collect::<String>()) {
                        results.push(SearchResult { page_index, excerpt: text.to_string(), bounds: vec![annotation.bounds], annotation: Some(annotation.id) });
                        break;
                    }
                }
            }
        }
        Ok(results)
    }

    fn extract_text(&mut self, workspace: &PdfWorkspace) -> Result<String, PdfError> {
        let mut out = String::new();
        for (i, page) in workspace.pages.iter().enumerate() {
            if i > 0 {
                out.push_str("\n\n");
            }
            out.push_str(&format!("Page {}\n", i + 1));
            let words = self.workspace_words(workspace, page)?;
            out.push_str(&words.iter().map(|w| w.text.as_str()).collect::<Vec<_>>().join(" "));
            for annotation in page.annotations.iter().filter(|a| matches!(a.kind, refr_core::AnnotationKind::Text | refr_core::AnnotationKind::Note)) {
                out.push('\n');
                out.push_str(&annotation.text);
            }
        }
        Ok(out)
    }

    /// Builds a new PDF from workspace pages: original page content is copied, not
    /// rasterized; workspace rotation and crop become page properties; annotations become
    /// vector objects and comments become PDF text annotations.
    fn export(&mut self, workspace: &PdfWorkspace, selection: &[usize]) -> Result<Vec<u8>, PdfError> {
        if selection.is_empty() {
            return invalid("Select at least one page.");
        }
        for &i in selection {
            let page = workspace.pages.get(i).ok_or_else(|| PdfError::Invalid("That page does not exist.".into()))?;
            if let Some(source) = Self::source(workspace, page)? {
                self.ensure(source)?;
            }
        }
        let mut out = self.pdfium.create_new_pdf()?;
        let fonts = draw::Fonts::new(&mut out);
        for (destination, &i) in selection.iter().enumerate() {
            let state = &workspace.pages[i];
            let mut page = match Self::source(workspace, state)? {
                Some(source) => {
                    let document = self.document(source)?;
                    out.pages_mut().copy_page_from_document(document, (state.source_page - 1) as PdfPageIndex, destination as PdfPageIndex)?;
                    out.pages().get(destination as PdfPageIndex)?
                }
                None => out.pages_mut().create_page_at_end(PdfPagePaperSize::Custom(PdfPoints::new(state.width as f32), PdfPoints::new(state.height as f32)))?,
            };
            let frame = PageFrame::of(&page);
            if let Some(crop) = state.crop {
                page.boundaries_mut().set_crop(frame.rect_to_user(crop))?;
            }
            page.set_rotation(frame::rotation_from_degrees(frame.rotation + state.rotation));
            let matrix = frame.logical_to_user_matrix();
            {
                let mut painter = draw::Painter { document: &out, page: &mut page, matrix, fonts: &fonts };
                for annotation in &state.annotations {
                    painter.annotation(annotation)?;
                }
            }
            for note in state.annotations.iter().filter(|a| a.kind == refr_core::AnnotationKind::Note) {
                let mut contents = note.text.clone();
                for reply in &note.replies {
                    contents.push_str(&format!("\n\n{}: {}", reply.author, reply.text));
                }
                let mut annotation = page.annotations_mut().create_text_annotation(&contents)?;
                annotation.set_bounds(frame.rect_to_user(note.bounds))?;
                annotation.set_creator(&note.author)?;
            }
        }
        Ok(out.save_to_bytes()?)
    }

    fn export_png(&mut self, workspace: &PdfWorkspace, index: usize, scale: f64) -> Result<Vec<u8>, PdfError> {
        let page = workspace.pages.get(index).ok_or_else(|| PdfError::Invalid("That page does not exist.".into()))?;
        let scale = scale.clamp(0.1, 4.0).min(MAX_PNG_SIDE / page.display_width().max(page.display_height()));
        // Render the exported single page so annotations are painted exactly as in the PDF.
        let bytes: Arc<[u8]> = Arc::from(self.export(workspace, &[index])?);
        let single = PdfSource { id: Uuid::new_v4(), name: "page.pdf".into(), bytes };
        self.ensure(&single)?;
        let exported = {
            let document = self.document(&single)?;
            let pdf_page = document.pages().get(0)?;
            let frame = PageFrame::of(&pdf_page);
            PdfPageState { source_id: Some(single.id), width: frame.logical_width(), height: frame.logical_height(), ..Default::default() }
        };
        let bitmap = self.render(&single, &exported, scale);
        self.loaded.remove(&source_key(&single));
        bitmap?.to_png()
    }

    fn split(&mut self, workspace: &PdfWorkspace, stem: &str) -> Result<Vec<u8>, PdfError> {
        if workspace.pages.len() > MAX_SPLIT_PAGES {
            return invalid(format!("Splitting is limited to {MAX_SPLIT_PAGES} pages."));
        }
        let mut zip = zip::ZipWriter::new(std::io::Cursor::new(Vec::new()));
        let options = zip::write::SimpleFileOptions::default().compression_method(zip::CompressionMethod::Deflated);
        let mut total = 0usize;
        for i in 0..workspace.pages.len() {
            let pdf = self.export(workspace, &[i])?;
            total += pdf.len();
            if total > MAX_ARCHIVE_BYTES {
                return invalid("The split archive would exceed 256 MB.");
            }
            zip.start_file(format!("{stem}-page-{:03}.pdf", i + 1), options).map_err(|e| PdfError::Invalid(e.to_string()))?;
            zip.write_all(&pdf).map_err(|e| PdfError::Invalid(e.to_string()))?;
        }
        Ok(zip.finish().map_err(|e| PdfError::Invalid(e.to_string()))?.into_inner())
    }
}

impl Engine {
    /// Starts the engine thread and binds PDFium from `library` (a path to libpdfium).
    /// PDFium binds once per process; later calls return an error.
    pub fn start(library: impl AsRef<Path>) -> Result<Engine, PdfError> {
        let path = library.as_ref().to_path_buf();
        let (jobs, receiver) = mpsc::channel::<Job>();
        let (ready_tx, ready_rx) = mpsc::channel();
        std::thread::Builder::new()
            .name("pdfium".into())
            .spawn(move || {
                let pdfium = match Pdfium::bind_to_library(&path) {
                    Ok(bindings) => Box::leak(Box::new(Pdfium::new(bindings))),
                    Err(error) => {
                        let _ = ready_tx.send(Err(PdfError::Library { path, message: format!("{error:?}") }));
                        return;
                    }
                };
                let _ = ready_tx.send(Ok(()));
                let mut state = State { pdfium, loaded: HashMap::new(), clock: 0 };
                while let Ok(job) = receiver.recv() {
                    job(&mut state);
                    state.trim();
                }
            })
            .map_err(|e| PdfError::Invalid(e.to_string()))?;
        ready_rx.recv().map_err(|_| PdfError::Stopped)??;
        Ok(Engine { jobs })
    }

    /// Finds libpdfium: `REFR_PDFIUM`, next to the executable (or in an app bundle's
    /// Frameworks folder), then the development copy in `vendor/pdfium/lib`.
    pub fn locate_library() -> Option<PathBuf> {
        let name = Pdfium::pdfium_platform_library_name();
        let mut candidates = Vec::new();
        if let Some(path) = std::env::var_os("REFR_PDFIUM") {
            candidates.push(PathBuf::from(path));
        }
        if let Ok(exe) = std::env::current_exe()
            && let Some(dir) = exe.parent()
        {
            candidates.push(dir.join(&name));
            candidates.push(dir.join("../Frameworks").join(&name));
            candidates.push(dir.join("lib").join(&name));
        }
        candidates.push(Path::new(env!("CARGO_MANIFEST_DIR")).join("../../vendor/pdfium/lib").join(&name));
        candidates.into_iter().find(|p| p.is_file())
    }

    fn call<R: Send + 'static>(&self, job: impl FnOnce(&mut State) -> Result<R, PdfError> + Send + 'static) -> Result<R, PdfError> {
        let (tx, rx) = mpsc::sync_channel(1);
        self.jobs
            .send(Box::new(move |state| {
                let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| job(state)))
                    .unwrap_or_else(|_| Err(PdfError::Invalid("The PDF engine failed on this document.".into())));
                let _ = tx.send(result);
            }))
            .map_err(|_| PdfError::Stopped)?;
        rx.recv().map_err(|_| PdfError::Stopped)?
    }

    pub fn open(&self, bytes: impl Into<Arc<[u8]>>, name: &str) -> Result<PdfWorkspace, PdfError> {
        let (bytes, name) = (bytes.into(), name.to_string());
        self.call(move |s| s.open(bytes, &name))
    }

    /// Renders a page's visible (cropped, rotated) area at `scale` pixels per point.
    /// Returns `None` for blank pages, which have no source content.
    pub fn render(&self, workspace: &Arc<PdfWorkspace>, page_index: usize, scale: f64) -> Result<Option<Bitmap>, PdfError> {
        let workspace = workspace.clone();
        self.call(move |s| {
            let page = workspace.pages.get(page_index).ok_or_else(|| PdfError::Invalid("That page does not exist.".into()))?;
            match State::source(&workspace, page)? {
                Some(source) => s.render(source, page, scale).map(Some),
                None => Ok(None),
            }
        })
    }

    pub fn words(&self, workspace: &Arc<PdfWorkspace>, page_index: usize) -> Result<Vec<PdfWord>, PdfError> {
        let workspace = workspace.clone();
        self.call(move |s| {
            let page = workspace.pages.get(page_index).ok_or_else(|| PdfError::Invalid("That page does not exist.".into()))?;
            s.workspace_words(&workspace, page)
        })
    }

    pub fn find(&self, workspace: &Arc<PdfWorkspace>, query: &str, match_case: bool) -> Result<Vec<SearchResult>, PdfError> {
        let (workspace, query) = (workspace.clone(), query.to_string());
        self.call(move |s| s.find(&workspace, &query, match_case))
    }

    pub fn extract_text(&self, workspace: &Arc<PdfWorkspace>) -> Result<String, PdfError> {
        let workspace = workspace.clone();
        self.call(move |s| s.extract_text(&workspace))
    }

    /// Exports the selected pages (all when `None`) as a new PDF.
    pub fn export_pdf(&self, workspace: &Arc<PdfWorkspace>, pages: Option<Vec<usize>>) -> Result<Vec<u8>, PdfError> {
        let workspace = workspace.clone();
        self.call(move |s| {
            let pages = pages.unwrap_or_else(|| (0..workspace.pages.len()).collect());
            s.export(&workspace, &pages)
        })
    }

    pub fn export_png(&self, workspace: &Arc<PdfWorkspace>, page_index: usize, scale: f64) -> Result<Vec<u8>, PdfError> {
        let workspace = workspace.clone();
        self.call(move |s| s.export_png(&workspace, page_index, scale))
    }

    /// A ZIP archive with one PDF per page.
    pub fn split(&self, workspace: &Arc<PdfWorkspace>) -> Result<Vec<u8>, PdfError> {
        let workspace = workspace.clone();
        self.call(move |s| {
            let stem = Path::new(&workspace.title).file_stem().and_then(|s| s.to_str()).unwrap_or("document").to_string();
            s.split(&workspace, &stem)
        })
    }

    /// The bundled six-page demonstration report, generated on the fly.
    pub fn sample(&self) -> Result<PdfWorkspace, PdfError> {
        self.call(sample::create)
    }
}

#[cfg(test)]
mod tests;
