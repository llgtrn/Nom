//! Frame block: a named container that groups child blocks on the canvas.
#![deny(unsafe_code)]
use crate::block_model::NomtuRef;
use serde::{Deserialize, Serialize};

/// A frame block that groups child blocks within a labelled, styled boundary.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct FrameBlock {
    /// DB entity reference (NON-OPTIONAL).
    pub entity: NomtuRef,
    /// Display label of the frame.
    pub label: String,
    /// Ordered list of child entity references contained in this frame.
    pub children: Vec<NomtuRef>,
    /// Optional CSS-style background color string (e.g. `"#ffffff"`).
    pub background_color: Option<String>,
    /// Border width in logical pixels.
    pub border_width: f32,
}

impl FrameBlock {
    /// Construct a new, empty [`FrameBlock`] with the given entity and label.
    pub fn new(entity: NomtuRef, label: impl Into<String>) -> Self {
        Self {
            entity,
            label: label.into(),
            children: Vec::new(),
            background_color: None,
            border_width: 1.0,
        }
    }

    /// Append a child entity reference to this frame.
    pub fn add_child(&mut self, child: NomtuRef) {
        self.children.push(child);
    }

    /// Remove the first child whose `id` matches the given value. Returns `true` if removed.
    pub fn remove_child(&mut self, id: &str) -> bool {
        if let Some(pos) = self.children.iter().position(|c| c.id == id) {
            self.children.remove(pos);
            true
        } else {
            false
        }
    }

    /// Number of direct children.
    pub fn child_count(&self) -> usize {
        self.children.len()
    }

    /// Set the background color string.
    pub fn set_background(&mut self, color: impl Into<String>) {
        self.background_color = Some(color.into());
    }

    /// Set the border width in logical pixels.
    pub fn set_border_width(&mut self, width: f32) {
        self.border_width = width;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entity(id: &str) -> NomtuRef {
        NomtuRef::new(id, "frame", "concept")
    }

    #[test]
    fn frame_new_has_empty_children() {
        let f = FrameBlock::new(entity("f1"), "My Frame");
        assert_eq!(f.label, "My Frame");
        assert!(f.children.is_empty());
    }

    #[test]
    fn frame_new_default_border_width() {
        let f = FrameBlock::new(entity("f2"), "");
        assert!((f.border_width - 1.0).abs() < f32::EPSILON);
    }

    #[test]
    fn frame_new_no_background_by_default() {
        let f = FrameBlock::new(entity("f3"), "label");
        assert!(f.background_color.is_none());
    }

    #[test]
    fn frame_entity_non_optional() {
        let f = FrameBlock::new(entity("eid-frame"), "label");
        assert_eq!(f.entity.id, "eid-frame");
        assert!(!f.entity.id.is_empty());
    }

    #[test]
    fn frame_add_child_increments_count() {
        let mut f = FrameBlock::new(entity("f4"), "label");
        f.add_child(entity("c1"));
        f.add_child(entity("c2"));
        assert_eq!(f.child_count(), 2);
    }

    #[test]
    fn frame_remove_child_existing() {
        let mut f = FrameBlock::new(entity("f5"), "label");
        f.add_child(entity("c1"));
        f.add_child(entity("c2"));
        let removed = f.remove_child("c1");
        assert!(removed);
        assert_eq!(f.child_count(), 1);
        assert_eq!(f.children[0].id, "c2");
    }

    #[test]
    fn frame_remove_child_missing_returns_false() {
        let mut f = FrameBlock::new(entity("f6"), "label");
        f.add_child(entity("c1"));
        let removed = f.remove_child("not-present");
        assert!(!removed);
        assert_eq!(f.child_count(), 1);
    }

    #[test]
    fn frame_child_count_zero_on_empty() {
        let f = FrameBlock::new(entity("f7"), "label");
        assert_eq!(f.child_count(), 0);
    }

    #[test]
    fn frame_set_background() {
        let mut f = FrameBlock::new(entity("f8"), "label");
        f.set_background("#aabbcc");
        assert_eq!(f.background_color.as_deref(), Some("#aabbcc"));
    }

    #[test]
    fn frame_set_background_overwrites() {
        let mut f = FrameBlock::new(entity("f9"), "label");
        f.set_background("#000000");
        f.set_background("#ffffff");
        assert_eq!(f.background_color.as_deref(), Some("#ffffff"));
    }

    #[test]
    fn frame_set_border_width() {
        let mut f = FrameBlock::new(entity("f10"), "label");
        f.set_border_width(4.5);
        assert!((f.border_width - 4.5).abs() < f32::EPSILON);
    }

    #[test]
    fn frame_set_border_width_zero() {
        let mut f = FrameBlock::new(entity("f11"), "label");
        f.set_border_width(0.0);
        assert!((f.border_width).abs() < f32::EPSILON);
    }

    #[test]
    fn frame_remove_only_child_leaves_empty() {
        let mut f = FrameBlock::new(entity("f12"), "label");
        f.add_child(entity("only"));
        f.remove_child("only");
        assert_eq!(f.child_count(), 0);
    }
}
