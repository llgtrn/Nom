//! Preview panel — document render preview pane.

use crate::DocumentId;

/// Preview panel state.
#[derive(Debug)]
pub struct Preview {
    pub source_id: Option<DocumentId>,
    pub is_visible: bool,
    pub scroll_offset_px: f32,
}

impl Preview {
    pub fn new() -> Self {
        Self {
            source_id: None,
            is_visible: false,
            scroll_offset_px: 0.0,
        }
    }

    /// Open the preview for the given document.
    pub fn open(&mut self, id: DocumentId) {
        self.source_id = Some(id);
        self.is_visible = true;
    }

    /// Close the preview pane and clear the source.
    pub fn close(&mut self) {
        self.source_id = None;
        self.is_visible = false;
    }

    /// Scroll the preview by `dy` pixels (positive = down).
    pub fn scroll_by(&mut self, dy: f32) {
        self.scroll_offset_px = (self.scroll_offset_px + dy).max(0.0);
    }

    /// Stub paint method — rendering lives in the GPU layer.
    pub fn paint(&self) {}
}

impl Default for Preview {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_is_hidden_with_no_source() {
        let p = Preview::new();
        assert!(p.source_id.is_none());
        assert!(!p.is_visible);
    }

    #[test]
    fn open_sets_source_and_visible() {
        let mut p = Preview::new();
        p.open(42);
        assert_eq!(p.source_id, Some(42));
        assert!(p.is_visible);
    }

    #[test]
    fn close_clears_source_and_visible() {
        let mut p = Preview::new();
        p.open(42);
        p.close();
        assert!(p.source_id.is_none());
        assert!(!p.is_visible);
    }

    #[test]
    fn scroll_by_clamps_to_zero() {
        let mut p = Preview::new();
        p.scroll_by(100.0);
        assert_eq!(p.scroll_offset_px, 100.0);
        p.scroll_by(-200.0);
        assert_eq!(p.scroll_offset_px, 0.0);
    }
}
