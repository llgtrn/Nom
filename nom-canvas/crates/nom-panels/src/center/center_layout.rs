//! Layout descriptor for the center editor region.

use crate::center::tab_manager::TabManager;

/// Direction along which the center area is split.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SplitDirection {
    Horizontal,
    Vertical,
}

/// Describes how the center editor area is arranged.
#[derive(Debug, Clone)]
pub struct CenterLayout {
    pub tabs: TabManager,
    pub split: Option<SplitDirection>,
    /// Fraction allocated to the primary (left/top) pane. Clamped to 0.1–0.9.
    pub split_ratio: f32,
}

impl CenterLayout {
    /// Create a default layout with no split and a 50/50 ratio.
    pub fn new() -> Self {
        Self {
            tabs: TabManager::new(),
            split: None,
            split_ratio: 0.5,
        }
    }

    /// Activate a split in the given direction (builder-style).
    pub fn with_split(mut self, dir: SplitDirection) -> Self {
        self.split = Some(dir);
        self
    }

    /// Set the split ratio, clamped to the range `[0.1, 0.9]`.
    pub fn set_split_ratio(mut self, ratio: f32) -> Self {
        self.split_ratio = ratio.clamp(0.1, 0.9);
        self
    }
}

impl Default for CenterLayout {
    fn default() -> Self {
        Self::new()
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_has_no_split() {
        let layout = CenterLayout::new();
        assert!(layout.split.is_none());
        assert!((layout.split_ratio - 0.5).abs() < f32::EPSILON);
    }

    #[test]
    fn with_split_sets_direction() {
        let layout = CenterLayout::new().with_split(SplitDirection::Vertical);
        assert_eq!(layout.split, Some(SplitDirection::Vertical));
    }

    #[test]
    fn split_ratio_clamped() {
        let lo = CenterLayout::new().set_split_ratio(-1.0);
        assert!((lo.split_ratio - 0.1).abs() < f32::EPSILON);

        let hi = CenterLayout::new().set_split_ratio(2.0);
        assert!((hi.split_ratio - 0.9).abs() < f32::EPSILON);

        let mid = CenterLayout::new().set_split_ratio(0.7);
        assert!((mid.split_ratio - 0.7).abs() < f32::EPSILON);
    }
}
