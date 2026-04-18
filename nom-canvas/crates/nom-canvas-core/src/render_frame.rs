//! RenderFrame and dirty region tracking for the NomCanvas render loop.

/// An axis-aligned bounding region that marks a portion of the canvas as needing repaint.
#[derive(Debug, Clone, PartialEq)]
pub struct DirtyRegion {
    /// Left edge in canvas coordinates.
    pub x: f32,
    /// Top edge in canvas coordinates.
    pub y: f32,
    /// Width of the dirty area.
    pub width: f32,
    /// Height of the dirty area.
    pub height: f32,
}

impl DirtyRegion {
    /// Creates a new `DirtyRegion`.
    pub fn new(x: f32, y: f32, width: f32, height: f32) -> Self {
        Self { x, y, width, height }
    }

    /// Returns the area of the region.
    pub fn area(&self) -> f32 {
        self.width * self.height
    }

    /// Expands this region to also cover `other`.
    pub fn expand_to_include(&mut self, other: &DirtyRegion) {
        let min_x = self.x.min(other.x);
        let min_y = self.y.min(other.y);
        let max_x = (self.x + self.width).max(other.x + other.width);
        let max_y = (self.y + self.height).max(other.y + other.height);
        self.x = min_x;
        self.y = min_y;
        self.width = max_x - min_x;
        self.height = max_y - min_y;
    }

    /// Returns `true` if the area is zero.
    pub fn is_empty(&self) -> bool {
        self.area() == 0.0
    }
}

/// Tracks a collection of dirty regions accumulated during a frame.
#[derive(Debug, Default)]
pub struct DirtyTracker {
    /// All dirty regions recorded this frame.
    pub regions: Vec<DirtyRegion>,
}

impl DirtyTracker {
    /// Creates an empty `DirtyTracker`.
    pub fn new() -> Self {
        Self { regions: Vec::new() }
    }

    /// Adds a dirty region.
    pub fn mark_dirty(&mut self, region: DirtyRegion) {
        self.regions.push(region);
    }

    /// Removes all dirty regions.
    pub fn clear(&mut self) {
        self.regions.clear();
    }

    /// Returns `true` if any dirty regions are registered.
    pub fn has_dirty(&self) -> bool {
        !self.regions.is_empty()
    }

    /// Merges all dirty regions into a single bounding region.
    /// Returns `None` if there are no dirty regions.
    pub fn merged_region(&self) -> Option<DirtyRegion> {
        let mut iter = self.regions.iter();
        let first = iter.next()?.clone();
        let merged = iter.fold(first, |mut acc, r| {
            acc.expand_to_include(r);
            acc
        });
        Some(merged)
    }
}

/// A single render frame with metadata and dirty region tracking.
#[derive(Debug)]
pub struct RenderFrame {
    /// Monotonically increasing frame identifier.
    pub frame_id: u64,
    /// Frame width in pixels.
    pub width: u32,
    /// Frame height in pixels.
    pub height: u32,
    /// Dirty region tracker for this frame.
    pub dirty: DirtyTracker,
    /// Number of elements painted this frame.
    pub element_count: u32,
}

impl RenderFrame {
    /// Creates a new `RenderFrame` with no dirty regions and zero element count.
    pub fn new(frame_id: u64, width: u32, height: u32) -> Self {
        Self {
            frame_id,
            width,
            height,
            dirty: DirtyTracker::new(),
            element_count: 0,
        }
    }

    /// Records the bounding box of an element as a dirty region.
    pub fn mark_element_dirty(&mut self, x: f32, y: f32, w: f32, h: f32) {
        self.dirty.mark_dirty(DirtyRegion::new(x, y, w, h));
    }

    /// Increments the element count for this frame.
    pub fn add_element(&mut self) {
        self.element_count += 1;
    }

    /// Returns `true` if any dirty regions have been recorded.
    pub fn needs_redraw(&self) -> bool {
        self.dirty.has_dirty()
    }

    /// Clears all dirty regions for this frame.
    pub fn clear_dirty(&mut self) {
        self.dirty.clear();
    }
}

#[cfg(test)]
mod render_frame_tests {
    use super::*;

    #[test]
    fn dirty_region_area() {
        let r = DirtyRegion::new(0.0, 0.0, 10.0, 5.0);
        assert_eq!(r.area(), 50.0);
    }

    #[test]
    fn dirty_region_is_empty_for_zero_area() {
        let r = DirtyRegion::new(5.0, 5.0, 0.0, 10.0);
        assert!(r.is_empty());
        let r2 = DirtyRegion::new(5.0, 5.0, 10.0, 0.0);
        assert!(r2.is_empty());
        let r3 = DirtyRegion::new(5.0, 5.0, 10.0, 10.0);
        assert!(!r3.is_empty());
    }

    #[test]
    fn dirty_region_expand_to_include_grows_region() {
        let mut a = DirtyRegion::new(0.0, 0.0, 10.0, 10.0);
        let b = DirtyRegion::new(5.0, 5.0, 20.0, 20.0);
        a.expand_to_include(&b);
        assert_eq!(a.x, 0.0);
        assert_eq!(a.y, 0.0);
        assert_eq!(a.width, 25.0);
        assert_eq!(a.height, 25.0);
    }

    #[test]
    fn dirty_tracker_mark_dirty_and_has_dirty() {
        let mut tracker = DirtyTracker::new();
        assert!(!tracker.has_dirty());
        tracker.mark_dirty(DirtyRegion::new(0.0, 0.0, 100.0, 100.0));
        assert!(tracker.has_dirty());
    }

    #[test]
    fn dirty_tracker_clear_removes_all() {
        let mut tracker = DirtyTracker::new();
        tracker.mark_dirty(DirtyRegion::new(0.0, 0.0, 50.0, 50.0));
        tracker.mark_dirty(DirtyRegion::new(60.0, 60.0, 10.0, 10.0));
        tracker.clear();
        assert!(!tracker.has_dirty());
        assert!(tracker.regions.is_empty());
    }

    #[test]
    fn merged_region_returns_none_for_empty() {
        let tracker = DirtyTracker::new();
        assert!(tracker.merged_region().is_none());
    }

    #[test]
    fn merged_region_covers_all_regions() {
        let mut tracker = DirtyTracker::new();
        tracker.mark_dirty(DirtyRegion::new(0.0, 0.0, 10.0, 10.0));
        tracker.mark_dirty(DirtyRegion::new(20.0, 30.0, 5.0, 5.0));
        let merged = tracker.merged_region().unwrap();
        assert_eq!(merged.x, 0.0);
        assert_eq!(merged.y, 0.0);
        assert_eq!(merged.width, 25.0);
        assert_eq!(merged.height, 35.0);
    }

    #[test]
    fn render_frame_needs_redraw_after_mark() {
        let mut frame = RenderFrame::new(1, 1920, 1080);
        assert!(!frame.needs_redraw());
        frame.mark_element_dirty(10.0, 10.0, 100.0, 50.0);
        assert!(frame.needs_redraw());
    }

    #[test]
    fn render_frame_clear_dirty_resets() {
        let mut frame = RenderFrame::new(2, 800, 600);
        frame.mark_element_dirty(0.0, 0.0, 800.0, 600.0);
        assert!(frame.needs_redraw());
        frame.clear_dirty();
        assert!(!frame.needs_redraw());
    }
}
