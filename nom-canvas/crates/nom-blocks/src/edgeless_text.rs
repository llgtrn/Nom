//! Edgeless text block: free-floating styled text on the canvas.
#![deny(unsafe_code)]
use crate::block_model::NomtuRef;
use serde::{Deserialize, Serialize};

/// A free-floating text block that exists outside any frame or note container.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct EdgelessTextBlock {
    /// DB entity reference (NON-OPTIONAL).
    pub entity: NomtuRef,
    /// Raw text content.
    pub content: String,
    /// Font size in logical pixels.
    pub font_size: f32,
    /// Optional CSS-style color string (e.g. `"#333333"`).
    pub color: Option<String>,
    /// Rotation angle in degrees (clockwise).
    pub rotation_deg: f32,
}

impl EdgelessTextBlock {
    /// Construct a new [`EdgelessTextBlock`] with the given entity and content.
    pub fn new(entity: NomtuRef, content: impl Into<String>) -> Self {
        Self {
            entity,
            content: content.into(),
            font_size: 16.0,
            color: None,
            rotation_deg: 0.0,
        }
    }

    /// Set the font size in logical pixels.
    pub fn set_font_size(&mut self, size: f32) {
        self.font_size = size;
    }

    /// Set the text color string.
    pub fn set_color(&mut self, color: impl Into<String>) {
        self.color = Some(color.into());
    }

    /// Add `delta` degrees to the current rotation (wraps within 0–360).
    pub fn rotate(&mut self, delta: f32) {
        self.rotation_deg = (self.rotation_deg + delta).rem_euclid(360.0);
    }

    /// Count words in `content` (whitespace-delimited, trims empty tokens).
    pub fn word_count(&self) -> usize {
        self.content
            .split_whitespace()
            .filter(|w| !w.is_empty())
            .count()
    }

    /// Return `true` when `content` is empty or whitespace only.
    pub fn is_empty(&self) -> bool {
        self.content.trim().is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entity(id: &str) -> NomtuRef {
        NomtuRef::new(id, "text", "concept")
    }

    #[test]
    fn edgeless_new_stores_content() {
        let b = EdgelessTextBlock::new(entity("e1"), "Hello world");
        assert_eq!(b.content, "Hello world");
    }

    #[test]
    fn edgeless_default_font_size() {
        let b = EdgelessTextBlock::new(entity("e2"), "");
        assert!((b.font_size - 16.0).abs() < f32::EPSILON);
    }

    #[test]
    fn edgeless_default_no_color() {
        let b = EdgelessTextBlock::new(entity("e3"), "text");
        assert!(b.color.is_none());
    }

    #[test]
    fn edgeless_default_rotation_zero() {
        let b = EdgelessTextBlock::new(entity("e4"), "text");
        assert!((b.rotation_deg).abs() < f32::EPSILON);
    }

    #[test]
    fn edgeless_entity_non_optional() {
        let b = EdgelessTextBlock::new(entity("eid-et"), "text");
        assert_eq!(b.entity.id, "eid-et");
        assert!(!b.entity.id.is_empty());
    }

    #[test]
    fn edgeless_set_font_size() {
        let mut b = EdgelessTextBlock::new(entity("e5"), "text");
        b.set_font_size(24.0);
        assert!((b.font_size - 24.0).abs() < f32::EPSILON);
    }

    #[test]
    fn edgeless_set_color() {
        let mut b = EdgelessTextBlock::new(entity("e6"), "text");
        b.set_color("#ff0000");
        assert_eq!(b.color.as_deref(), Some("#ff0000"));
    }

    #[test]
    fn edgeless_set_color_overwrites() {
        let mut b = EdgelessTextBlock::new(entity("e7"), "text");
        b.set_color("#aaa");
        b.set_color("#bbb");
        assert_eq!(b.color.as_deref(), Some("#bbb"));
    }

    #[test]
    fn edgeless_rotate_accumulates() {
        let mut b = EdgelessTextBlock::new(entity("e8"), "text");
        b.rotate(90.0);
        b.rotate(45.0);
        assert!((b.rotation_deg - 135.0).abs() < f32::EPSILON);
    }

    #[test]
    fn edgeless_rotate_wraps_at_360() {
        let mut b = EdgelessTextBlock::new(entity("e9"), "text");
        b.rotate(270.0);
        b.rotate(180.0);
        // 270 + 180 = 450 -> 90
        assert!((b.rotation_deg - 90.0).abs() < 0.001);
    }

    #[test]
    fn edgeless_word_count_basic() {
        let b = EdgelessTextBlock::new(entity("e10"), "one two three");
        assert_eq!(b.word_count(), 3);
    }

    #[test]
    fn edgeless_word_count_empty() {
        let b = EdgelessTextBlock::new(entity("e11"), "");
        assert_eq!(b.word_count(), 0);
    }

    #[test]
    fn edgeless_word_count_whitespace_only() {
        let b = EdgelessTextBlock::new(entity("e12"), "   \t\n  ");
        assert_eq!(b.word_count(), 0);
    }

    #[test]
    fn edgeless_is_empty_true_for_blank() {
        let b = EdgelessTextBlock::new(entity("e13"), "   ");
        assert!(b.is_empty());
    }

    #[test]
    fn edgeless_is_empty_false_for_content() {
        let b = EdgelessTextBlock::new(entity("e14"), "hello");
        assert!(!b.is_empty());
    }

    // ── wave AB: additional edgeless_text tests ──────────────────────────────

    /// Rotating 90 degrees three times results in 270 degrees total.
    #[test]
    fn edgeless_rotate_90_three_times_is_270() {
        let mut b = EdgelessTextBlock::new(entity("e-rot3"), "text");
        b.rotate(90.0);
        b.rotate(90.0);
        b.rotate(90.0);
        assert!((b.rotation_deg - 270.0).abs() < 0.001);
    }

    /// word_count of a 5-word sentence is 5.
    #[test]
    fn edgeless_word_count_five_words() {
        let b = EdgelessTextBlock::new(entity("e-5w"), "the quick brown fox jumps");
        assert_eq!(b.word_count(), 5);
    }

    /// Rotating 360 degrees results in 0 degrees (full wrap).
    #[test]
    fn edgeless_rotate_360_wraps_to_zero() {
        let mut b = EdgelessTextBlock::new(entity("e-360"), "text");
        b.rotate(360.0);
        assert!(b.rotation_deg.abs() < 0.001);
    }
}
