//! Freehand drawing block schema (strokes over a surface).
//!
//! Color representation: sRGB 8-bit per channel color (`SrgbColor`).
#![deny(unsafe_code)]

use crate::block_model::FractionalIndex;
use crate::block_schema::{BlockSchema, Role};
use crate::flavour::{DRAWING, SURFACE};

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SrgbColor {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub a: u8,
}

impl SrgbColor {
    pub const BLACK: SrgbColor = SrgbColor { r: 0, g: 0, b: 0, a: 255 };
    pub const RED: SrgbColor = SrgbColor { r: 255, g: 0, b: 0, a: 255 };

    pub fn new(r: u8, g: u8, b: u8, a: u8) -> Self {
        SrgbColor { r, g, b, a }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PressurePoint {
    pub x: f32,
    pub y: f32,
    /// Normalised stylus/touch pressure in 0.0..=1.0.
    pub pressure: f32,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Stroke {
    pub points: Vec<PressurePoint>,
    pub color: SrgbColor,
    pub width: f32,
}

impl Stroke {
    pub fn new(color: SrgbColor, width: f32) -> Self {
        Stroke { points: Vec::new(), color, width }
    }

    pub fn add_point(&mut self, x: f32, y: f32, pressure: f32) {
        let pressure = pressure.clamp(0.0, 1.0);
        self.points.push(PressurePoint { x, y, pressure });
    }

    pub fn len(&self) -> usize {
        self.points.len()
    }

    pub fn is_empty(&self) -> bool {
        self.points.is_empty()
    }

    /// Returns `(min_x, min_y, max_x, max_y)` or `None` when the stroke is empty.
    pub fn bounding_box(&self) -> Option<(f32, f32, f32, f32)> {
        let mut iter = self.points.iter();
        let first = iter.next()?;
        let (mut min_x, mut min_y, mut max_x, mut max_y) =
            (first.x, first.y, first.x, first.y);
        for p in iter {
            if p.x < min_x { min_x = p.x; }
            if p.y < min_y { min_y = p.y; }
            if p.x > max_x { max_x = p.x; }
            if p.y > max_y { max_y = p.y; }
        }
        Some((min_x, min_y, max_x, max_y))
    }
}

// ---------------------------------------------------------------------------
// Ramer-Douglas-Peucker line simplification
// ---------------------------------------------------------------------------

/// Perpendicular distance from point `p` to the line defined by `a`–`b`.
fn point_to_line_distance(p: &PressurePoint, a: &PressurePoint, b: &PressurePoint) -> f32 {
    let dx = b.x - a.x;
    let dy = b.y - a.y;
    let len_sq = dx * dx + dy * dy;
    if len_sq == 0.0 {
        // Degenerate segment: a == b; distance is point-to-point.
        let ex = p.x - a.x;
        let ey = p.y - a.y;
        return (ex * ex + ey * ey).sqrt();
    }
    // |cross(ab, ap)| / |ab|
    let cross = (p.x - a.x) * dy - (p.y - a.y) * dx;
    cross.abs() / len_sq.sqrt()
}

fn rdp(points: &[PressurePoint], epsilon: f32, out: &mut Vec<PressurePoint>) {
    if points.len() < 2 {
        if let Some(p) = points.first() {
            out.push(*p);
        }
        return;
    }

    let first = &points[0];
    let last = &points[points.len() - 1];

    // Find the point with maximum perpendicular distance.
    let mut max_dist = 0.0_f32;
    let mut max_idx = 0;
    for (i, p) in points[1..points.len() - 1].iter().enumerate() {
        let d = point_to_line_distance(p, first, last);
        if d > max_dist {
            max_dist = d;
            max_idx = i + 1; // adjust for the slice offset
        }
    }

    if max_dist > epsilon {
        // Recurse on each sub-segment; avoid duplicating the split point.
        rdp(&points[..=max_idx], epsilon, out);
        out.pop(); // remove the duplicated split point
        rdp(&points[max_idx..], epsilon, out);
    } else {
        // All intermediate points are within tolerance; keep only endpoints.
        out.push(*first);
        out.push(*last);
    }
}

/// Ramer-Douglas-Peucker simplification of a stroke.
///
/// Returns a new `Stroke` whose points form a polyline whose maximum deviation
/// from the original polyline is less than `epsilon` pixels.  The start and
/// end points are always preserved.  Colour and width are copied unchanged.
pub fn simplify_stroke(stroke: &Stroke, epsilon: f32) -> Stroke {
    let mut simplified = Stroke::new(stroke.color, stroke.width);
    if stroke.points.len() < 2 {
        simplified.points = stroke.points.clone();
        return simplified;
    }
    rdp(&stroke.points, epsilon, &mut simplified.points);
    simplified
}

// ---------------------------------------------------------------------------
// DrawingProps
// ---------------------------------------------------------------------------

#[derive(Clone, Debug, PartialEq)]
pub struct DrawingProps {
    pub xywh: String,
    pub strokes: Vec<Stroke>,
    pub index: FractionalIndex,
}

impl DrawingProps {
    pub fn new() -> Self {
        DrawingProps {
            xywh: "0 0 500 500".to_owned(),
            strokes: Vec::new(),
            index: "a0".to_owned(),
        }
    }

    pub fn push_stroke(&mut self, stroke: Stroke) {
        self.strokes.push(stroke);
    }

    pub fn stroke_count(&self) -> usize {
        self.strokes.len()
    }

    pub fn total_points(&self) -> usize {
        self.strokes.iter().map(|s| s.len()).sum()
    }
}

impl Default for DrawingProps {
    fn default() -> Self {
        Self::new()
    }
}

// ---------------------------------------------------------------------------
// Schema
// ---------------------------------------------------------------------------

pub fn drawing_schema() -> BlockSchema {
    BlockSchema {
        flavour: DRAWING,
        version: 1,
        role: Role::Content,
        parents: &[SURFACE],
        children: &[],
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stroke_new_is_empty() {
        let s = Stroke::new(SrgbColor::BLACK, 2.0);
        assert!(s.is_empty());
        assert_eq!(s.len(), 0);
    }

    #[test]
    fn add_point_clamps_pressure_below_zero() {
        let mut s = Stroke::new(SrgbColor::BLACK, 1.0);
        s.add_point(0.0, 0.0, -5.0);
        assert_eq!(s.points[0].pressure, 0.0);
    }

    #[test]
    fn add_point_clamps_pressure_above_one() {
        let mut s = Stroke::new(SrgbColor::BLACK, 1.0);
        s.add_point(0.0, 0.0, 99.0);
        assert_eq!(s.points[0].pressure, 1.0);
    }

    #[test]
    fn add_point_keeps_valid_pressure() {
        let mut s = Stroke::new(SrgbColor::BLACK, 1.0);
        s.add_point(1.0, 2.0, 0.7);
        assert!((s.points[0].pressure - 0.7).abs() < f32::EPSILON);
    }

    #[test]
    fn bounding_box_empty_returns_none() {
        let s = Stroke::new(SrgbColor::BLACK, 1.0);
        assert!(s.bounding_box().is_none());
    }

    #[test]
    fn bounding_box_single_point() {
        let mut s = Stroke::new(SrgbColor::BLACK, 1.0);
        s.add_point(3.0, 7.0, 0.5);
        assert_eq!(s.bounding_box(), Some((3.0, 7.0, 3.0, 7.0)));
    }

    #[test]
    fn bounding_box_multiple_points() {
        let mut s = Stroke::new(SrgbColor::BLACK, 1.0);
        s.add_point(1.0, 5.0, 0.5);
        s.add_point(10.0, 2.0, 0.5);
        s.add_point(4.0, 8.0, 0.5);
        assert_eq!(s.bounding_box(), Some((1.0, 2.0, 10.0, 8.0)));
    }

    #[test]
    fn simplify_preserves_start_and_end() {
        let mut s = Stroke::new(SrgbColor::RED, 1.0);
        for i in 0..20 {
            let t = i as f32;
            s.add_point(t, t.sin() * 0.01, 0.5); // nearly collinear
        }
        let simplified = simplify_stroke(&s, 5.0);
        assert!(!simplified.is_empty());
        let first_orig = s.points.first().unwrap();
        let last_orig = s.points.last().unwrap();
        let first_simp = simplified.points.first().unwrap();
        let last_simp = simplified.points.last().unwrap();
        assert_eq!(first_simp.x, first_orig.x);
        assert_eq!(first_simp.y, first_orig.y);
        assert_eq!(last_simp.x, last_orig.x);
        assert_eq!(last_simp.y, last_orig.y);
    }

    #[test]
    fn simplify_large_epsilon_reduces_points() {
        // Build a stroke that is nearly a straight line with a small bump.
        let mut s = Stroke::new(SrgbColor::BLACK, 1.0);
        for i in 0..=100 {
            let x = i as f32;
            let y = if i == 50 { 0.5 } else { 0.0 }; // tiny bump at midpoint
        s.add_point(x, y, 0.5);
        }
        let simplified = simplify_stroke(&s, 1.0); // epsilon > bump height
        // With the bump smaller than epsilon the result should be just 2 pts.
        assert_eq!(simplified.len(), 2);
        assert!(simplified.len() < s.len());
    }

    #[test]
    fn drawing_props_new_defaults() {
        let d = DrawingProps::new();
        assert_eq!(d.xywh, "0 0 500 500");
        assert_eq!(d.index, "a0");
        assert_eq!(d.stroke_count(), 0);
        assert_eq!(d.total_points(), 0);
    }

    #[test]
    fn total_points_sums_across_strokes() {
        let mut d = DrawingProps::new();
        let mut s1 = Stroke::new(SrgbColor::BLACK, 1.0);
        s1.add_point(0.0, 0.0, 0.5);
        s1.add_point(1.0, 1.0, 0.5);
        let mut s2 = Stroke::new(SrgbColor::RED, 2.0);
        s2.add_point(2.0, 2.0, 0.5);
        d.push_stroke(s1);
        d.push_stroke(s2);
        assert_eq!(d.stroke_count(), 2);
        assert_eq!(d.total_points(), 3);
    }

    #[test]
    fn drawing_schema_fields() {
        let schema = drawing_schema();
        assert_eq!(schema.flavour, DRAWING);
        assert_eq!(schema.version, 1);
        assert_eq!(schema.role, Role::Content);
        assert_eq!(schema.parents, &[SURFACE]);
        assert!(schema.children.is_empty());
    }
}
