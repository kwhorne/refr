//! Writes workspace annotations into a PDF page as vector page objects.
//!
//! Objects are built in logical page coordinates and placed with the page's logical→user
//! matrix, so intrinsic `/Rotate` and box offsets need no special cases here.

use pdfium_render::prelude::*;
use refr_core::text_layout;
use refr_core::{Annotation, AnnotationKind, PointD, RectD};

use crate::frame::Matrix;

pub struct Fonts {
    pub regular: PdfFontToken,
    pub bold: PdfFontToken,
}

impl Fonts {
    pub fn new(document: &mut PdfDocument) -> Fonts {
        let fonts = document.fonts_mut();
        Fonts { regular: fonts.helvetica(), bold: fonts.helvetica_bold() }
    }
}

pub fn color(argb: u32) -> PdfColor {
    PdfColor::new((argb >> 16) as u8, (argb >> 8) as u8, argb as u8, (argb >> 24) as u8)
}

fn pts(v: f64) -> PdfPoints {
    PdfPoints::new(v as f32)
}

pub struct Painter<'d, 'a> {
    pub document: &'d PdfDocument<'a>,
    pub page: &'d mut PdfPage<'a>,
    pub matrix: Matrix,
    pub fonts: &'d Fonts,
}

impl<'d, 'a> Painter<'d, 'a> {
    fn place(&mut self, mut path: PdfPagePathObject<'a>) -> Result<(), PdfiumError> {
        path.apply_matrix(self.matrix.to_pdfium())?;
        self.page.objects_mut().add_path_object(path)?;
        Ok(())
    }

    fn stroke_path(&mut self, points: &[PointD], stroke: PdfColor, width: f64, closed: bool) -> Result<(), PdfiumError> {
        let Some(first) = points.first() else { return Ok(()) };
        let mut path = PdfPagePathObject::new(self.document, pts(first.x), pts(first.y), Some(stroke), Some(pts(width)), None)?;
        for p in &points[1..] {
            path.line_to(pts(p.x), pts(p.y))?;
        }
        if closed {
            path.close_path()?;
        }
        path.set_line_cap(PdfPageObjectLineCap::Round)?;
        path.set_line_join(PdfPageObjectLineJoin::Round)?;
        self.place(path)
    }

    pub fn line(&mut self, a: PointD, b: PointD, stroke: PdfColor, width: f64) -> Result<(), PdfiumError> {
        self.stroke_path(&[a, b], stroke, width, false)
    }

    pub fn rect(&mut self, r: RectD, stroke: Option<(PdfColor, f64)>, fill: Option<PdfColor>) -> Result<(), PdfiumError> {
        let rect = PdfRect::new_from_values(r.y as f32, r.x as f32, r.bottom() as f32, r.right() as f32);
        let path = PdfPagePathObject::new_rect(self.document, rect, stroke.map(|s| s.0), stroke.map(|s| pts(s.1)), fill)?;
        self.place(path)
    }

    pub fn ellipse(&mut self, r: RectD, stroke: PdfColor, width: f64) -> Result<(), PdfiumError> {
        let rect = PdfRect::new_from_values(r.y as f32, r.x as f32, r.bottom() as f32, r.right() as f32);
        let path = PdfPagePathObject::new_ellipse(self.document, rect, Some(stroke), Some(pts(width)), None)?;
        self.place(path)
    }

    fn dot(&mut self, center: PointD, radius: f64, fill: PdfColor) -> Result<(), PdfiumError> {
        let r = RectD::new(center.x - radius, center.y - radius, radius * 2.0, radius * 2.0);
        let rect = PdfRect::new_from_values(r.y as f32, r.x as f32, r.bottom() as f32, r.right() as f32);
        let path = PdfPagePathObject::new_ellipse(self.document, rect, None, None, Some(fill))?;
        self.place(path)
    }

    /// Draws wrapped text whose first line's top edge is at `(x, top)` in logical space.
    pub fn text(&mut self, text: &str, x: f64, top: f64, size: f64, fill: PdfColor, max_width: f64, bold: bool) -> Result<(), PdfiumError> {
        let font = if bold { self.fonts.bold } else { self.fonts.regular };
        let mut baseline = top + size * text_layout::ASCENT;
        for line in text_layout::wrap(text, size, max_width) {
            if !line.trim().is_empty() {
                let mut object = PdfPageTextObject::new(self.document, &line, font, pts(size))?;
                object.set_fill_color(fill)?;
                // Glyphs are drawn y-up; flip locally so they read upright in y-down logical space.
                let local = Matrix { a: 1.0, b: 0.0, c: 0.0, d: -1.0, e: x, f: baseline };
                object.apply_matrix(local.then(&self.matrix).to_pdfium())?;
                self.page.objects_mut().add_text_object(object)?;
            }
            baseline += size * text_layout::LINE_HEIGHT;
        }
        Ok(())
    }

    pub fn annotation(&mut self, a: &Annotation) -> Result<(), PdfiumError> {
        let r = a.bounds;
        let c = color(a.color);
        let w = a.stroke_width;
        match a.kind {
            AnnotationKind::Highlight => {
                let fill = PdfColor::new(c.red(), c.green(), c.blue(), 75);
                let rect = PdfRect::new_from_values(r.y as f32, r.x as f32, r.bottom() as f32, r.right() as f32);
                let mut path = PdfPagePathObject::new_rect(self.document, rect, None, None, Some(fill))?;
                path.set_blend_mode(PdfPageObjectBlendMode::Multiply)?;
                self.place(path)?;
            }
            AnnotationKind::Underline => self.line(PointD::new(r.x, r.bottom()), PointD::new(r.right(), r.bottom()), c, w)?,
            AnnotationKind::Strikeout => {
                let mid = r.y + r.height / 2.0;
                self.line(PointD::new(r.x, mid), PointD::new(r.right(), mid), c, w)?
            }
            AnnotationKind::Rectangle => self.rect(r, Some((c, w)), None)?,
            AnnotationKind::Ellipse => self.ellipse(r, c, w)?,
            AnnotationKind::Ink | AnnotationKind::Signature => {
                if a.points.len() == 1 {
                    self.dot(a.points[0], w / 2.0, c)?;
                } else {
                    self.stroke_path(&a.points, c, w, false)?;
                }
            }
            AnnotationKind::Line | AnnotationKind::Arrow => {
                let start = a.points.first().copied().unwrap_or(PointD::new(r.x, r.y));
                let end = a.points.last().copied().filter(|_| a.points.len() > 1).unwrap_or(PointD::new(r.right(), r.bottom()));
                self.line(start, end, c, w)?;
                if a.kind == AnnotationKind::Arrow {
                    for head in arrow_head(start, end, w) {
                        self.line(end, head, c, w)?;
                    }
                }
            }
            AnnotationKind::Note => {
                self.rect(r, None, Some(c))?;
                let white = PdfColor::new(255, 255, 255, 255);
                self.line(PointD::new(r.x + 5.0, r.y + 7.0), PointD::new(r.right() - 5.0, r.y + 7.0), white, 1.0)?;
                self.line(PointD::new(r.x + 5.0, r.y + 12.0), PointD::new(r.right() - 5.0, r.y + 12.0), white, 1.0)?;
            }
            AnnotationKind::Check => {
                let knee = PointD::new(r.x + r.width * 0.35, r.bottom());
                self.stroke_path(&[PointD::new(r.x, r.y + r.height / 2.0), knee, PointD::new(r.right(), r.y)], c, w, false)?;
            }
            AnnotationKind::Stamp => {
                self.rect(r, Some((c, w)), None)?;
                let label = if a.text.is_empty() { "APPROVED" } else { a.text.as_str() };
                self.text(label, r.x + 10.0, r.y + 7.0, stamp_font_size(a), c, r.width - 20.0, true)?;
            }
            AnnotationKind::Text => self.text(&a.text, r.x, r.y, a.font_size, c, r.width.max(1.0), false)?,
        }
        Ok(())
    }
}

pub fn stamp_font_size(a: &Annotation) -> f64 {
    a.font_size.min(a.bounds.height - 12.0).max(4.0)
}

/// The two end points of an arrow head at `end`.
pub fn arrow_head(start: PointD, end: PointD, stroke_width: f64) -> [PointD; 2] {
    let angle = (end.y - start.y).atan2(end.x - start.x);
    let length = (stroke_width * 4.0).max(8.0);
    [-0.5f64, 0.5].map(|offset| PointD::new(end.x - length * (angle + offset).cos(), end.y - length * (angle + offset).sin()))
}
