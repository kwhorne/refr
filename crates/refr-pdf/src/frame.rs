//! Mapping between PDF user space and Refr's logical page space.
//!
//! The logical page is what PDFium displays at no extra rotation: the page's effective box
//! (media ∩ crop) turned by its intrinsic `/Rotate`, with a top-left origin. Workspace
//! annotations, crops and word bounds all live in this space.

use pdfium_render::prelude::*;
use refr_core::{PointD, RectD};

#[derive(Clone, Copy, Debug)]
pub struct PageFrame {
    pub left: f64,
    pub bottom: f64,
    pub right: f64,
    pub top: f64,
    /// Intrinsic clockwise rotation in degrees.
    pub rotation: u32,
}

/// A PDF transformation matrix: `x' = a·x + c·y + e`, `y' = b·x + d·y + f`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Matrix {
    pub a: f64,
    pub b: f64,
    pub c: f64,
    pub d: f64,
    pub e: f64,
    pub f: f64,
}

impl Matrix {
    pub fn apply(&self, x: f64, y: f64) -> (f64, f64) {
        (self.a * x + self.c * y + self.e, self.b * x + self.d * y + self.f)
    }

    /// The matrix that applies `self` first, then `then`.
    pub fn then(&self, then: &Matrix) -> Matrix {
        Matrix {
            a: self.a * then.a + self.b * then.c,
            b: self.a * then.b + self.b * then.d,
            c: self.c * then.a + self.d * then.c,
            d: self.c * then.b + self.d * then.d,
            e: self.e * then.a + self.f * then.c + then.e,
            f: self.e * then.b + self.f * then.d + then.f,
        }
    }

    pub fn to_pdfium(self) -> PdfMatrix {
        PdfMatrix::new(self.a as f32, self.b as f32, self.c as f32, self.d as f32, self.e as f32, self.f as f32)
    }
}

pub fn rotation_degrees(rotation: PdfPageRenderRotation) -> u32 {
    match rotation {
        PdfPageRenderRotation::None => 0,
        PdfPageRenderRotation::Degrees90 => 90,
        PdfPageRenderRotation::Degrees180 => 180,
        PdfPageRenderRotation::Degrees270 => 270,
    }
}

pub fn rotation_from_degrees(degrees: u32) -> PdfPageRenderRotation {
    match degrees % 360 {
        90 => PdfPageRenderRotation::Degrees90,
        180 => PdfPageRenderRotation::Degrees180,
        270 => PdfPageRenderRotation::Degrees270,
        _ => PdfPageRenderRotation::None,
    }
}

impl PageFrame {
    pub fn of(page: &PdfPage) -> PageFrame {
        let rect = page
            .boundaries()
            .bounding()
            .map(|b| b.bounds)
            .unwrap_or_else(|_| PdfRect::new_from_values(0.0, 0.0, page.height().value, page.width().value));
        PageFrame {
            left: rect.left().value as f64,
            bottom: rect.bottom().value as f64,
            right: rect.right().value as f64,
            top: rect.top().value as f64,
            rotation: page.rotation().map(rotation_degrees).unwrap_or(0),
        }
    }

    fn box_width(&self) -> f64 {
        self.right - self.left
    }

    fn box_height(&self) -> f64 {
        self.top - self.bottom
    }

    pub fn logical_width(&self) -> f64 {
        if self.rotation % 180 == 0 { self.box_width() } else { self.box_height() }
    }

    pub fn logical_height(&self) -> f64 {
        if self.rotation % 180 == 0 { self.box_height() } else { self.box_width() }
    }

    pub fn user_to_logical(&self, x: f64, y: f64) -> PointD {
        let (dx, dy) = (x - self.left, self.top - y);
        let (bw, bh) = (self.box_width(), self.box_height());
        match self.rotation {
            90 => PointD::new(bh - dy, dx),
            180 => PointD::new(bw - dx, bh - dy),
            270 => PointD::new(dy, bw - dx),
            _ => PointD::new(dx, dy),
        }
    }

    pub fn logical_to_user(&self, p: PointD) -> (f64, f64) {
        let (bw, bh) = (self.box_width(), self.box_height());
        let (dx, dy) = match self.rotation {
            90 => (p.y, bh - p.x),
            180 => (bw - p.x, bh - p.y),
            270 => (bw - p.y, p.x),
            _ => (p.x, p.y),
        };
        (self.left + dx, self.top - dy)
    }

    /// Logical page space → PDF user space as an affine matrix.
    pub fn logical_to_user_matrix(&self) -> Matrix {
        let (e, f) = self.logical_to_user(PointD::new(0.0, 0.0));
        let (x1, y1) = self.logical_to_user(PointD::new(1.0, 0.0));
        let (x2, y2) = self.logical_to_user(PointD::new(0.0, 1.0));
        Matrix { a: x1 - e, b: y1 - f, c: x2 - e, d: y2 - f, e, f }
    }

    pub fn rect_to_logical(&self, rect: PdfRect) -> RectD {
        let a = self.user_to_logical(rect.left().value as f64, rect.bottom().value as f64);
        let b = self.user_to_logical(rect.right().value as f64, rect.top().value as f64);
        RectD::between(a, b)
    }

    pub fn rect_to_user(&self, rect: RectD) -> PdfRect {
        let (x1, y1) = self.logical_to_user(PointD::new(rect.x, rect.y));
        let (x2, y2) = self.logical_to_user(PointD::new(rect.right(), rect.bottom()));
        PdfRect::new_from_values(y1.min(y2) as f32, x1.min(x2) as f32, y1.max(y2) as f32, x1.max(x2) as f32)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn logical_and_user_space_round_trip() {
        for rotation in [0, 90, 180, 270] {
            let frame = PageFrame { left: 10.0, bottom: 20.0, right: 610.0, top: 820.0, rotation };
            let p = PointD::new(33.0, 44.0);
            let (x, y) = frame.logical_to_user(p);
            assert!(frame.user_to_logical(x, y).distance(p) < 1e-9);
            let m = frame.logical_to_user_matrix();
            let (mx, my) = m.apply(p.x, p.y);
            assert!((mx - x).abs() < 1e-9 && (my - y).abs() < 1e-9);
        }
    }
}
