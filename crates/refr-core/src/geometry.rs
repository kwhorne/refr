use std::ops::{Add, Sub};

use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct PointD {
    pub x: f64,
    pub y: f64,
}

impl PointD {
    pub const fn new(x: f64, y: f64) -> Self {
        Self { x, y }
    }

    pub fn distance(self, other: PointD) -> f64 {
        ((self.x - other.x).powi(2) + (self.y - other.y).powi(2)).sqrt()
    }

    pub fn is_finite(self) -> bool {
        self.x.is_finite() && self.y.is_finite()
    }
}

impl Add for PointD {
    type Output = PointD;
    fn add(self, rhs: PointD) -> PointD {
        PointD::new(self.x + rhs.x, self.y + rhs.y)
    }
}

impl Sub for PointD {
    type Output = PointD;
    fn sub(self, rhs: PointD) -> PointD {
        PointD::new(self.x - rhs.x, self.y - rhs.y)
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct RectD {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}

impl RectD {
    pub const fn new(x: f64, y: f64, width: f64, height: f64) -> Self {
        Self { x, y, width, height }
    }

    pub fn right(&self) -> f64 {
        self.x + self.width
    }

    pub fn bottom(&self) -> f64 {
        self.y + self.height
    }

    pub fn center(&self) -> PointD {
        PointD::new(self.x + self.width / 2.0, self.y + self.height / 2.0)
    }

    pub fn is_finite(&self) -> bool {
        self.x.is_finite() && self.y.is_finite() && self.width.is_finite() && self.height.is_finite()
    }

    pub fn contains(&self, p: PointD) -> bool {
        p.x >= self.x && p.x <= self.right() && p.y >= self.y && p.y <= self.bottom()
    }

    pub fn intersects(&self, r: &RectD) -> bool {
        self.right() >= r.x && r.right() >= self.x && self.bottom() >= r.y && r.bottom() >= self.y
    }

    pub fn inflate(&self, value: f64) -> RectD {
        RectD::new(self.x - value, self.y - value, self.width + value * 2.0, self.height + value * 2.0)
    }

    pub fn translate(&self, delta: PointD) -> RectD {
        RectD::new(self.x + delta.x, self.y + delta.y, self.width, self.height)
    }

    pub fn between(a: PointD, b: PointD) -> RectD {
        RectD::new(a.x.min(b.x), a.y.min(b.y), (a.x - b.x).abs(), (a.y - b.y).abs())
    }

    pub fn union(a: RectD, b: RectD) -> RectD {
        let x = a.x.min(b.x);
        let y = a.y.min(b.y);
        RectD::new(x, y, a.right().max(b.right()) - x, a.bottom().max(b.bottom()) - y)
    }

    /// The overlap of two rectangles, or `None` when they do not share any area.
    pub fn intersection(a: RectD, b: RectD) -> Option<RectD> {
        let x = a.x.max(b.x);
        let y = a.y.max(b.y);
        let right = a.right().min(b.right());
        let bottom = a.bottom().min(b.bottom());
        (right > x && bottom > y).then(|| RectD::new(x, y, right - x, bottom - y))
    }

    pub fn bounding(points: &[PointD]) -> Option<RectD> {
        let first = points.first()?;
        let (mut min, mut max) = (*first, *first);
        for p in &points[1..] {
            min = PointD::new(min.x.min(p.x), min.y.min(p.y));
            max = PointD::new(max.x.max(p.x), max.y.max(p.y));
        }
        Some(RectD::between(min, max))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn union_and_intersection() {
        let a = RectD::new(0.0, 0.0, 10.0, 10.0);
        let b = RectD::new(5.0, 5.0, 10.0, 10.0);
        assert_eq!(RectD::union(a, b), RectD::new(0.0, 0.0, 15.0, 15.0));
        assert_eq!(RectD::intersection(a, b), Some(RectD::new(5.0, 5.0, 5.0, 5.0)));
        assert_eq!(RectD::intersection(a, RectD::new(20.0, 20.0, 1.0, 1.0)), None);
    }
}
