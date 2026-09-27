//! The document viewport: page painting, annotation overlays and pointer tools.
//!
//! Each frame, `prepare` (run in the canvas prepaint, with the real canvas bounds) lays out
//! pages, picks page images and turns annotations into screen-space marks; `Frame::paint`
//! then draws them. Pointer handlers reuse the placements stored by the last `prepare`.

use std::sync::Arc;

use gpui::{
    App, Bounds, ContentMask, Context, CursorStyle, FontWeight, Hsla, InteractiveElement, IntoElement, MouseButton, MouseDownEvent,
    MouseMoveEvent, MouseUpEvent, ParentElement, PathBuilder, Pixels, Point, Render, RenderImage, ScrollWheelEvent, SharedString,
    Styled, TextRun, Window, canvas, div, fill, font, point, px, rgb, rgba, size,
};
use refr_core::layout::{self, PagePlacement};
use refr_core::text_layout;
use refr_core::{Annotation, AnnotationKind, PdfPageState, PdfTool, PointD, RectD, Uuid};

use crate::actions::*;
use crate::glyphs;
use crate::document::{DocumentEvent, DocumentView, Fit, Gesture, TextSelection, px_f};
use crate::theme::{self, argb};

const HANDLE: f64 = 8.0;
const SCROLLBAR: f64 = 8.0;

pub(crate) enum Mark {
    Fill { rect: RectD, color: Hsla, radius: f32 },
    Polygon { points: Vec<PointD>, color: Hsla },
    Stroke { points: Vec<PointD>, width: f64, color: Hsla, closed: bool },
    Text { origin: PointD, text: SharedString, size: f64, color: Hsla, bold: bool },
    Glyphs { cmds: Vec<glyphs::Cmd>, color: Hsla },
    Outline { rect: RectD, color: Hsla },
    Handle { rect: RectD },
}

pub(crate) struct PageDraw {
    rect: RectD,
    image: Option<Arc<RenderImage>>,
    loading: bool,
    failed: bool,
}

pub(crate) struct Frame {
    origin: Point<Pixels>,
    bounds: Bounds<Pixels>,
    pages: Vec<PageDraw>,
    marks: Vec<Mark>,
    scrollbar: Option<RectD>,
}

fn pt(origin: Point<Pixels>, p: PointD) -> Point<Pixels> {
    point(origin.x + px_f(p.x), origin.y + px_f(p.y))
}

fn rect_bounds(origin: Point<Pixels>, r: RectD) -> Bounds<Pixels> {
    Bounds::new(pt(origin, PointD::new(r.x, r.y)), size(px_f(r.width), px_f(r.height)))
}

impl Frame {
    fn paint(self, window: &mut Window, cx: &mut App) {
        let origin = self.origin;
        window.paint_quad(fill(self.bounds, rgb(theme::CANVAS)));
        window.with_content_mask(Some(ContentMask { bounds: self.bounds }), |window| {
            for page in &self.pages {
                let shadow = RectD::new(page.rect.x - 1.0, page.rect.y, page.rect.width + 2.0, page.rect.height + 2.0);
                window.paint_quad(fill(rect_bounds(origin, shadow), rgba(0x00000022)));
                window.paint_quad(fill(rect_bounds(origin, page.rect), rgb(0xFFFFFF)));
                if let Some(image) = &page.image {
                    window.paint_image(rect_bounds(origin, page.rect), 0.0.into(), image.clone(), 0, false).ok();
                } else if page.loading || page.failed {
                    let label = if page.failed { "This page could not be rendered." } else { "Loading…" };
                    paint_text(window, cx, pt(origin, PointD::new(page.rect.x + 16.0, page.rect.y + 16.0)), label, 12.0, rgb(0x999999).into(), false, theme::FONT);
                }
            }
            for mark in self.marks {
                match mark {
                    Mark::Fill { rect, color, radius } => {
                        window.paint_quad(gpui::quad(rect_bounds(origin, rect), px(radius), color, 0.0, gpui::transparent_black(), Default::default()));
                    }
                    Mark::Polygon { points, color } => {
                        let mut builder = PathBuilder::fill();
                        builder.add_polygon(&points.iter().map(|p| pt(origin, *p)).collect::<Vec<_>>(), true);
                        if let Ok(path) = builder.build() {
                            window.paint_path(path, color);
                        }
                    }
                    Mark::Stroke { points, width, color, closed } => {
                        if points.len() < 2 {
                            continue;
                        }
                        let mut builder = PathBuilder::stroke(px_f(width.max(0.5)));
                        builder.add_polygon(&points.iter().map(|p| pt(origin, *p)).collect::<Vec<_>>(), closed);
                        if let Ok(path) = builder.build() {
                            window.paint_path(path, color);
                        }
                    }
                    Mark::Text { origin: at, text, size, color, bold } => {
                        paint_text(window, cx, pt(origin, at), &text, size, color, bold, theme::ANNOTATION_FONT);
                    }
                    Mark::Glyphs { cmds, color } => {
                        if cmds.is_empty() {
                            continue;
                        }
                        let mut builder = PathBuilder::fill();
                        for cmd in cmds {
                            match cmd {
                                glyphs::Cmd::Move(p) => builder.move_to(pt(origin, p)),
                                glyphs::Cmd::Line(p) => builder.line_to(pt(origin, p)),
                                glyphs::Cmd::Quad { ctrl, to } => builder.curve_to(pt(origin, to), pt(origin, ctrl)),
                                glyphs::Cmd::Cubic { ctrl_a, ctrl_b, to } => builder.cubic_bezier_to(pt(origin, to), pt(origin, ctrl_a), pt(origin, ctrl_b)),
                                glyphs::Cmd::Close => builder.close(),
                            }
                        }
                        if let Ok(path) = builder.build() {
                            window.paint_path(path, color);
                        }
                    }
                    Mark::Outline { rect, color } => {
                        window.paint_quad(gpui::quad(rect_bounds(origin, rect), px(2.), gpui::transparent_black(), px(1.5), color, gpui::BorderStyle::Dashed));
                    }
                    Mark::Handle { rect } => {
                        window.paint_quad(gpui::quad(rect_bounds(origin, rect), px(2.), rgb(0xFFFFFF), px(1.5), rgb(theme::ACCENT), Default::default()));
                    }
                }
            }
            if let Some(thumb) = self.scrollbar {
                window.paint_quad(gpui::quad(rect_bounds(origin, thumb), px(4.), rgba(0x00000040), 0.0, gpui::transparent_black(), Default::default()));
            }
        });
    }
}

#[allow(clippy::too_many_arguments)]
fn paint_text(window: &mut Window, cx: &mut App, origin: Point<Pixels>, text: &str, size: f64, color: Hsla, bold: bool, family: &'static str) {
    if text.is_empty() || size < 1.0 {
        return;
    }
    let mut f = font(family);
    if bold {
        f.weight = FontWeight::BOLD;
    }
    let run = TextRun { len: text.len(), font: f, color, background_color: None, underline: None, strikethrough: None };
    let line = window.text_system().shape_line(SharedString::from(text.to_string()), px_f(size), &[run], None);
    line.paint(origin, px_f(size), window, cx).ok();
}

fn ellipse_points(r: RectD, steps: usize) -> Vec<PointD> {
    (0..steps)
        .map(|i| {
            let t = i as f64 / steps as f64 * std::f64::consts::TAU;
            PointD::new(r.x + r.width / 2.0 * (1.0 + t.cos()), r.y + r.height / 2.0 * (1.0 + t.sin()))
        })
        .collect()
}

fn arrow_head(start: PointD, end: PointD, stroke: f64) -> [PointD; 2] {
    let angle = (end.y - start.y).atan2(end.x - start.x);
    let length = (stroke * 4.0).max(8.0);
    [-0.5f64, 0.5].map(|o| PointD::new(end.x - length * (angle + o).cos(), end.y - length * (angle + o).sin()))
}

fn line_ends(a: &Annotation) -> (PointD, PointD) {
    let r = a.bounds;
    let start = a.points.first().copied().unwrap_or(PointD::new(r.x, r.y));
    let end = a.points.last().copied().filter(|_| a.points.len() > 1).unwrap_or(PointD::new(r.right(), r.bottom()));
    (start, end)
}

fn distance_to_segment(p: PointD, a: PointD, b: PointD) -> f64 {
    let (dx, dy) = (b.x - a.x, b.y - a.y);
    let len = dx * dx + dy * dy;
    if len < 1e-9 {
        return p.distance(a);
    }
    let t = (((p.x - a.x) * dx + (p.y - a.y) * dy) / len).clamp(0.0, 1.0);
    p.distance(PointD::new(a.x + t * dx, a.y + t * dy))
}

/// Screen-space marks for one annotation on a placed page.
fn annotation_marks(a: &Annotation, placement: &PagePlacement, page: &PdfPageState, zoom: f64, marks: &mut Vec<Mark>) {
    let s = |p: PointD| placement.to_screen(page, p, zoom);
    let poly = |r: RectD| vec![s(PointD::new(r.x, r.y)), s(PointD::new(r.right(), r.y)), s(PointD::new(r.right(), r.bottom())), s(PointD::new(r.x, r.bottom()))];
    let mut color = argb(a.color);
    if a.resolved {
        color.a *= 0.45;
    }
    let width = a.stroke_width * zoom;
    let r = a.bounds;
    match a.kind {
        AnnotationKind::Highlight => marks.push(Mark::Polygon { points: poly(r), color: Hsla { a: 0.32, ..color } }),
        AnnotationKind::Underline => marks.push(Mark::Stroke { points: vec![s(PointD::new(r.x, r.bottom())), s(PointD::new(r.right(), r.bottom()))], width, color, closed: false }),
        AnnotationKind::Strikeout => {
            let mid = r.y + r.height / 2.0;
            marks.push(Mark::Stroke { points: vec![s(PointD::new(r.x, mid)), s(PointD::new(r.right(), mid))], width, color, closed: false })
        }
        AnnotationKind::Rectangle => marks.push(Mark::Stroke { points: poly(r), width, color, closed: true }),
        AnnotationKind::Ellipse => marks.push(Mark::Stroke { points: ellipse_points(r, 64).into_iter().map(s).collect(), width, color, closed: true }),
        AnnotationKind::Ink | AnnotationKind::Signature => {
            if a.points.len() == 1 {
                let d = a.stroke_width;
                let dot = RectD::new(a.points[0].x - d / 2.0, a.points[0].y - d / 2.0, d, d);
                marks.push(Mark::Polygon { points: ellipse_points(dot, 16).into_iter().map(s).collect(), color });
            } else {
                marks.push(Mark::Stroke { points: a.points.iter().copied().map(s).collect(), width, color, closed: false });
            }
        }
        AnnotationKind::Line | AnnotationKind::Arrow => {
            let (start, end) = line_ends(a);
            marks.push(Mark::Stroke { points: vec![s(start), s(end)], width, color, closed: false });
            if a.kind == AnnotationKind::Arrow {
                for head in arrow_head(start, end, a.stroke_width) {
                    marks.push(Mark::Stroke { points: vec![s(end), s(head)], width, color, closed: false });
                }
            }
        }
        AnnotationKind::Note => {
            let d = placement.rect_to_screen(page, r, zoom);
            marks.push(Mark::Fill { rect: d, color, radius: (3.0 * zoom) as f32 });
            let white: Hsla = rgb(0xFFFFFF).into();
            for dy in [7.0, 12.0] {
                let y = d.y + dy * zoom;
                marks.push(Mark::Stroke { points: vec![PointD::new(d.x + 5.0 * zoom, y), PointD::new(d.right() - 5.0 * zoom, y)], width: zoom.max(0.5), color: white, closed: false });
            }
        }
        AnnotationKind::Check => {
            let knee = PointD::new(r.x + r.width * 0.35, r.bottom());
            marks.push(Mark::Stroke { points: vec![s(PointD::new(r.x, r.y + r.height / 2.0)), s(knee), s(PointD::new(r.right(), r.y))], width, color, closed: false });
        }
        AnnotationKind::Stamp => {
            marks.push(Mark::Stroke { points: poly(r), width, color, closed: true });
            let size = a.font_size.min(r.height - 12.0).max(4.0);
            let label = if a.text.is_empty() { "APPROVED" } else { a.text.as_str() };
            text_block(label, PointD::new(r.x + 10.0, r.y + 7.0), size, r.width - 20.0, true, color, placement, page, zoom, marks);
        }
        AnnotationKind::Text => text_block(&a.text, PointD::new(r.x, r.y), a.font_size, r.width.max(1.0), false, color, placement, page, zoom, marks),
    }
}

/// Wrapped annotation text whose first line's top-left is `top_left` in page space, laid
/// out like the exported PDF. Upright pages use GPUI's text; rotated pages use glyph
/// outlines so the text turns with the page.
#[allow(clippy::too_many_arguments)]
fn text_block(text: &str, top_left: PointD, size: f64, max_width: f64, bold: bool, color: Hsla, placement: &PagePlacement, page: &PdfPageState, zoom: f64, marks: &mut Vec<Mark>) {
    let lines = text_layout::wrap(text, size, max_width);
    let step = size * text_layout::LINE_HEIGHT;
    if page.rotation % 360 != 0 {
        let mut cmds = Vec::new();
        for (i, line) in lines.iter().enumerate() {
            let baseline = PointD::new(top_left.x, top_left.y + size * text_layout::ASCENT + i as f64 * step);
            match glyphs::line(line, baseline, size, bold) {
                Some(outline) => cmds.extend(outline.into_iter().map(|cmd| cmd.map(|p| placement.to_screen(page, p, zoom)))),
                None => break,
            }
        }
        if !cmds.is_empty() || lines.iter().all(|l| l.trim().is_empty()) {
            marks.push(Mark::Glyphs { cmds, color });
            return;
        }
    }
    for (i, line) in lines.into_iter().enumerate() {
        let origin = placement.to_screen(page, PointD::new(top_left.x, top_left.y + i as f64 * step), zoom);
        marks.push(Mark::Text { origin, text: line.into(), size: size * zoom, color, bold });
    }
}

/// The annotation as it looks mid-gesture (moved or resized), if a gesture is editing it.
fn preview(gesture: &Option<Gesture>, page: &PdfPageState, a: &Annotation) -> Option<Annotation> {
    match gesture {
        Some(Gesture::Move { original, start, current, .. }) if original.id == a.id => Some(original.moved(*current - *start)),
        Some(Gesture::Resize { original, current, .. }) if original.id == a.id => Some(original.resized(layout::page_bounds(page, *current))),
        _ => None,
    }
}

impl DocumentView {
    pub(crate) fn prepare(&mut self, bounds: Bounds<Pixels>, window: &mut Window, cx: &mut Context<Self>) -> Frame {
        self.viewport = bounds;
        self.scale_factor = window.scale_factor();
        if let Some(fit) = self.pending_fit.take() {
            self.fit(fit, cx);
        }
        self.clamp_scroll();
        let (width, height) = self.viewport_size();
        let zoom = self.zoom;
        let doc = self.doc().clone();
        self.placements = layout::arrange(&doc.pages, width, zoom, self.scroll, self.pan, self.layout_mode, self.session.current_page());

        // Follow scrolling with the current page, but never override an explicit navigation.
        if (self.scroll - self.tracked_scroll).abs() > 0.5 && self.gesture.is_none() {
            let probe = height * 0.35;
            if let Some(p) = self.placements.iter().find(|p| p.bounds.y <= probe && p.bounds.bottom() + layout::GAP >= probe)
                && p.index != self.session.current_page()
            {
                self.session.navigate(p.index);
            }
            self.tracked_scroll = self.scroll;
        }

        let scale = zoom * self.scale_factor as f64;
        let margin = height * 0.5;
        let visible: Vec<PagePlacement> = self.placements.iter().copied().filter(|p| p.bounds.bottom() >= -margin && p.bounds.y <= height + margin && p.bounds.right() >= 0.0 && p.bounds.x <= width).collect();
        let mut pages = Vec::with_capacity(visible.len());
        for p in &visible {
            let has_source = doc.pages[p.index].source_id.is_some();
            let (image, exact, failed) = if has_source { self.page_image(p.index, scale, false, cx) } else { (None, true, false) };
            pages.push(PageDraw { rect: p.bounds, loading: has_source && image.is_none() && !exact, failed, image });
        }

        let tool = self.session.tool();
        if matches!(tool, PdfTool::Select | PdfTool::Highlight | PdfTool::Underline | PdfTool::Strikeout) {
            self.words(self.session.current_page(), cx);
        }

        let mut marks = Vec::new();
        for p in &visible {
            let page = &doc.pages[p.index];
            for result in self.search.iter().enumerate().filter(|(_, r)| r.page_index == p.index && r.annotation.is_none()) {
                let active = self.active_search == Some(result.0);
                for b in &result.1.bounds {
                    let color = if active { rgba(0xFF9A0080) } else { rgba(0xFFD40055) };
                    marks.push(Mark::Fill { rect: p.rect_to_screen(page, *b, zoom), color: color.into(), radius: 2.0 });
                }
            }
            for a in &page.annotations {
                let shown = preview(&self.gesture, page, a);
                annotation_marks(shown.as_ref().unwrap_or(a), p, page, zoom, &mut marks);
            }
            if let Some(selection) = self.text_selection.as_ref().filter(|s| s.page == p.index) {
                for b in crate::document::line_boxes(&selection.words) {
                    marks.push(Mark::Fill { rect: p.rect_to_screen(page, b, zoom), color: rgba(0x1473E640).into(), radius: 1.0 });
                }
            }
            if p.index == self.session.current_page()
                && let Some(a) = self.session.selected_annotation()
            {
                let a = preview(&self.gesture, page, a).unwrap_or_else(|| a.clone());
                let r = p.rect_to_screen(page, a.bounds, zoom).inflate(3.0);
                marks.push(Mark::Outline { rect: r, color: rgb(theme::ACCENT).into() });
                if resizable(&a) {
                    for c in corners(r) {
                        marks.push(Mark::Handle { rect: RectD::new(c.x - HANDLE / 2.0, c.y - HANDLE / 2.0, HANDLE, HANDLE) });
                    }
                }
            }
            self.gesture_marks(p, page, &mut marks);
        }

        let (_, content_h) = self.content_size();
        let scrollbar = (content_h > height + 1.0).then(|| {
            let thumb = (height * height / content_h).max(32.0);
            let y = (self.scroll / (content_h - height)) * (height - thumb);
            RectD::new(width - SCROLLBAR - 2.0, y, SCROLLBAR, thumb)
        });
        Frame { origin: bounds.origin, bounds, pages, marks, scrollbar }
    }

    fn gesture_marks(&self, p: &PagePlacement, page: &PdfPageState, marks: &mut Vec<Mark>) {
        let zoom = self.zoom;
        let color = argb(self.session.color);
        let width = self.session.stroke_width * zoom;
        let s = |q: PointD| p.to_screen(page, q, zoom);
        match &self.gesture {
            Some(Gesture::Draw { page: i, points, .. }) if *i == p.index => {
                marks.push(Mark::Stroke { points: points.iter().copied().map(s).collect(), width, color, closed: false });
            }
            Some(Gesture::Shape { page: i, tool, start, end }) if *i == p.index => {
                let r = RectD::between(*start, *end);
                let poly = vec![s(PointD::new(r.x, r.y)), s(PointD::new(r.right(), r.y)), s(PointD::new(r.right(), r.bottom())), s(PointD::new(r.x, r.bottom()))];
                match tool {
                    PdfTool::Ellipse => marks.push(Mark::Stroke { points: ellipse_points(r, 64).into_iter().map(s).collect(), width, color, closed: true }),
                    PdfTool::Line | PdfTool::Arrow => {
                        marks.push(Mark::Stroke { points: vec![s(*start), s(*end)], width, color, closed: false });
                        if *tool == PdfTool::Arrow {
                            for head in arrow_head(*start, *end, self.session.stroke_width) {
                                marks.push(Mark::Stroke { points: vec![s(*end), s(head)], width, color, closed: false });
                            }
                        }
                    }
                    PdfTool::Crop => {
                        marks.push(Mark::Polygon { points: poly.clone(), color: rgba(0x1473E61A).into() });
                        marks.push(Mark::Stroke { points: poly, width: 1.5, color: rgb(theme::ACCENT).into(), closed: true });
                    }
                    _ => marks.push(Mark::Stroke { points: poly, width, color, closed: true }),
                }
            }
            Some(Gesture::TextSelect { page: i, start, end, tool }) if *i == p.index => {
                let words = self.words_between(*i, *start, *end);
                if words.is_empty() && *tool != PdfTool::Select {
                    let r = RectD::between(*start, *end);
                    marks.push(Mark::Polygon {
                        points: vec![s(PointD::new(r.x, r.y)), s(PointD::new(r.right(), r.y)), s(PointD::new(r.right(), r.bottom())), s(PointD::new(r.x, r.bottom()))],
                        color: Hsla { a: 0.25, ..color },
                    });
                }
                for b in crate::document::line_boxes(&words) {
                    marks.push(Mark::Fill { rect: p.rect_to_screen(page, b, zoom), color: rgba(0x1473E640).into(), radius: 1.0 });
                }
            }
            _ => {}
        }
    }

    // ---- Hit testing ----------------------------------------------------------------------

    fn local(&self, position: Point<Pixels>) -> PointD {
        PointD::new(f64::from(position.x - self.viewport.origin.x), f64::from(position.y - self.viewport.origin.y))
    }

    fn placement_at(&self, local: PointD, slack: f64) -> Option<PagePlacement> {
        self.placements.iter().copied().find(|p| p.bounds.inflate(slack).contains(local))
    }

    fn placement(&self, index: usize) -> Option<PagePlacement> {
        self.placements.iter().copied().find(|p| p.index == index)
    }

    fn page_point(&self, index: usize, local: PointD) -> Option<PointD> {
        let p = self.placement(index)?;
        let page = self.doc().pages.get(index)?;
        let q = p.to_page(page, local, self.zoom);
        let b = page.visible_box();
        Some(PointD::new(q.x.clamp(b.x, b.right()), q.y.clamp(b.y, b.bottom())))
    }

    fn hit_annotation(&self, index: usize, q: PointD) -> Option<Uuid> {
        let tolerance = 5.0 / self.zoom;
        let page = &self.doc().pages[index];
        page.annotations.iter().rev().find(|a| {
            let slack = tolerance + a.stroke_width / 2.0;
            match a.kind {
                AnnotationKind::Ink | AnnotationKind::Signature => {
                    a.points.windows(2).any(|w| distance_to_segment(q, w[0], w[1]) <= slack) || (a.points.len() == 1 && a.points[0].distance(q) <= slack)
                }
                AnnotationKind::Line | AnnotationKind::Arrow => {
                    let (s, e) = line_ends(a);
                    distance_to_segment(q, s, e) <= slack
                }
                _ => a.bounds.inflate(slack).contains(q),
            }
        })
        .map(|a| a.id)
    }

    fn hit_handle(&self, local: PointD) -> Option<(usize, usize, RectD)> {
        let index = self.session.current_page();
        let a = self.session.selected_annotation().filter(|a| resizable(a))?;
        let p = self.placement(index)?;
        let page = &self.doc().pages[index];
        let r = p.rect_to_screen(page, a.bounds, self.zoom).inflate(3.0);
        corners(r).iter().position(|c| (c.x - local.x).abs() <= HANDLE && (c.y - local.y).abs() <= HANDLE).map(|corner| (index, corner, layout::display_bounds(page, a.bounds)))
    }

    /// Words from the one nearest `a` to the one nearest `b`, in reading order.
    pub(crate) fn words_between(&self, index: usize, a: PointD, b: PointD) -> Vec<refr_pdf::PdfWord> {
        let Some(page) = self.doc().pages.get(index) else { return Vec::new() };
        let Some(key) = page.source_id.map(|id| (id, page.source_page)) else { return Vec::new() };
        let Some(words) = self.cached_words(key) else { return Vec::new() };
        let nearest = |q: PointD| -> Option<(usize, f64)> {
            words
                .iter()
                .enumerate()
                .map(|(i, w)| {
                    let b = w.bounds;
                    let dx = (b.x - q.x).max(q.x - b.right()).max(0.0);
                    let dy = (b.y - q.y).max(q.y - b.bottom()).max(0.0);
                    (i, dx * 0.5 + dy * 2.0)
                })
                .min_by(|x, y| x.1.total_cmp(&y.1))
        };
        let (Some((i, di)), Some((j, dj))) = (nearest(a), nearest(b)) else { return Vec::new() };
        // A drag that starts and ends far from any text selects nothing.
        if di > 30.0 && dj > 30.0 || (a.distance(b) < 2.0 && di > 0.0) {
            return Vec::new();
        }
        let (from, to) = if i <= j { (i, j) } else { (j, i) };
        words[from..=to].to_vec()
    }

    // ---- Pointer input --------------------------------------------------------------------

    fn on_mouse_down(&mut self, event: &MouseDownEvent, window: &mut Window, cx: &mut Context<Self>) {
        window.focus(&self.focus_handle);
        // Focus is handled here; stop GPUI's click-to-focus from taking it back from the
        // inline text editor that a tool may open below.
        window.prevent_default();
        if self.inline_text.is_some() {
            self.finish_text(true, cx);
            return;
        }
        let local = self.local(event.position);
        let (width, _) = self.viewport_size();
        // Scrollbar.
        let (_, content_h) = self.content_size();
        let (_, height) = self.viewport_size();
        if content_h > height && local.x >= width - SCROLLBAR - 6.0 {
            let thumb = (height * height / content_h).max(32.0);
            let y = (self.scroll / (content_h - height)) * (height - thumb);
            let grab = if local.y >= y && local.y <= y + thumb { local.y - y } else { thumb / 2.0 };
            self.gesture = Some(Gesture::ScrollDrag { grab });
            self.drag_scrollbar(local.y, cx);
            return;
        }
        let tool = self.session.tool();
        if tool == PdfTool::Hand {
            self.gesture = Some(Gesture::Pan { last: event.position });
            cx.notify();
            return;
        }
        if tool == PdfTool::Select
            && let Some((page, corner, display)) = self.hit_handle(local)
        {
            let original = self.session.selected_annotation().cloned().expect("handle belongs to the selection");
            self.gesture = Some(Gesture::Resize { page, original, corner, display, current: display });
            return;
        }
        let Some(p) = self.placement_at(local, 0.0) else {
            self.session.select(None);
            self.text_selection = None;
            cx.notify();
            return;
        };
        let index = p.index;
        let page = self.doc().pages[index].clone();
        let q = p.to_page(&page, local, self.zoom);
        if index != self.session.current_page() {
            self.session.navigate(index);
        }
        match tool {
            PdfTool::Select => {
                self.text_selection = None;
                if let Some(id) = self.hit_annotation(index, q) {
                    self.session.select(Some(id));
                    let a = page.annotation(id).cloned().expect("hit annotation exists");
                    if event.click_count >= 2 {
                        match a.kind {
                            AnnotationKind::Text => self.start_text(index, PointD::new(a.bounds.x, a.bounds.y), Some(id), window, cx),
                            AnnotationKind::Note | AnnotationKind::Stamp => cx.emit(DocumentEvent::EditNote(id)),
                            _ => {}
                        }
                    } else {
                        self.gesture = Some(Gesture::Move { page: index, original: a, start: q, current: q });
                    }
                } else {
                    self.session.select(None);
                    self.gesture = Some(Gesture::TextSelect { page: index, tool, start: q, end: q });
                }
            }
            PdfTool::Highlight | PdfTool::Underline | PdfTool::Strikeout => {
                self.gesture = Some(Gesture::TextSelect { page: index, tool, start: q, end: q });
            }
            PdfTool::Ink | PdfTool::Signature => self.gesture = Some(Gesture::Draw { page: index, tool, points: vec![q] }),
            PdfTool::Rectangle | PdfTool::Ellipse | PdfTool::Line | PdfTool::Arrow | PdfTool::Crop => {
                self.gesture = Some(Gesture::Shape { page: index, tool, start: q, end: q })
            }
            PdfTool::Text => self.start_text(index, q, None, window, cx),
            PdfTool::Note => cx.emit(DocumentEvent::NoteRequested { page: index, point: q }),
            PdfTool::Stamp => {
                let bounds = RectD::new(q.x - 75.0, q.y - 20.0, 150.0, 40.0);
                let a = Annotation { color: self.session.color, stroke_width: self.session.stroke_width, font_size: 18.0, text: "APPROVED".into(), ..Annotation::new(AnnotationKind::Stamp, bounds) };
                self.edit(cx, |s| s.add_annotation(a, Some(index)));
            }
            PdfTool::Check => {
                let bounds = RectD::new(q.x - 10.0, q.y - 8.0, 20.0, 16.0);
                let a = Annotation { color: self.session.color, stroke_width: self.session.stroke_width.max(2.0), ..Annotation::new(AnnotationKind::Check, bounds) };
                self.edit(cx, |s| s.add_annotation(a, Some(index)));
            }
            PdfTool::Hand => {}
        }
        cx.notify();
    }

    fn drag_scrollbar(&mut self, y: f64, cx: &mut Context<Self>) {
        let Some(Gesture::ScrollDrag { grab }) = self.gesture else { return };
        let (_, height) = self.viewport_size();
        let (_, content_h) = self.content_size();
        let thumb = (height * height / content_h).max(32.0);
        let fraction = ((y - grab) / (height - thumb).max(1.0)).clamp(0.0, 1.0);
        self.scroll = fraction * (content_h - height);
        self.clamp_scroll();
        cx.notify();
    }

    fn on_mouse_move(&mut self, event: &MouseMoveEvent, _window: &mut Window, cx: &mut Context<Self>) {
        let Some(gesture) = self.gesture.as_mut() else { return };
        if event.pressed_button.is_none() {
            return;
        }
        let local = PointD::new(f64::from(event.position.x - self.viewport.origin.x), f64::from(event.position.y - self.viewport.origin.y));
        match gesture {
            Gesture::Pan { last } => {
                let delta = event.position - *last;
                *last = event.position;
                self.scroll_by(f64::from(delta.x), f64::from(delta.y), cx);
                return;
            }
            Gesture::ScrollDrag { .. } => {
                self.drag_scrollbar(local.y, cx);
                return;
            }
            _ => {}
        }
        let page_index = match gesture {
            Gesture::Move { page, .. } | Gesture::Resize { page, .. } | Gesture::Draw { page, .. } | Gesture::Shape { page, .. } | Gesture::TextSelect { page, .. } => *page,
            _ => return,
        };
        let Some(q) = self.page_point(page_index, local) else { return };
        let zoom = self.zoom;
        let page = self.doc().pages[page_index].clone();
        match self.gesture.as_mut() {
            Some(Gesture::Move { current, .. }) => *current = q,
            Some(Gesture::Draw { points, .. }) => {
                if points.last().is_none_or(|last| last.distance(q) * zoom >= 1.5) && points.len() < 100_000 {
                    points.push(q);
                }
            }
            Some(Gesture::Shape { end, .. }) | Some(Gesture::TextSelect { end, .. }) => *end = q,
            Some(Gesture::Resize { corner, display, current, .. }) => {
                let d = layout::to_display(&page, q);
                let (mut l, mut t, mut r, mut b) = (display.x, display.y, display.right(), display.bottom());
                match corner {
                    0 => (l, t) = (d.x, d.y),
                    1 => (r, t) = (d.x, d.y),
                    2 => (r, b) = (d.x, d.y),
                    _ => (l, b) = (d.x, d.y),
                }
                let min = 4.0;
                *current = RectD::between(PointD::new(l, t), PointD::new(r, b));
                current.width = current.width.max(min);
                current.height = current.height.max(min);
            }
            _ => {}
        }
        cx.notify();
    }

    fn on_mouse_up(&mut self, _: &MouseUpEvent, _window: &mut Window, cx: &mut Context<Self>) {
        let Some(gesture) = self.gesture.take() else { return };
        let session_color = self.session.color;
        let stroke = self.session.stroke_width;
        match gesture {
            Gesture::Pan { .. } | Gesture::ScrollDrag { .. } => {}
            Gesture::Move { original, start, current, .. } => {
                let delta = current - start;
                if delta.x.abs() > 0.01 || delta.y.abs() > 0.01 {
                    self.edit(cx, |s| s.update_annotation(original.id, "Move annotation", |a| a.moved(delta)));
                }
            }
            Gesture::Resize { page, original, current, display, .. } => {
                if current != display {
                    let rect = layout::page_bounds(&self.doc().pages[page], current);
                    self.edit(cx, |s| {
                        s.update_annotation(original.id, "Resize annotation", |a| {
                            let mut resized = a.resized(rect);
                            if a.kind == AnnotationKind::Text {
                                // Text reflows to the new width; its height follows the text.
                                let lines = text_layout::wrap(&a.text, a.font_size, rect.width).len();
                                resized.bounds.height = text_layout::height(lines, a.font_size);
                            }
                            resized
                        })
                    });
                }
            }
            Gesture::Draw { page, tool, points } => {
                let kind = if tool == PdfTool::Signature { AnnotationKind::Signature } else { AnnotationKind::Ink };
                if let Some(bounds) = RectD::bounding(&points) {
                    let a = Annotation { color: if tool == PdfTool::Signature && session_color == refr_core::model::DEFAULT_COLOR { 0xFF1A1A6E } else { session_color }, stroke_width: stroke, points, ..Annotation::new(kind, bounds) };
                    self.edit(cx, |s| s.add_annotation(a, Some(page)));
                }
            }
            Gesture::Shape { page, tool, start, end } => {
                let r = RectD::between(start, end);
                match tool {
                    PdfTool::Crop => {
                        if r.width >= 10.0 && r.height >= 10.0 {
                            self.edit(cx, |s| s.crop_page(page, Some(r)));
                            self.status("Page cropped. Cropping hides content; it does not remove it.", cx);
                        }
                    }
                    PdfTool::Line | PdfTool::Arrow => {
                        if start.distance(end) >= 2.0 {
                            let kind = if tool == PdfTool::Arrow { AnnotationKind::Arrow } else { AnnotationKind::Line };
                            let a = Annotation { color: session_color, stroke_width: stroke, points: vec![start, end], ..Annotation::new(kind, r) };
                            self.edit(cx, |s| s.add_annotation(a, Some(page)));
                        }
                    }
                    _ => {
                        if r.width >= 2.0 || r.height >= 2.0 {
                            let kind = if tool == PdfTool::Ellipse { AnnotationKind::Ellipse } else { AnnotationKind::Rectangle };
                            let a = Annotation { color: session_color, stroke_width: stroke, ..Annotation::new(kind, r) };
                            self.edit(cx, |s| s.add_annotation(a, Some(page)));
                        }
                    }
                }
            }
            Gesture::TextSelect { page, tool, start, end } => {
                let words = self.words_between(page, start, end);
                if tool == PdfTool::Select {
                    if words.is_empty() {
                        self.text_selection = None;
                    } else {
                        let count = words.len();
                        self.text_selection = Some(TextSelection { page, words });
                        self.status(format!("{count} word{} selected. ⌘C copies.", if count == 1 { "" } else { "s" }), cx);
                    }
                } else {
                    let kind = match tool {
                        PdfTool::Underline => AnnotationKind::Underline,
                        PdfTool::Strikeout => AnnotationKind::Strikeout,
                        _ => AnnotationKind::Highlight,
                    };
                    if !words.is_empty() {
                        self.text_selection = Some(TextSelection { page, words });
                        self.markup_selection(kind, cx);
                    } else {
                        let r = RectD::between(start, end);
                        if r.width >= 4.0 && r.height >= 4.0 {
                            let a = Annotation { color: session_color, stroke_width: stroke.max(1.0), ..Annotation::new(kind, r) };
                            self.edit(cx, |s| s.add_annotation(a, Some(page)));
                        } else {
                            self.status("No text here. Drag across selectable text, or drag an area to mark it.", cx);
                        }
                    }
                }
            }
        }
        cx.notify();
    }

    fn on_scroll(&mut self, event: &ScrollWheelEvent, window: &mut Window, cx: &mut Context<Self>) {
        let delta = event.delta.pixel_delta(window.line_height());
        let (dx, dy) = (f64::from(delta.x), f64::from(delta.y));
        if event.modifiers.platform || event.modifiers.control {
            let factor = (dy * 0.01).exp();
            let anchor = self.local(event.position);
            self.zoom_to(self.zoom * factor, Some(anchor), cx);
        } else if event.modifiers.shift && dx.abs() < 0.01 {
            self.scroll_by(dy, 0.0, cx);
        } else {
            self.scroll_by(dx, dy, cx);
        }
    }

    // ---- Keyboard actions -------------------------------------------------------------------

    fn nudge(&mut self, dx: f64, dy: f64, cx: &mut Context<Self>) {
        let page = self.session.page().clone();
        let delta = layout::to_page(&page, PointD::new(dx, dy)) - layout::to_page(&page, PointD::new(0.0, 0.0));
        if self.session.selected_annotation().is_some() {
            self.edit(cx, |s| s.move_selection(delta));
        } else {
            self.scroll_by(-dx * 4.0, -dy * 4.0, cx);
        }
    }

    fn cursor(&self) -> CursorStyle {
        match (self.session.tool(), &self.gesture) {
            (_, Some(Gesture::Pan { .. })) => CursorStyle::ClosedHand,
            (PdfTool::Hand, _) => CursorStyle::OpenHand,
            (PdfTool::Select, Some(Gesture::Move { .. })) => CursorStyle::ClosedHand,
            (PdfTool::Select, _) => CursorStyle::Arrow,
            (PdfTool::Text, _) | (PdfTool::Highlight | PdfTool::Underline | PdfTool::Strikeout, _) => CursorStyle::IBeam,
            _ => CursorStyle::Crosshair,
        }
    }
}

fn corners(r: RectD) -> [PointD; 4] {
    [PointD::new(r.x, r.y), PointD::new(r.right(), r.y), PointD::new(r.right(), r.bottom()), PointD::new(r.x, r.bottom())]
}

fn resizable(a: &Annotation) -> bool {
    !a.kind.is_text_markup() && a.kind != AnnotationKind::Note
}

impl Render for DocumentView {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let entity = cx.entity();
        let inline = self.inline_text.as_ref().and_then(|inline| {
            let p = self.placement(inline.page)?;
            let page = &self.doc().pages[inline.page];
            let at = p.to_screen(page, inline.origin, self.zoom);
            let size = self.session.font_size * self.zoom;
            Some(
                div()
                    .absolute()
                    .left(px_f(at.x - 4.0))
                    .top(px_f(at.y - 4.0))
                    .w(px_f((inline.width * self.zoom).max(160.0)))
                    .p(px(3.))
                    .bg(rgb(0xFFFFFF))
                    .border_1()
                    .border_color(rgb(theme::ACCENT))
                    .rounded(px(3.))
                    .shadow_md()
                    .text_size(px_f(size.clamp(10.0, 48.0)))
                    .font_family(theme::ANNOTATION_FONT)
                    .text_color(argb(self.session.color))
                    .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
                    .child(inline.input.clone()),
            )
        });
        div()
            .id("viewport")
            .key_context("Viewport")
            .track_focus(&self.focus_handle)
            .size_full()
            .relative()
            .overflow_hidden()
            .cursor(self.cursor())
            .on_action(cx.listener(|this, _: &DeleteSelection, _, cx| {
                if this.session.selected_annotation().is_some() {
                    this.edit(cx, |s| s.delete_selection());
                }
            }))
            .on_action(cx.listener(|this, _: &NudgeLeft, _, cx| this.nudge(-1.0, 0.0, cx)))
            .on_action(cx.listener(|this, _: &NudgeRight, _, cx| this.nudge(1.0, 0.0, cx)))
            .on_action(cx.listener(|this, _: &NudgeUp, _, cx| this.nudge(0.0, -1.0, cx)))
            .on_action(cx.listener(|this, _: &NudgeDown, _, cx| this.nudge(0.0, 1.0, cx)))
            .on_action(cx.listener(|this, _: &NudgeLeftFar, _, cx| this.nudge(-10.0, 0.0, cx)))
            .on_action(cx.listener(|this, _: &NudgeRightFar, _, cx| this.nudge(10.0, 0.0, cx)))
            .on_action(cx.listener(|this, _: &NudgeUpFar, _, cx| this.nudge(0.0, -10.0, cx)))
            .on_action(cx.listener(|this, _: &NudgeDownFar, _, cx| this.nudge(0.0, 10.0, cx)))
            .on_action(cx.listener(|this, _: &CancelGesture, _, cx| {
                this.gesture = None;
                this.text_selection = None;
                this.session.select(None);
                cx.notify();
            }))
            .on_action(cx.listener(|this, _: &Copy, _, cx| this.copy_selection(cx)))
            .on_action(cx.listener(|this, _: &ToolSelect, _, cx| this.set_tool(PdfTool::Select, cx)))
            .on_action(cx.listener(|this, _: &ToolHand, _, cx| this.set_tool(PdfTool::Hand, cx)))
            .on_action(cx.listener(|this, _: &ToolText, _, cx| this.set_tool(PdfTool::Text, cx)))
            .on_action(cx.listener(|this, _: &ToolInk, _, cx| this.set_tool(PdfTool::Ink, cx)))
            .on_action(cx.listener(|this, _: &FitPage, _, cx| this.fit(Fit::Page, cx)))
            .on_mouse_down(MouseButton::Left, cx.listener(Self::on_mouse_down))
            .on_mouse_down(
                MouseButton::Middle,
                cx.listener(|this, e: &MouseDownEvent, _, cx| {
                    this.gesture = Some(Gesture::Pan { last: e.position });
                    cx.notify();
                }),
            )
            .on_mouse_move(cx.listener(Self::on_mouse_move))
            .on_mouse_up(MouseButton::Left, cx.listener(Self::on_mouse_up))
            .on_mouse_up_out(MouseButton::Left, cx.listener(Self::on_mouse_up))
            .on_mouse_up(MouseButton::Middle, cx.listener(Self::on_mouse_up))
            .on_mouse_up_out(MouseButton::Middle, cx.listener(Self::on_mouse_up))
            .on_scroll_wheel(cx.listener(Self::on_scroll))
            .child(
                canvas(
                    move |bounds, window, cx| entity.update(cx, |view, cx| view.prepare(bounds, window, cx)),
                    |_, frame, window, cx| frame.paint(window, cx),
                )
                .size_full(),
            )
            .children(inline)
    }
}
