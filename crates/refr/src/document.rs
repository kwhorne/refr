//! One open document: its editor session, view state and page image caches.
//! Painting and pointer input live in `viewport.rs`.

use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::sync::Arc;

use gpui::{App, AppContext, Bounds, Context, Entity, EventEmitter, FocusHandle, Focusable, Pixels, RenderImage, Window, px};
use refr_core::layout::{self, PageLayoutMode, PagePlacement};
use refr_core::{Annotation, AnnotationKind, EditError, EditorSession, PdfTool, PdfWorkspace, PointD, RectD, Uuid};
use refr_pdf::{Bitmap, Engine, PdfWord, SearchResult};
use smallvec::SmallVec;

use crate::text_input::{InputEvent, TextInput};

const VIEW_CACHE: usize = 36;
const THUMB_CACHE: usize = 400;
/// Longest side of a rendered page image, in pixels. Beyond this the image is scaled up.
const MAX_IMAGE_SIDE: f64 = 5000.0;
pub const THUMB_WIDTH: f64 = 150.0;

pub enum DocumentEvent {
    /// The document changed (edit, undo or redo).
    Changed,
    Status(String, bool),
    NoteRequested { page: usize, point: PointD },
    EditNote(Uuid),
}

/// What decides a page's rendered pixels. Annotations are painted separately by GPUI.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct PageKey {
    source: Uuid,
    source_page: u32,
    rotation: u32,
    crop: [i64; 4],
}

impl PageKey {
    pub fn of(doc: &PdfWorkspace, index: usize) -> Option<PageKey> {
        let page = doc.pages.get(index)?;
        let crop = page.crop.map(|c| [c.x, c.y, c.width, c.height].map(|v| (v * 100.0).round() as i64)).unwrap_or_default();
        Some(PageKey { source: page.source_id?, source_page: page.source_page, rotation: page.rotation, crop })
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
struct ImageKey {
    page: PageKey,
    /// Pixels per point × 1000.
    scale: u32,
}

struct CachedImage {
    image: Option<Arc<RenderImage>>,
    failed: bool,
    last_used: u64,
}

pub enum Words {
    Pending,
    Ready(Arc<Vec<PdfWord>>),
}

#[derive(Clone, Debug)]
pub struct TextSelection {
    pub page: usize,
    pub words: Vec<PdfWord>,
}

impl TextSelection {
    pub fn text(&self) -> String {
        self.words.iter().map(|w| w.text.as_str()).collect::<Vec<_>>().join(" ")
    }
}

pub struct InlineText {
    pub page: usize,
    /// Top-left of the text box in page coordinates.
    pub origin: PointD,
    pub width: f64,
    /// The annotation being edited, or `None` for a new one.
    pub editing: Option<Uuid>,
    pub input: Entity<TextInput>,
}

#[derive(Clone)]
pub enum Gesture {
    Pan { last: gpui::Point<Pixels> },
    ScrollDrag { grab: f64 },
    Move { page: usize, original: Annotation, start: PointD, current: PointD },
    Resize { page: usize, original: Annotation, corner: usize, display: RectD, current: RectD },
    Draw { page: usize, tool: PdfTool, points: Vec<PointD> },
    Shape { page: usize, tool: PdfTool, start: PointD, end: PointD },
    TextSelect { page: usize, tool: PdfTool, start: PointD, end: PointD },
}

pub struct DocumentView {
    pub session: EditorSession,
    pub engine: Engine,
    pub focus_handle: FocusHandle,
    /// Where the editable workspace was last saved.
    pub path: Option<PathBuf>,
    pub zoom: f64,
    pub(crate) scroll: f64,
    pub(crate) pan: f64,
    pub layout_mode: PageLayoutMode,
    /// Canvas bounds in window coordinates, from the last paint.
    pub(crate) viewport: Bounds<Pixels>,
    pub(crate) scale_factor: f32,
    /// Page placements relative to the canvas origin, from the last paint.
    pub(crate) placements: Vec<PagePlacement>,
    pub(crate) tracked_scroll: f64,
    pub(crate) pending_fit: Option<Fit>,
    images: HashMap<ImageKey, CachedImage>,
    thumbs: HashMap<ImageKey, CachedImage>,
    pending_pages: HashSet<(PageKey, bool)>,
    words: HashMap<(Uuid, u32), Words>,
    clock: u64,
    pub(crate) gesture: Option<Gesture>,
    pub text_selection: Option<TextSelection>,
    pub search: Vec<SearchResult>,
    pub active_search: Option<usize>,
    pub inline_text: Option<InlineText>,
    last_revision: u64,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Fit {
    Page,
    Width,
}

impl EventEmitter<DocumentEvent> for DocumentView {}

impl Focusable for DocumentView {
    fn focus_handle(&self, _: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl DocumentView {
    pub fn new(session: EditorSession, engine: Engine, cx: &mut Context<Self>) -> Self {
        Self {
            session,
            engine,
            focus_handle: cx.focus_handle(),
            path: None,
            zoom: 1.0,
            scroll: 0.0,
            pan: 0.0,
            layout_mode: PageLayoutMode::Continuous,
            viewport: Bounds::default(),
            scale_factor: 2.0,
            placements: Vec::new(),
            tracked_scroll: 0.0,
            pending_fit: Some(Fit::Page),
            images: HashMap::new(),
            thumbs: HashMap::new(),
            pending_pages: HashSet::new(),
            words: HashMap::new(),
            clock: 0,
            gesture: None,
            text_selection: None,
            search: Vec::new(),
            active_search: None,
            inline_text: None,
            last_revision: 0,
        }
    }

    pub fn doc(&self) -> &Arc<PdfWorkspace> {
        self.session.document()
    }

    pub fn title(&self) -> &str {
        &self.doc().title
    }

    pub fn status(&self, text: impl Into<String>, cx: &mut Context<Self>) {
        cx.emit(DocumentEvent::Status(text.into(), false));
    }

    pub fn error(&self, text: impl Into<String>, cx: &mut Context<Self>) {
        cx.emit(DocumentEvent::Status(text.into(), true));
    }

    /// Runs a document command, reporting failures in the status bar.
    pub fn edit(&mut self, cx: &mut Context<Self>, command: impl FnOnce(&mut EditorSession) -> Result<(), EditError>) {
        match command(&mut self.session) {
            Ok(()) => self.after_change(cx),
            Err(error) => self.error(error.to_string(), cx),
        }
    }

    pub(crate) fn after_change(&mut self, cx: &mut Context<Self>) {
        if self.session.revision() != self.last_revision {
            self.last_revision = self.session.revision();
            self.search.clear();
            self.active_search = None;
            if self.text_selection.as_ref().is_some_and(|s| s.page >= self.doc().pages.len()) {
                self.text_selection = None;
            }
            cx.emit(DocumentEvent::Changed);
        }
        cx.notify();
    }

    pub fn undo(&mut self, cx: &mut Context<Self>) {
        self.finish_text(true, cx);
        match self.session.undo() {
            Some(label) => self.status(format!("Undid: {label}"), cx),
            None => self.status("Nothing to undo.", cx),
        }
        self.after_change(cx);
    }

    pub fn redo(&mut self, cx: &mut Context<Self>) {
        self.finish_text(true, cx);
        match self.session.redo() {
            Some(label) => self.status(format!("Redid: {label}"), cx),
            None => self.status("Nothing to redo.", cx),
        }
        self.after_change(cx);
    }

    pub fn set_tool(&mut self, tool: PdfTool, cx: &mut Context<Self>) {
        self.finish_text(true, cx);
        self.gesture = None;
        self.session.set_tool(tool);
        if !matches!(tool, PdfTool::Select) {
            self.text_selection = None;
        }
        let hint = match tool {
            PdfTool::Hand => "Drag to pan. ⌘-scroll zooms around the pointer.",
            PdfTool::Select => "Select annotations or drag across text. Double-click added text to edit it.",
            PdfTool::Text => "Click on a page to add text. Press Return or click outside to apply.",
            PdfTool::Note => "Click on a page to place a comment.",
            PdfTool::Signature => "Draw your signature. This creates a visual mark, not a certificate-based digital signature.",
            PdfTool::Highlight | PdfTool::Underline | PdfTool::Strikeout => "Drag across text to mark it up.",
            PdfTool::Crop => "Drag a rectangle to crop the page. Cropping hides content; it does not redact it.",
            PdfTool::Stamp => "Click to place an approval stamp.",
            PdfTool::Check => "Click to place a check mark.",
            _ => "Drag on a page to draw.",
        };
        self.status(hint, cx);
        cx.notify();
    }

    // ---- Navigation and zoom -------------------------------------------------------------

    pub(crate) fn viewport_size(&self) -> (f64, f64) {
        (f64::from(self.viewport.size.width), f64::from(self.viewport.size.height))
    }

    fn arrange_at(&self, zoom: f64, scroll: f64, pan: f64) -> Vec<PagePlacement> {
        let (width, _) = self.viewport_size();
        layout::arrange(&self.doc().pages, width, zoom, scroll, pan, self.layout_mode, self.session.current_page())
    }

    pub(crate) fn content_size(&self) -> (f64, f64) {
        let (width, _) = self.viewport_size();
        layout::content_size(&self.doc().pages, width, self.zoom, self.layout_mode, self.session.current_page())
    }

    pub(crate) fn clamp_scroll(&mut self) {
        let (width, height) = self.viewport_size();
        let (content_w, content_h) = self.content_size();
        self.scroll = self.scroll.clamp(0.0, (content_h - height).max(0.0));
        let overflow = (content_w - width).max(0.0);
        self.pan = self.pan.clamp(-overflow, 0.0);
    }

    pub fn go_to_page(&mut self, index: usize, cx: &mut Context<Self>) {
        let index = index.min(self.doc().pages.len() - 1);
        self.finish_text(true, cx);
        self.session.navigate(index);
        if self.layout_mode == PageLayoutMode::SinglePage {
            self.scroll = 0.0;
        } else if let Some(p) = self.arrange_at(self.zoom, 0.0, self.pan).into_iter().find(|p| p.index == index) {
            self.scroll = p.bounds.y - layout::GAP;
        }
        self.clamp_scroll();
        self.tracked_scroll = self.scroll;
        cx.notify();
    }

    /// Scrolls so a page-space rectangle is visible, keeping the zoom.
    pub fn reveal_rect(&mut self, index: usize, rect: RectD, cx: &mut Context<Self>) {
        self.go_to_page(index, cx);
        let page = &self.doc().pages[index];
        if let Some(p) = self.arrange_at(self.zoom, 0.0, self.pan).into_iter().find(|p| p.index == index) {
            let screen = p.rect_to_screen(page, rect, self.zoom);
            let (_, height) = self.viewport_size();
            if self.layout_mode != PageLayoutMode::SinglePage {
                self.scroll = screen.y - height / 3.0;
            }
            self.clamp_scroll();
            self.tracked_scroll = self.scroll;
        }
        cx.notify();
    }

    pub fn fit(&mut self, fit: Fit, cx: &mut Context<Self>) {
        let (width, height) = self.viewport_size();
        if width < 50.0 || height < 50.0 {
            self.pending_fit = Some(fit);
            cx.notify();
            return;
        }
        let page = self.session.page();
        let (dw, dh) = match self.layout_mode {
            PageLayoutMode::TwoPage => {
                let next = self.doc().pages.get(self.session.current_page() + 1).map(|p| p.display_width()).unwrap_or(0.0);
                (page.display_width() + next + layout::GAP, page.display_height())
            }
            _ => (page.display_width(), page.display_height()),
        };
        let fit_width = (width - layout::GAP * 2.0 - 12.0) / dw;
        let zoom = match fit {
            Fit::Width => fit_width,
            Fit::Page => fit_width.min((height - layout::GAP * 2.0) / dh),
        };
        let index = self.session.current_page();
        self.zoom = layout::clamp_zoom(zoom);
        self.pan = 0.0;
        self.go_to_page(index, cx);
    }

    /// Zooms keeping the page point under `anchor` (canvas-local pixels) fixed.
    pub fn zoom_to(&mut self, zoom: f64, anchor: Option<PointD>, cx: &mut Context<Self>) {
        let zoom = layout::clamp_zoom(zoom);
        let (width, height) = self.viewport_size();
        let anchor = anchor.unwrap_or(PointD::new(width / 2.0, height / 2.0));
        let before = self.arrange_at(self.zoom, self.scroll, self.pan);
        let hit = before
            .iter()
            .find(|p| p.bounds.y <= anchor.y && p.bounds.bottom() + layout::GAP >= anchor.y)
            .or_else(|| before.iter().find(|p| p.index == self.session.current_page()))
            .copied();
        let old = self.zoom;
        self.zoom = zoom;
        if let Some(p) = hit {
            let fy = (anchor.y - p.bounds.y) / (p.bounds.height.max(1.0));
            let fx = (anchor.x - p.bounds.x) / (p.bounds.width.max(1.0));
            let after = self.arrange_at(zoom, 0.0, 0.0);
            if let Some(q) = after.iter().find(|q| q.index == p.index) {
                self.scroll = q.bounds.y + fy * q.bounds.height - anchor.y;
                let (content_w, _) = self.content_size();
                self.pan = if content_w > width { anchor.x - (q.bounds.x + fx * q.bounds.width) } else { 0.0 };
            }
        } else {
            self.scroll *= zoom / old;
        }
        self.clamp_scroll();
        self.tracked_scroll = self.scroll;
        cx.notify();
    }

    pub fn set_layout_mode(&mut self, mode: PageLayoutMode, cx: &mut Context<Self>) {
        self.layout_mode = mode;
        let index = self.session.current_page();
        self.go_to_page(index, cx);
    }

    pub fn scroll_by(&mut self, dx: f64, dy: f64, cx: &mut Context<Self>) {
        self.scroll -= dy;
        self.pan += dx;
        self.clamp_scroll();
        cx.notify();
    }

    // ---- Page images ------------------------------------------------------------------

    fn tick(&mut self) -> u64 {
        self.clock += 1;
        self.clock
    }

    fn quantize(scale: f64) -> u32 {
        // Eighth steps: fine enough to stay sharp, coarse enough that small zoom
        // changes reuse the image.
        ((scale * 8.0).ceil() / 8.0 * 1000.0) as u32
    }

    /// The best available image for a page at `scale`, requesting a sharper one if needed.
    /// Returns `(image, is_exact)`.
    pub(crate) fn page_image(&mut self, index: usize, scale: f64, thumbnail: bool, cx: &mut Context<Self>) -> (Option<Arc<RenderImage>>, bool, bool) {
        let doc = self.doc().clone();
        let Some(page_key) = PageKey::of(&doc, index) else { return (None, true, false) };
        let page = &doc.pages[index];
        let longest = page.display_width().max(page.display_height());
        let scale = scale.min(MAX_IMAGE_SIDE / longest.max(1.0));
        let key = ImageKey { page: page_key, scale: Self::quantize(scale) };
        let now = self.tick();
        let cache = if thumbnail { &mut self.thumbs } else { &mut self.images };
        if let Some(entry) = cache.get_mut(&key) {
            entry.last_used = now;
            if entry.image.is_some() || entry.failed {
                return (entry.image.clone(), true, entry.failed);
            }
        }
        // Fall back to the closest-scale image of the same page while a sharper one renders.
        let fallback = cache
            .iter()
            .filter(|(k, v)| k.page == page_key && v.image.is_some())
            .min_by_key(|(k, _)| (k.scale as i64 - key.scale as i64).abs())
            .and_then(|(_, v)| v.image.clone())
            .or_else(|| {
                let other = if thumbnail { &self.images } else { &self.thumbs };
                other.iter().filter(|(k, v)| k.page == page_key && v.image.is_some()).max_by_key(|(k, _)| k.scale).and_then(|(_, v)| v.image.clone())
            });
        if !self.pending_pages.contains(&(page_key, thumbnail)) && !cache_has(if thumbnail { &self.thumbs } else { &self.images }, &key) {
            self.request_image(key, index, scale, thumbnail, cx);
        }
        (fallback, false, false)
    }

    fn request_image(&mut self, key: ImageKey, index: usize, scale: f64, thumbnail: bool, cx: &mut Context<Self>) {
        self.pending_pages.insert((key.page, thumbnail));
        let now = self.tick();
        let cache = if thumbnail { &mut self.thumbs } else { &mut self.images };
        cache.insert(key, CachedImage { image: None, failed: false, last_used: now });
        let engine = self.engine.clone();
        let doc = self.doc().clone();
        let task = cx.background_executor().spawn(async move { engine.render(&doc, index, scale) });
        cx.spawn(async move |this, cx| {
            let result = task.await;
            this.update(cx, |this, cx| {
                this.pending_pages.remove(&(key.page, thumbnail));
                let image = match result {
                    Ok(Some(bitmap)) => Some(to_render_image(bitmap)),
                    Ok(None) => None,
                    Err(error) => {
                        if !thumbnail {
                            this.error(format!("Page {} could not be rendered: {error}", index + 1), cx);
                        }
                        None
                    }
                };
                let cache = if thumbnail { &mut this.thumbs } else { &mut this.images };
                if let Some(entry) = cache.get_mut(&key) {
                    entry.failed = image.is_none();
                    entry.image = image;
                }
                this.evict(thumbnail, cx);
                cx.notify();
            })
            .ok();
        })
        .detach();
    }

    fn evict(&mut self, thumbnail: bool, cx: &mut Context<Self>) {
        let (cache, limit) = if thumbnail { (&mut self.thumbs, THUMB_CACHE) } else { (&mut self.images, VIEW_CACHE) };
        while cache.len() > limit {
            let Some(oldest) = cache.iter().filter(|(_, v)| v.image.is_some() || v.failed).min_by_key(|(_, v)| v.last_used).map(|(k, _)| *k) else { break };
            if let Some(entry) = cache.remove(&oldest)
                && let Some(image) = entry.image
            {
                cx.drop_image(image, None);
            }
        }
    }

    pub fn thumbnail(&mut self, index: usize, cx: &mut Context<Self>) -> (Option<Arc<RenderImage>>, bool) {
        let page = &self.doc().pages[index];
        let scale = THUMB_WIDTH / page.display_width().max(page.display_height()).max(1.0) * self.scale_factor as f64 * 1.4;
        let (image, _, failed) = self.page_image(index, scale, true, cx);
        (image, failed)
    }

    pub fn release_images(&mut self, cx: &mut App) {
        for entry in self.images.drain().chain(self.thumbs.drain()).map(|(_, v)| v) {
            if let Some(image) = entry.image {
                cx.drop_image(image, None);
            }
        }
    }

    // ---- Text -------------------------------------------------------------------------

    pub(crate) fn cached_words(&self, key: (Uuid, u32)) -> Option<Arc<Vec<PdfWord>>> {
        match self.words.get(&key) {
            Some(Words::Ready(words)) => Some(words.clone()),
            _ => None,
        }
    }

    pub fn words(&mut self, index: usize, cx: &mut Context<Self>) -> Option<Arc<Vec<PdfWord>>> {
        let page = self.doc().pages.get(index)?;
        let key = (page.source_id?, page.source_page);
        match self.words.get(&key) {
            Some(Words::Ready(words)) => return Some(words.clone()),
            Some(Words::Pending) => return None,
            None => {}
        }
        self.words.insert(key, Words::Pending);
        let engine = self.engine.clone();
        let doc = self.doc().clone();
        let task = cx.background_executor().spawn(async move { engine.words(&doc, index) });
        cx.spawn(async move |this, cx| {
            let words = task.await.unwrap_or_default();
            this.update(cx, |this, cx| {
                this.words.insert(key, Words::Ready(Arc::new(words)));
                cx.notify();
            })
            .ok();
        })
        .detach();
        None
    }

    pub fn copy_selection(&mut self, cx: &mut Context<Self>) {
        if let Some(selection) = &self.text_selection {
            cx.write_to_clipboard(gpui::ClipboardItem::new_string(selection.text()));
            let count = selection.words.len();
            self.status(format!("Copied {count} word{}.", if count == 1 { "" } else { "s" }), cx);
        } else if let Some(annotation) = self.session.selected_annotation().filter(|a| !a.text.is_empty()) {
            cx.write_to_clipboard(gpui::ClipboardItem::new_string(annotation.text.clone()));
            self.status("Copied annotation text.", cx);
        }
    }

    /// Marks up the selected text with highlight, underline or strikeout, one mark per line.
    pub fn markup_selection(&mut self, kind: AnnotationKind, cx: &mut Context<Self>) {
        let Some(selection) = self.text_selection.take() else { return };
        let color = self.session.color;
        let stroke = self.session.stroke_width.max(1.0);
        let lines = line_boxes(&selection.words);
        let page_id = self.doc().pages[selection.page].id;
        let marks: Vec<Annotation> = lines
            .into_iter()
            .map(|bounds| Annotation { color, stroke_width: stroke, text: String::new(), ..Annotation::new(kind, bounds) })
            .collect();
        let count = marks.len();
        let label = format!("Add {}", kind.label());
        self.edit(cx, |s| {
            s.execute(label, |doc| {
                doc.update_page(page_id, |p| {
                    let mut p = p.clone();
                    p.annotations.extend(marks);
                    p
                })
            })
        });
        if count > 0 {
            self.status(format!("Added {} to the selected text.", kind.label()), cx);
        }
    }

    // ---- Inline text ---------------------------------------------------------------------

    pub fn start_text(&mut self, page: usize, origin: PointD, editing: Option<Uuid>, window: &mut Window, cx: &mut Context<Self>) {
        self.finish_text(true, cx);
        let existing = editing.and_then(|id| self.doc().pages[page].annotation(id).cloned());
        let width = existing.as_ref().map(|a| a.bounds.width).unwrap_or(220.0);
        let input = cx.new(|cx| {
            let mut input = TextInput::new("Type text", cx);
            if let Some(a) = &existing {
                input.set_text(a.text.clone(), cx);
                input.select_all_text(cx);
            }
            input
        });
        input.update(cx, |input, cx| input.watch_blur(window, cx));
        cx.subscribe(&input, |this, _, event: &InputEvent, cx| match event {
            InputEvent::Confirm(_) | InputEvent::Blur => this.finish_text(true, cx),
            InputEvent::Cancel => this.finish_text(false, cx),
            InputEvent::Changed => cx.notify(),
        })
        .detach();
        input.update(cx, |input, _| input.focus(window));
        self.inline_text = Some(InlineText { page, origin, width, editing, input });
        cx.notify();
    }

    pub fn finish_text(&mut self, apply: bool, cx: &mut Context<Self>) {
        let Some(inline) = self.inline_text.take() else { return };
        let text = inline.input.read(cx).text().trim().to_string();
        cx.notify();
        if !apply {
            return;
        }
        match inline.editing {
            Some(id) if text.is_empty() => self.edit(cx, |s| s.delete_annotation(id)),
            Some(id) => {
                self.edit(cx, |s| {
                    s.update_annotation(id, "Edit text", |a| {
                        let lines = refr_core::text_layout::wrap(&text, a.font_size, a.bounds.width).len();
                        let height = refr_core::text_layout::height(lines, a.font_size);
                        Annotation { text: text.clone(), bounds: RectD { height, ..a.bounds }, ..a.clone() }
                    })
                });
            }
            None if text.is_empty() => {}
            None => {
                let size = self.session.font_size;
                let measured = refr_core::text_layout::measure(&text, size) + 4.0;
                let page = &self.doc().pages[inline.page];
                let width = measured.min(inline.width.max(40.0)).min(page.width - inline.origin.x).max(20.0);
                let lines = refr_core::text_layout::wrap(&text, size, width).len();
                let annotation = Annotation {
                    kind: AnnotationKind::Text,
                    bounds: RectD::new(inline.origin.x, inline.origin.y, width, refr_core::text_layout::height(lines, size)),
                    color: self.session.color,
                    font_size: size,
                    text,
                    ..Default::default()
                };
                self.edit(cx, |s| s.add_annotation(annotation, Some(inline.page)));
            }
        }
    }
}

fn cache_has(cache: &HashMap<ImageKey, CachedImage>, key: &ImageKey) -> bool {
    cache.contains_key(key)
}

pub fn to_render_image(bitmap: Bitmap) -> Arc<RenderImage> {
    let buffer = image::RgbaImage::from_raw(bitmap.width, bitmap.height, bitmap.bgra).expect("bitmap size matches its buffer");
    let frames: SmallVec<[image::Frame; 1]> = SmallVec::from_elem(image::Frame::new(buffer), 1);
    Arc::new(RenderImage::new(frames))
}

/// Groups words into per-line boxes, in reading order.
pub fn line_boxes(words: &[PdfWord]) -> Vec<RectD> {
    let mut lines: Vec<RectD> = Vec::new();
    for word in words {
        let center = word.bounds.y + word.bounds.height / 2.0;
        match lines.last_mut() {
            Some(line) if center >= line.y && center <= line.bottom() && word.bounds.x >= line.x - 2.0 => *line = RectD::union(*line, word.bounds),
            _ => lines.push(word.bounds),
        }
    }
    lines
}

pub fn px_f(v: f64) -> Pixels {
    px(v as f32)
}
