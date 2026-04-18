//! Grid snapping and element bounds utilities for the NomCanvas viewport.

/// Configuration for snap-to-grid behaviour.
#[derive(Debug, Clone, PartialEq)]
pub struct GridConfig {
    /// Size of each grid cell in pixels.
    pub cell_size_px: f32,
    /// X coordinate of the grid origin.
    pub origin_x: f32,
    /// Y coordinate of the grid origin.
    pub origin_y: f32,
    /// Whether snapping is active.
    pub enabled: bool,
}

impl GridConfig {
    /// Create a new `GridConfig` with origin at (0, 0) and snapping enabled.
    pub fn new(cell_size_px: f32) -> Self {
        Self {
            cell_size_px,
            origin_x: 0.0,
            origin_y: 0.0,
            enabled: true,
        }
    }

    /// Snap an x coordinate to the nearest grid line.
    /// Returns `x` unchanged when snapping is disabled.
    pub fn snap_x(&self, x: f32) -> f32 {
        if !self.enabled {
            return x;
        }
        let relative = x - self.origin_x;
        (relative / self.cell_size_px).round() * self.cell_size_px + self.origin_x
    }

    /// Snap a y coordinate to the nearest grid line.
    /// Returns `y` unchanged when snapping is disabled.
    pub fn snap_y(&self, y: f32) -> f32 {
        if !self.enabled {
            return y;
        }
        let relative = y - self.origin_y;
        (relative / self.cell_size_px).round() * self.cell_size_px + self.origin_y
    }

    /// Snap both axes of a point, returning `(snapped_x, snapped_y)`.
    pub fn snap_point(&self, x: f32, y: f32) -> (f32, f32) {
        (self.snap_x(x), self.snap_y(y))
    }
}

/// Axis-aligned bounding rectangle.
#[derive(Debug, Clone, PartialEq)]
pub struct BoundsRect {
    /// Left edge x coordinate.
    pub x: f32,
    /// Top edge y coordinate.
    pub y: f32,
    /// Width of the rectangle.
    pub width: f32,
    /// Height of the rectangle.
    pub height: f32,
}

impl BoundsRect {
    /// Create a new `BoundsRect`.
    pub fn new(x: f32, y: f32, width: f32, height: f32) -> Self {
        Self { x, y, width, height }
    }

    /// Right edge: `x + width`.
    pub fn right(&self) -> f32 {
        self.x + self.width
    }

    /// Bottom edge: `y + height`.
    pub fn bottom(&self) -> f32 {
        self.y + self.height
    }

    /// Returns `true` when the point `(px, py)` lies inside (or on the border of) this rect.
    pub fn contains_point(&self, px: f32, py: f32) -> bool {
        px >= self.x && px <= self.right() && py >= self.y && py <= self.bottom()
    }

    /// Smallest axis-aligned rect that contains both `self` and `other`.
    pub fn union(&self, other: &BoundsRect) -> BoundsRect {
        let min_x = self.x.min(other.x);
        let min_y = self.y.min(other.y);
        let max_x = self.right().max(other.right());
        let max_y = self.bottom().max(other.bottom());
        BoundsRect::new(min_x, min_y, max_x - min_x, max_y - min_y)
    }

    /// Returns `true` when `self` and `other` overlap (touching edges count as overlap).
    pub fn intersects(&self, other: &BoundsRect) -> bool {
        self.x <= other.right()
            && self.right() >= other.x
            && self.y <= other.bottom()
            && self.bottom() >= other.y
    }

    /// Area of the rectangle (`width * height`).
    pub fn area(&self) -> f32 {
        self.width * self.height
    }
}

/// Utilities for computing unions and derived properties of collections of `BoundsRect`.
pub struct BoundsUnion;

impl BoundsUnion {
    /// Union of all rects in `rects`. Returns `None` when the slice is empty.
    pub fn union_all(rects: &[BoundsRect]) -> Option<BoundsRect> {
        let mut iter = rects.iter();
        let first = iter.next()?;
        let result = iter.fold(first.clone(), |acc, r| acc.union(r));
        Some(result)
    }

    /// Centroid of `rect`: `(x + width/2, y + height/2)`.
    pub fn centroid(rect: &BoundsRect) -> (f32, f32) {
        (rect.x + rect.width / 2.0, rect.y + rect.height / 2.0)
    }

    /// Expand `rect` outward by `margin` on all four sides.
    pub fn expand(rect: BoundsRect, margin: f32) -> BoundsRect {
        BoundsRect::new(
            rect.x - margin,
            rect.y - margin,
            rect.width + margin * 2.0,
            rect.height + margin * 2.0,
        )
    }
}

#[cfg(test)]
mod snap_grid_tests {
    use super::*;

    // 1. GridConfig::snap_x() snaps to grid
    #[test]
    fn snap_x_snaps_to_grid() {
        let cfg = GridConfig::new(10.0);
        // 13.0 is closer to 10.0 than to 20.0 → snaps to 10.0
        assert_eq!(cfg.snap_x(13.0), 10.0);
        // 17.0 is closer to 20.0 → snaps to 20.0
        assert_eq!(cfg.snap_x(17.0), 20.0);
        // Exact multiple passes through unchanged
        assert_eq!(cfg.snap_x(30.0), 30.0);
    }

    // 2. snap_x() identity when disabled
    #[test]
    fn snap_x_identity_when_disabled() {
        let mut cfg = GridConfig::new(10.0);
        cfg.enabled = false;
        assert_eq!(cfg.snap_x(13.7), 13.7);
    }

    // 3. BoundsRect::contains_point() true
    #[test]
    fn contains_point_inside_returns_true() {
        let r = BoundsRect::new(0.0, 0.0, 100.0, 50.0);
        assert!(r.contains_point(50.0, 25.0));
        // On border counts as inside
        assert!(r.contains_point(0.0, 0.0));
        assert!(r.contains_point(100.0, 50.0));
    }

    // 4. BoundsRect::contains_point() false outside
    #[test]
    fn contains_point_outside_returns_false() {
        let r = BoundsRect::new(0.0, 0.0, 100.0, 50.0);
        assert!(!r.contains_point(101.0, 25.0));
        assert!(!r.contains_point(50.0, 51.0));
        assert!(!r.contains_point(-1.0, 25.0));
    }

    // 5. BoundsRect::union() spans both rects
    #[test]
    fn union_spans_both_rects() {
        let a = BoundsRect::new(0.0, 0.0, 50.0, 50.0);
        let b = BoundsRect::new(30.0, 30.0, 50.0, 50.0);
        let u = a.union(&b);
        assert_eq!(u.x, 0.0);
        assert_eq!(u.y, 0.0);
        assert_eq!(u.right(), 80.0);
        assert_eq!(u.bottom(), 80.0);
    }

    // 6. BoundsRect::intersects() true for overlap
    #[test]
    fn intersects_true_for_overlap() {
        let a = BoundsRect::new(0.0, 0.0, 60.0, 60.0);
        let b = BoundsRect::new(40.0, 40.0, 60.0, 60.0);
        assert!(a.intersects(&b));
    }

    // 7. BoundsRect::intersects() false for no overlap
    #[test]
    fn intersects_false_for_no_overlap() {
        let a = BoundsRect::new(0.0, 0.0, 40.0, 40.0);
        let b = BoundsRect::new(50.0, 50.0, 40.0, 40.0);
        assert!(!a.intersects(&b));
    }

    // 8. BoundsUnion::union_all() returns None for empty
    #[test]
    fn union_all_returns_none_for_empty() {
        assert!(BoundsUnion::union_all(&[]).is_none());
    }

    // 9. BoundsUnion::expand() increases all sides
    #[test]
    fn expand_increases_all_sides() {
        let r = BoundsRect::new(10.0, 10.0, 100.0, 80.0);
        let expanded = BoundsUnion::expand(r, 5.0);
        assert_eq!(expanded.x, 5.0);
        assert_eq!(expanded.y, 5.0);
        assert_eq!(expanded.width, 110.0);
        assert_eq!(expanded.height, 90.0);
    }
}
