//! Annotation text as vector outlines, for pages where the text must be drawn rotated.
//!
//! GPUI shapes and paints text only upright, but it can fill arbitrary paths. On a
//! rotated page the viewer lays glyph outlines from the system Helvetica along the page's
//! x axis and maps them through the page transform, like the exported PDF does with the
//! standard Helvetica font.

use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};

use refr_core::PointD;
use ttf_parser::{Face, GlyphId, OutlineBuilder};

const HELVETICA: &str = "/System/Library/Fonts/Helvetica.ttc";

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Cmd {
    Move(PointD),
    Line(PointD),
    Quad { ctrl: PointD, to: PointD },
    Cubic { ctrl_a: PointD, ctrl_b: PointD, to: PointD },
    Close,
}

impl Cmd {
    pub fn map(self, f: impl Fn(PointD) -> PointD) -> Cmd {
        match self {
            Cmd::Move(p) => Cmd::Move(f(p)),
            Cmd::Line(p) => Cmd::Line(f(p)),
            Cmd::Quad { ctrl, to } => Cmd::Quad { ctrl: f(ctrl), to: f(to) },
            Cmd::Cubic { ctrl_a, ctrl_b, to } => Cmd::Cubic { ctrl_a: f(ctrl_a), ctrl_b: f(ctrl_b), to: f(to) },
            Cmd::Close => Cmd::Close,
        }
    }
}

/// A glyph in font units (y up), with its advance.
struct Glyph {
    cmds: Vec<Cmd>,
    advance: f64,
}

struct Font {
    data: &'static [u8],
    regular: u32,
    bold: u32,
    glyphs: Mutex<HashMap<(char, bool), Option<std::sync::Arc<Glyph>>>>,
}

struct Collector(Vec<Cmd>);

impl OutlineBuilder for Collector {
    fn move_to(&mut self, x: f32, y: f32) {
        self.0.push(Cmd::Move(PointD::new(x as f64, y as f64)));
    }
    fn line_to(&mut self, x: f32, y: f32) {
        self.0.push(Cmd::Line(PointD::new(x as f64, y as f64)));
    }
    fn quad_to(&mut self, x1: f32, y1: f32, x: f32, y: f32) {
        self.0.push(Cmd::Quad { ctrl: PointD::new(x1 as f64, y1 as f64), to: PointD::new(x as f64, y as f64) });
    }
    fn curve_to(&mut self, x1: f32, y1: f32, x2: f32, y2: f32, x: f32, y: f32) {
        self.0.push(Cmd::Cubic { ctrl_a: PointD::new(x1 as f64, y1 as f64), ctrl_b: PointD::new(x2 as f64, y2 as f64), to: PointD::new(x as f64, y as f64) });
    }
    fn close(&mut self) {
        self.0.push(Cmd::Close);
    }
}

fn font() -> Option<&'static Font> {
    static FONT: OnceLock<Option<Font>> = OnceLock::new();
    FONT.get_or_init(|| {
        let data: &'static [u8] = Box::leak(std::fs::read(HELVETICA).ok()?.into_boxed_slice());
        let count = ttf_parser::fonts_in_collection(data).unwrap_or(1);
        let mut regular = None;
        let mut bold = None;
        for index in 0..count {
            let Ok(face) = Face::parse(data, index) else { continue };
            // Pick by weight and style: the name table's English names are Mac-only here.
            if face.is_italic() || face.is_oblique() {
                continue;
            }
            match face.weight() {
                ttf_parser::Weight::Normal if regular.is_none() => regular = Some(index),
                ttf_parser::Weight::Bold if bold.is_none() => bold = Some(index),
                _ => {}
            }
        }
        let regular = regular.unwrap_or(0);
        Some(Font { data, regular, bold: bold.unwrap_or(regular), glyphs: Mutex::new(HashMap::new()) })
    })
    .as_ref()
}

impl Font {
    fn glyph(&self, c: char, bold: bool) -> Option<std::sync::Arc<Glyph>> {
        let mut cache = self.glyphs.lock().ok()?;
        cache
            .entry((c, bold))
            .or_insert_with(|| {
                let face = Face::parse(self.data, if bold { self.bold } else { self.regular }).ok()?;
                let id = face.glyph_index(c).unwrap_or(GlyphId(0));
                let scale = 1.0 / face.units_per_em() as f64;
                let mut collector = Collector(Vec::new());
                face.outline_glyph(id, &mut collector);
                let cmds = collector.0.into_iter().map(|cmd| cmd.map(|p| PointD::new(p.x * scale, p.y * scale))).collect();
                let advance = face.glyph_hor_advance(id).unwrap_or(0) as f64 * scale;
                Some(std::sync::Arc::new(Glyph { cmds, advance }))
            })
            .clone()
    }
}

/// Outlines for one line of text in page coordinates: the pen starts at `origin` on the
/// baseline and advances along +x; glyphs extend towards −y (up the page).
/// Returns `None` when the system Helvetica is unavailable.
pub fn line(text: &str, origin: PointD, size: f64, bold: bool) -> Option<Vec<Cmd>> {
    let font = font()?;
    let mut cmds = Vec::new();
    let mut pen = origin.x;
    for c in text.chars() {
        let Some(glyph) = font.glyph(c, bold) else { continue };
        let x = pen;
        cmds.extend(glyph.cmds.iter().map(|cmd| cmd.map(|p| PointD::new(x + p.x * size, origin.y - p.y * size))));
        pen += glyph.advance * size;
    }
    Some(cmds)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn helvetica_outlines_follow_the_pen() {
        let Some(cmds) = line("Hi", PointD::new(100.0, 200.0), 10.0, false) else { return };
        let points: Vec<PointD> = cmds
            .iter()
            .filter_map(|c| match c {
                Cmd::Move(p) | Cmd::Line(p) => Some(*p),
                _ => None,
            })
            .collect();
        assert!(!points.is_empty());
        // Cap height sits above the baseline (smaller y), and nothing starts left of the pen.
        assert!(points.iter().all(|p| p.x >= 99.0 && p.y <= 200.5 && p.y >= 191.0), "{points:?}");
        let bold = line("Hi", PointD::new(100.0, 200.0), 10.0, true).unwrap();
        assert_ne!(bold, cmds, "bold uses a different face");
    }
}
