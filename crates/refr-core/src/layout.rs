//! Reversible page transforms and viewport page placement.
//!
//! Page space: PDF points, top-left origin, in the unrotated, uncropped logical page.
//! Display space: points after crop and quarter-turn rotation.
//! Screen space: display space scaled by zoom and placed by [`arrange`].

use crate::geometry::{PointD, RectD};
use crate::model::PdfPageState;

pub fn to_display(page: &PdfPageState, point: PointD) -> PointD {
    let b = page.visible_box();
    let (x, y) = (point.x - b.x, point.y - b.y);
    match page.rotation {
        90 => PointD::new(b.height - y, x),
        180 => PointD::new(b.width - x, b.height - y),
        270 => PointD::new(y, b.width - x),
        _ => PointD::new(x, y),
    }
}

pub fn to_page(page: &PdfPageState, display: PointD) -> PointD {
    let b = page.visible_box();
    let p = match page.rotation {
        90 => PointD::new(display.y, b.height - display.x),
        180 => PointD::new(b.width - display.x, b.height - display.y),
        270 => PointD::new(b.width - display.y, display.x),
        _ => display,
    };
    PointD::new(p.x + b.x, p.y + b.y)
}

pub fn display_bounds(page: &PdfPageState, bounds: RectD) -> RectD {
    let a = to_display(page, PointD::new(bounds.x, bounds.y));
    let b = to_display(page, PointD::new(bounds.right(), bounds.bottom()));
    RectD::between(a, b)
}

pub fn page_bounds(page: &PdfPageState, display: RectD) -> RectD {
    let a = to_page(page, PointD::new(display.x, display.y));
    let b = to_page(page, PointD::new(display.right(), display.bottom()));
    RectD::between(a, b)
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum PageLayoutMode {
    #[default]
    Continuous,
    SinglePage,
    TwoPage,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PagePlacement {
    pub index: usize,
    /// Screen bounds of the displayed (cropped, rotated) page.
    pub bounds: RectD,
}

impl PagePlacement {
    pub fn to_page(&self, page: &PdfPageState, screen: PointD, zoom: f64) -> PointD {
        to_page(page, PointD::new((screen.x - self.bounds.x) / zoom, (screen.y - self.bounds.y) / zoom))
    }

    pub fn to_screen(&self, page: &PdfPageState, point: PointD, zoom: f64) -> PointD {
        let p = to_display(page, point);
        PointD::new(self.bounds.x + p.x * zoom, self.bounds.y + p.y * zoom)
    }

    pub fn rect_to_screen(&self, page: &PdfPageState, rect: RectD, zoom: f64) -> RectD {
        let d = display_bounds(page, rect);
        RectD::new(self.bounds.x + d.x * zoom, self.bounds.y + d.y * zoom, d.width * zoom, d.height * zoom)
    }
}

pub const GAP: f64 = 20.0;
pub const MIN_ZOOM: f64 = 0.1;
pub const MAX_ZOOM: f64 = 8.0;

pub fn clamp_zoom(zoom: f64) -> f64 {
    zoom.clamp(MIN_ZOOM, MAX_ZOOM)
}

/// Places pages for a viewport. `scroll` and `pan` are screen offsets.
pub fn arrange(
    pages: &[PdfPageState],
    viewport_width: f64,
    zoom: f64,
    scroll: f64,
    pan: f64,
    mode: PageLayoutMode,
    current_page: usize,
) -> Vec<PagePlacement> {
    let zoom = clamp_zoom(zoom);
    let mut result = Vec::with_capacity(pages.len());
    if pages.is_empty() {
        return result;
    }
    let mut y = GAP - scroll;
    let centered = |width: f64| ((viewport_width - width) / 2.0).max(GAP) + pan;
    if mode == PageLayoutMode::SinglePage {
        let i = current_page.min(pages.len() - 1);
        let page = &pages[i];
        let (w, h) = (page.display_width() * zoom, page.display_height() * zoom);
        result.push(PagePlacement { index: i, bounds: RectD::new(centered(w), y, w, h) });
        return result;
    }
    let mut i = 0;
    while i < pages.len() {
        let page = &pages[i];
        let (w, h) = (page.display_width() * zoom, page.display_height() * zoom);
        if mode == PageLayoutMode::TwoPage && i + 1 < pages.len() {
            let next = &pages[i + 1];
            let (nw, nh) = (next.display_width() * zoom, next.display_height() * zoom);
            let x = centered(w + nw + GAP);
            result.push(PagePlacement { index: i, bounds: RectD::new(x, y, w, h) });
            result.push(PagePlacement { index: i + 1, bounds: RectD::new(x + w + GAP, y, nw, nh) });
            y += h.max(nh) + GAP;
            i += 2;
        } else {
            result.push(PagePlacement { index: i, bounds: RectD::new(centered(w), y, w, h) });
            y += h + GAP;
            i += 1;
        }
    }
    result
}

/// Total scrollable content size (width, height) for the given layout at zero scroll/pan.
pub fn content_size(pages: &[PdfPageState], viewport_width: f64, zoom: f64, mode: PageLayoutMode, current_page: usize) -> (f64, f64) {
    let placements = arrange(pages, viewport_width, zoom, 0.0, 0.0, mode, current_page);
    let right = placements.iter().map(|p| p.bounds.right()).fold(0.0, f64::max) + GAP;
    let bottom = placements.iter().map(|p| p.bounds.bottom()).fold(0.0, f64::max) + GAP;
    (right, bottom)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn transforms_round_trip_for_every_rotation_and_crop() {
        for rotation in [0, 90, 180, 270] {
            for crop in [None, Some(RectD::new(20.0, 30.0, 300.0, 400.0))] {
                let page = PdfPageState { rotation, crop, ..Default::default() };
                let p = PointD::new(123.0, 456.0);
                let back = to_page(&page, to_display(&page, p));
                assert!(p.distance(back) < 1e-9, "rotation {rotation} crop {crop:?}");
            }
        }
    }

    #[test]
    fn two_page_layout_pairs_pages() {
        let pages = vec![PdfPageState::default(); 3];
        let placements = arrange(&pages, 2000.0, 1.0, 0.0, 0.0, PageLayoutMode::TwoPage, 0);
        assert_eq!(placements.len(), 3);
        assert_eq!(placements[0].bounds.y, placements[1].bounds.y);
        assert!(placements[2].bounds.y > placements[0].bounds.bottom());
    }
}
