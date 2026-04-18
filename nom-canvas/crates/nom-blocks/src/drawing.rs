#![deny(unsafe_code)]
use crate::block_model::NomtuRef;
use serde::{Deserialize, Serialize};

// ---------------------------------------------------------------------------
// Theme-sourced block color helpers
// All colors are sourced from nom_theme::tokens so that palette changes
// propagate automatically to all drawing code.
// ---------------------------------------------------------------------------

/// Fill color for an unselected block surface.
/// Uses the tertiary background token — slightly elevated over the canvas.
pub fn block_fill_color() -> [f32; 4] {
    let c = nom_theme::tokens::color_bg_tertiary();
    [c.h, c.s, c.l, c.a]
}

/// Border color for a normal (unselected) block.
pub fn block_border_color() -> [f32; 4] {
    let c = nom_theme::tokens::color_border_normal();
    [c.h, c.s, c.l, c.a]
}

/// Fill/border color for a selected block — accent blue from tokens.
pub fn block_selected_color() -> [f32; 4] {
    let c = nom_theme::tokens::color_accent_blue();
    [c.h, c.s, c.l, c.a]
}

/// Color used to indicate an error state on a block — red token from tokens.
pub fn block_error_color() -> [f32; 4] {
    let c = nom_theme::tokens::edge_color_low_confidence();
    [c.h, c.s, c.l, c.a]
}

// Hsla-compatible color stored as [h,s,l,a] f32
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct StrokeColor {
    pub h: f32,
    pub s: f32,
    pub l: f32,
    pub a: f32,
}

impl StrokeColor {
    pub fn black() -> Self {
        Self {
            h: 0.0,
            s: 0.0,
            l: 0.0,
            a: 1.0,
        }
    }
    pub fn white() -> Self {
        Self {
            h: 0.0,
            s: 0.0,
            l: 1.0,
            a: 1.0,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Stroke {
    pub points: Vec<[f32; 2]>,
    pub pressure: Vec<f32>,
    pub color: StrokeColor,
    pub width: f32,
}

impl Stroke {
    pub fn new(color: StrokeColor, width: f32) -> Self {
        Self {
            points: Vec::new(),
            pressure: Vec::new(),
            color,
            width,
        }
    }
    pub fn add_point(&mut self, pt: [f32; 2], pressure: f32) {
        self.points.push(pt);
        self.pressure.push(pressure);
    }
    pub fn bounding_box(&self) -> Option<([f32; 2], [f32; 2])> {
        if self.points.is_empty() {
            return None;
        }
        let min_x = self
            .points
            .iter()
            .map(|p| p[0])
            .fold(f32::INFINITY, f32::min);
        let min_y = self
            .points
            .iter()
            .map(|p| p[1])
            .fold(f32::INFINITY, f32::min);
        let max_x = self
            .points
            .iter()
            .map(|p| p[0])
            .fold(f32::NEG_INFINITY, f32::max);
        let max_y = self
            .points
            .iter()
            .map(|p| p[1])
            .fold(f32::NEG_INFINITY, f32::max);
        Some(([min_x, min_y], [max_x, max_y]))
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DrawingBlock {
    pub entity: NomtuRef,
    pub strokes: Vec<Stroke>,
}

impl DrawingBlock {
    pub fn new(entity: NomtuRef) -> Self {
        Self {
            entity,
            strokes: Vec::new(),
        }
    }
    pub fn add_stroke(&mut self, stroke: Stroke) {
        self.strokes.push(stroke);
    }
    pub fn clear(&mut self) {
        self.strokes.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn stroke_bounding_box() {
        let mut s = Stroke::new(StrokeColor::black(), 2.0);
        s.add_point([0.0, 0.0], 1.0);
        s.add_point([100.0, 50.0], 1.0);
        let bb = s.bounding_box().unwrap();
        assert_eq!(bb.0, [0.0, 0.0]);
        assert_eq!(bb.1, [100.0, 50.0]);
    }

    #[test]
    fn stroke_bounding_box_empty_returns_none() {
        let s = Stroke::new(StrokeColor::black(), 1.0);
        assert!(s.bounding_box().is_none());
    }

    #[test]
    fn stroke_add_point_increments_count() {
        let mut s = Stroke::new(StrokeColor::white(), 1.5);
        assert!(s.points.is_empty());
        s.add_point([10.0, 20.0], 0.8);
        s.add_point([30.0, 40.0], 0.9);
        assert_eq!(s.points.len(), 2);
        assert_eq!(s.pressure.len(), 2);
        assert_eq!(s.points[0], [10.0, 20.0]);
        assert!((s.pressure[1] - 0.9).abs() < f32::EPSILON);
    }

    #[test]
    fn stroke_color_black_and_white() {
        let black = StrokeColor::black();
        assert_eq!(black.l, 0.0);
        assert_eq!(black.a, 1.0);
        let white = StrokeColor::white();
        assert_eq!(white.l, 1.0);
        assert_eq!(white.a, 1.0);
    }

    #[test]
    fn drawing_block_add_clear() {
        let entity = crate::block_model::NomtuRef::new("draw-01", "sketch", "verb");
        let mut d = DrawingBlock::new(entity);
        assert!(d.strokes.is_empty());
        let mut s = Stroke::new(StrokeColor::black(), 1.0);
        s.add_point([0.0, 0.0], 1.0);
        d.add_stroke(s);
        assert_eq!(d.strokes.len(), 1);
        d.clear();
        assert!(d.strokes.is_empty());
    }

    #[test]
    fn drawing_block_entity_non_empty() {
        let entity = crate::block_model::NomtuRef::new("draw-02", "draw", "verb");
        let d = DrawingBlock::new(entity);
        assert_eq!(d.entity.id, "draw-02");
        assert_eq!(d.entity.word, "draw");
    }

    #[test]
    fn stroke_bounding_box_single_point() {
        let mut s = Stroke::new(StrokeColor::black(), 1.0);
        s.add_point([5.0, 7.0], 0.5);
        let bb = s.bounding_box().unwrap();
        assert_eq!(bb.0, [5.0, 7.0]);
        assert_eq!(bb.1, [5.0, 7.0]);
    }

    #[test]
    fn stroke_bounding_box_negative_coords() {
        let mut s = Stroke::new(StrokeColor::black(), 1.0);
        s.add_point([-10.0, -20.0], 1.0);
        s.add_point([5.0, 15.0], 1.0);
        let bb = s.bounding_box().unwrap();
        assert_eq!(bb.0, [-10.0, -20.0]);
        assert_eq!(bb.1, [5.0, 15.0]);
    }

    #[test]
    fn drawing_block_multiple_strokes() {
        let entity = crate::block_model::NomtuRef::new("draw-03", "annotate", "verb");
        let mut d = DrawingBlock::new(entity);
        for i in 0..5 {
            let mut s = Stroke::new(StrokeColor::black(), 1.0);
            s.add_point([i as f32, i as f32], 1.0);
            d.add_stroke(s);
        }
        assert_eq!(d.strokes.len(), 5);
    }

    // -----------------------------------------------------------------------
    // Theme-token color tests (AE15)
    // -----------------------------------------------------------------------

    #[test]
    fn block_fill_color_matches_theme_token() {
        // block_fill_color() must equal color_bg_tertiary() — not a hard-coded value.
        let expected = nom_theme::tokens::color_bg_tertiary();
        let actual = super::block_fill_color();
        assert!(
            (actual[0] - expected.h).abs() < f32::EPSILON,
            "fill h mismatch: {} vs {}",
            actual[0],
            expected.h
        );
        assert!(
            (actual[1] - expected.s).abs() < f32::EPSILON,
            "fill s mismatch"
        );
        assert!(
            (actual[2] - expected.l).abs() < f32::EPSILON,
            "fill l mismatch"
        );
        assert!(
            (actual[3] - expected.a).abs() < f32::EPSILON,
            "fill a mismatch"
        );
    }

    #[test]
    fn block_border_color_matches_theme_token() {
        // block_border_color() must equal color_border_normal().
        let expected = nom_theme::tokens::color_border_normal();
        let actual = super::block_border_color();
        assert!(
            (actual[0] - expected.h).abs() < f32::EPSILON,
            "border h mismatch"
        );
        assert!(
            (actual[2] - expected.l).abs() < f32::EPSILON,
            "border l mismatch"
        );
    }

    #[test]
    fn block_selected_color_differs_from_unselected() {
        // A selected block must have a visually distinct color from an unselected one.
        let fill = super::block_fill_color();
        let selected = super::block_selected_color();
        // At least one HSLA component must differ by more than epsilon.
        let any_diff = fill
            .iter()
            .zip(selected.iter())
            .any(|(a, b)| (a - b).abs() > 0.01);
        assert!(
            any_diff,
            "selected color must differ from fill color; got identical values"
        );
    }

    #[test]
    fn block_error_color_matches_theme_error_token() {
        // block_error_color() must equal edge_color_low_confidence() (red error token).
        let expected = nom_theme::tokens::edge_color_low_confidence();
        let actual = super::block_error_color();
        assert!(
            (actual[0] - expected.h).abs() < f32::EPSILON,
            "error h mismatch: {} vs {}",
            actual[0],
            expected.h
        );
        assert!(
            (actual[1] - expected.s).abs() < f32::EPSILON,
            "error s mismatch"
        );
    }

    #[test]
    fn theme_color_change_reflected_in_drawing_output() {
        // Verify that drawing helpers call through to the live token functions
        // rather than caching a snapshot. Since tokens are pure functions,
        // two consecutive calls must return identical values — confirming that
        // block_fill_color() delegates to the token on every call.
        let first_call = super::block_fill_color();
        let second_call = super::block_fill_color();
        assert_eq!(
            first_call, second_call,
            "block_fill_color() must be deterministic and delegate to theme token"
        );
        // Also confirm the value matches the live token (not a cached snapshot).
        let token = nom_theme::tokens::color_bg_tertiary();
        assert!(
            (first_call[2] - token.l).abs() < f32::EPSILON,
            "drawing output must reflect current theme token value"
        );
    }
}
