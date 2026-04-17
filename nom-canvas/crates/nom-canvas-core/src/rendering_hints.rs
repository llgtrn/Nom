//! Per-element rendering hints: non-structural visual cues the renderer paints
//! on top of or behind the element (selection outline, hover glow, snap guide).
#![deny(unsafe_code)]

use nom_gpui::{Bounds, Pixels, Point};
#[cfg(test)]
use nom_gpui::Size;

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum HintLayer {
    Below,
    Above,
}

#[derive(Clone, Debug, PartialEq)]
pub enum RenderingHint {
    /// Filled outline around selected elements.
    SelectionOutline { bounds: Bounds<Pixels>, color_argb: u32, stroke_px: f32 },
    /// Soft glow on hover.
    HoverGlow { bounds: Bounds<Pixels>, color_argb: u32, radius_px: f32 },
    /// Dotted or dashed guide line from snapping.rs.
    SnapGuide { from: Point<Pixels>, to: Point<Pixels>, color_argb: u32, dash_px: f32 },
    /// 8 resize handles + 1 rotation handle.
    TransformHandle { at: Point<Pixels>, kind_tag: u8, color_argb: u32, size_px: f32 },
    /// Marquee rubber-band rectangle while dragging.
    MarqueeOutline { bounds: Bounds<Pixels>, fill_argb: u32, stroke_argb: u32, stroke_px: f32 },
    /// Inline error/warning badge over an element (e.g. compile error).
    Badge { at: Point<Pixels>, glyph: char, color_argb: u32, size_px: f32 },
}

impl RenderingHint {
    pub fn layer(&self) -> HintLayer {
        // Selection + handles + badges paint ABOVE the element.  Hover glow +
        // snap guides + marquee paint BELOW so the element stays legible.
        match self {
            Self::SelectionOutline { .. } => HintLayer::Above,
            Self::TransformHandle { .. } => HintLayer::Above,
            Self::Badge { .. } => HintLayer::Above,
            Self::HoverGlow { .. } => HintLayer::Below,
            Self::SnapGuide { .. } => HintLayer::Below,
            Self::MarqueeOutline { .. } => HintLayer::Below,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Default)]
pub struct HintQueue {
    hints: Vec<RenderingHint>,
}

impl HintQueue {
    pub fn new() -> Self {
        Self::default()
    }
    pub fn push(&mut self, hint: RenderingHint) {
        self.hints.push(hint);
    }
    pub fn clear(&mut self) {
        self.hints.clear();
    }
    pub fn len(&self) -> usize {
        self.hints.len()
    }
    pub fn is_empty(&self) -> bool {
        self.hints.is_empty()
    }
    pub fn below(&self) -> Vec<&RenderingHint> {
        self.hints.iter().filter(|h| h.layer() == HintLayer::Below).collect()
    }
    pub fn above(&self) -> Vec<&RenderingHint> {
        self.hints.iter().filter(|h| h.layer() == HintLayer::Above).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_bounds() -> Bounds<Pixels> {
        Bounds::new(
            Point { x: Pixels(0.0), y: Pixels(0.0) },
            Size { width: Pixels(10.0), height: Pixels(10.0) },
        )
    }

    fn make_point() -> Point<Pixels> {
        Point { x: Pixels(5.0), y: Pixels(5.0) }
    }

    // --- layer routing ---

    #[test]
    fn selection_outline_is_above() {
        let h = RenderingHint::SelectionOutline {
            bounds: make_bounds(),
            color_argb: 0xFF0000FF,
            stroke_px: 2.0,
        };
        assert_eq!(h.layer(), HintLayer::Above);
    }

    #[test]
    fn hover_glow_is_below() {
        let h = RenderingHint::HoverGlow {
            bounds: make_bounds(),
            color_argb: 0x40FFFFFF,
            radius_px: 8.0,
        };
        assert_eq!(h.layer(), HintLayer::Below);
    }

    #[test]
    fn snap_guide_is_below() {
        let h = RenderingHint::SnapGuide {
            from: make_point(),
            to: make_point(),
            color_argb: 0xFF00FF00,
            dash_px: 4.0,
        };
        assert_eq!(h.layer(), HintLayer::Below);
    }

    #[test]
    fn transform_handle_is_above() {
        let h = RenderingHint::TransformHandle {
            at: make_point(),
            kind_tag: 0,
            color_argb: 0xFF0000FF,
            size_px: 6.0,
        };
        assert_eq!(h.layer(), HintLayer::Above);
    }

    #[test]
    fn marquee_outline_is_below() {
        let h = RenderingHint::MarqueeOutline {
            bounds: make_bounds(),
            fill_argb: 0x20FFFFFF,
            stroke_argb: 0xFF0000FF,
            stroke_px: 1.0,
        };
        assert_eq!(h.layer(), HintLayer::Below);
    }

    #[test]
    fn badge_is_above() {
        let h = RenderingHint::Badge {
            at: make_point(),
            glyph: '!',
            color_argb: 0xFFFF0000,
            size_px: 12.0,
        };
        assert_eq!(h.layer(), HintLayer::Above);
    }

    // --- HintQueue behaviour ---

    #[test]
    fn new_queue_is_empty() {
        let q = HintQueue::new();
        assert!(q.is_empty());
        assert_eq!(q.len(), 0);
    }

    #[test]
    fn push_increments_len() {
        let mut q = HintQueue::new();
        q.push(RenderingHint::Badge {
            at: make_point(),
            glyph: 'E',
            color_argb: 0xFFFF0000,
            size_px: 10.0,
        });
        assert_eq!(q.len(), 1);
        assert!(!q.is_empty());
    }

    #[test]
    fn clear_empties_queue() {
        let mut q = HintQueue::new();
        q.push(RenderingHint::Badge {
            at: make_point(),
            glyph: 'E',
            color_argb: 0xFFFF0000,
            size_px: 10.0,
        });
        q.clear();
        assert!(q.is_empty());
    }

    #[test]
    fn below_above_filter_mixed_queue() {
        let mut q = HintQueue::new();
        // 3 Above
        q.push(RenderingHint::SelectionOutline {
            bounds: make_bounds(),
            color_argb: 0xFF0000FF,
            stroke_px: 2.0,
        });
        q.push(RenderingHint::TransformHandle {
            at: make_point(),
            kind_tag: 1,
            color_argb: 0xFF0000FF,
            size_px: 6.0,
        });
        q.push(RenderingHint::Badge {
            at: make_point(),
            glyph: '!',
            color_argb: 0xFFFF0000,
            size_px: 12.0,
        });
        // 3 Below
        q.push(RenderingHint::HoverGlow {
            bounds: make_bounds(),
            color_argb: 0x40FFFFFF,
            radius_px: 8.0,
        });
        q.push(RenderingHint::SnapGuide {
            from: make_point(),
            to: make_point(),
            color_argb: 0xFF00FF00,
            dash_px: 4.0,
        });
        q.push(RenderingHint::MarqueeOutline {
            bounds: make_bounds(),
            fill_argb: 0x20FFFFFF,
            stroke_argb: 0xFF0000FF,
            stroke_px: 1.0,
        });

        assert_eq!(q.len(), 6);
        assert_eq!(q.above().len(), 3);
        assert_eq!(q.below().len(), 3);
    }

    #[test]
    fn all_variants_construct() {
        let b = make_bounds();
        let p = make_point();
        let _ = RenderingHint::SelectionOutline { bounds: b.clone(), color_argb: 0, stroke_px: 1.0 };
        let _ = RenderingHint::HoverGlow { bounds: b.clone(), color_argb: 0, radius_px: 4.0 };
        let _ = RenderingHint::SnapGuide { from: p, to: p, color_argb: 0, dash_px: 2.0 };
        let _ = RenderingHint::TransformHandle { at: p, kind_tag: 0, color_argb: 0, size_px: 6.0 };
        let _ = RenderingHint::MarqueeOutline { bounds: b, fill_argb: 0, stroke_argb: 0, stroke_px: 1.0 };
        let _ = RenderingHint::Badge { at: p, glyph: 'X', color_argb: 0, size_px: 10.0 };
    }
}
