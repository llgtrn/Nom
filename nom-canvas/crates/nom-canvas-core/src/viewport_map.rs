//! Viewport element spatial index: bounding boxes, map, and visibility queries.

/// Axis-aligned bounding box for a canvas element.
#[derive(Debug, Clone, PartialEq)]
pub struct ElementBounds {
    /// Unique element identifier.
    pub element_id: u64,
    /// Left edge (canvas coordinates).
    pub x: f32,
    /// Top edge (canvas coordinates).
    pub y: f32,
    /// Width of the element.
    pub width: f32,
    /// Height of the element.
    pub height: f32,
}

impl ElementBounds {
    /// Create a new `ElementBounds`.
    pub fn new(element_id: u64, x: f32, y: f32, width: f32, height: f32) -> Self {
        Self { element_id, x, y, width, height }
    }

    /// Right edge: `x + width`.
    #[inline]
    pub fn right(&self) -> f32 {
        self.x + self.width
    }

    /// Bottom edge: `y + height`.
    #[inline]
    pub fn bottom(&self) -> f32 {
        self.y + self.height
    }

    /// AABB overlap test against an arbitrary rectangle `(rx, ry, rw, rh)`.
    ///
    /// Returns `true` when the two rectangles overlap (touching edges count as
    /// non-overlapping per strict less-than comparisons).
    pub fn intersects_rect(&self, rx: f32, ry: f32, rw: f32, rh: f32) -> bool {
        self.x < rx + rw && self.right() > rx && self.y < ry + rh && self.bottom() > ry
    }
}

/// Spatial index mapping element IDs to their canvas-space bounding boxes.
#[derive(Debug, Default)]
pub struct ViewportMap {
    elements: Vec<ElementBounds>,
}

impl ViewportMap {
    /// Create an empty `ViewportMap`.
    pub fn new() -> Self {
        Self { elements: Vec::new() }
    }

    /// Insert or replace the bounds for an element.
    ///
    /// If an entry with the same `element_id` already exists it is replaced;
    /// otherwise the new entry is appended.
    pub fn insert(&mut self, bounds: ElementBounds) {
        if let Some(pos) = self.elements.iter().position(|e| e.element_id == bounds.element_id) {
            self.elements[pos] = bounds;
        } else {
            self.elements.push(bounds);
        }
    }

    /// Remove the entry for `element_id`.  No-op if the element is not present.
    pub fn remove(&mut self, element_id: u64) {
        self.elements.retain(|e| e.element_id != element_id);
    }

    /// Look up bounds by `element_id`.
    pub fn get(&self, element_id: u64) -> Option<&ElementBounds> {
        self.elements.iter().find(|e| e.element_id == element_id)
    }

    /// Number of elements currently in the map.
    pub fn len(&self) -> usize {
        self.elements.len()
    }

    /// Returns `true` when the map contains no elements.
    pub fn is_empty(&self) -> bool {
        self.elements.is_empty()
    }
}

/// Query which elements are visible inside a viewport rectangle.
#[derive(Debug, Clone)]
pub struct VisibilityQuery {
    /// Left edge of the viewport (canvas coordinates).
    pub viewport_x: f32,
    /// Top edge of the viewport (canvas coordinates).
    pub viewport_y: f32,
    /// Width of the viewport.
    pub viewport_w: f32,
    /// Height of the viewport.
    pub viewport_h: f32,
}

impl VisibilityQuery {
    /// Create a new `VisibilityQuery` for the given viewport rectangle.
    pub fn new(viewport_x: f32, viewport_y: f32, viewport_w: f32, viewport_h: f32) -> Self {
        Self { viewport_x, viewport_y, viewport_w, viewport_h }
    }

    /// Return all `ElementBounds` from `map` that intersect this viewport.
    pub fn query<'a>(&self, map: &'a ViewportMap) -> Vec<&'a ElementBounds> {
        map.elements
            .iter()
            .filter(|e| e.intersects_rect(self.viewport_x, self.viewport_y, self.viewport_w, self.viewport_h))
            .collect()
    }

    /// Return the element IDs of all elements that intersect this viewport.
    pub fn query_ids(&self, map: &ViewportMap) -> Vec<u64> {
        self.query(map).into_iter().map(|e| e.element_id).collect()
    }
}

#[cfg(test)]
mod viewport_map_tests {
    use super::{ElementBounds, ViewportMap, VisibilityQuery};

    // ── ElementBounds ────────────────────────────────────────────────────────

    #[test]
    fn element_bounds_right_and_bottom() {
        let b = ElementBounds::new(1, 10.0, 20.0, 30.0, 40.0);
        assert_eq!(b.right(), 40.0);
        assert_eq!(b.bottom(), 60.0);
    }

    #[test]
    fn element_bounds_intersects_overlap() {
        // Element at (0,0) 100×100; rect at (50,50) 100×100 — overlap in all axes.
        let b = ElementBounds::new(2, 0.0, 0.0, 100.0, 100.0);
        assert!(b.intersects_rect(50.0, 50.0, 100.0, 100.0));
    }

    #[test]
    fn element_bounds_intersects_no_overlap() {
        // Element at (0,0) 10×10; rect at (20,0) 10×10 — separated on x axis.
        let b = ElementBounds::new(3, 0.0, 0.0, 10.0, 10.0);
        assert!(!b.intersects_rect(20.0, 0.0, 10.0, 10.0));
    }

    // ── ViewportMap ──────────────────────────────────────────────────────────

    #[test]
    fn viewport_map_insert_and_len() {
        let mut map = ViewportMap::new();
        assert_eq!(map.len(), 0);
        map.insert(ElementBounds::new(10, 0.0, 0.0, 50.0, 50.0));
        map.insert(ElementBounds::new(11, 60.0, 0.0, 50.0, 50.0));
        assert_eq!(map.len(), 2);
    }

    #[test]
    fn viewport_map_remove_element() {
        let mut map = ViewportMap::new();
        map.insert(ElementBounds::new(20, 0.0, 0.0, 50.0, 50.0));
        map.insert(ElementBounds::new(21, 60.0, 0.0, 50.0, 50.0));
        map.remove(20);
        assert_eq!(map.len(), 1);
        assert!(map.get(20).is_none());
        assert!(map.get(21).is_some());
    }

    #[test]
    fn viewport_map_get_existing() {
        let mut map = ViewportMap::new();
        map.insert(ElementBounds::new(30, 5.0, 10.0, 20.0, 25.0));
        let found = map.get(30).expect("element 30 must be present");
        assert_eq!(found.x, 5.0);
        assert_eq!(found.y, 10.0);
        assert_eq!(found.width, 20.0);
        assert_eq!(found.height, 25.0);
    }

    // ── VisibilityQuery ──────────────────────────────────────────────────────

    #[test]
    fn visibility_query_returns_visible() {
        let mut map = ViewportMap::new();
        // Element fully inside viewport.
        map.insert(ElementBounds::new(40, 10.0, 10.0, 20.0, 20.0));
        let q = VisibilityQuery::new(0.0, 0.0, 100.0, 100.0);
        let results = q.query(&map);
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].element_id, 40);
    }

    #[test]
    fn visibility_query_excludes_outside() {
        let mut map = ViewportMap::new();
        // Visible element.
        map.insert(ElementBounds::new(50, 5.0, 5.0, 10.0, 10.0));
        // Element completely outside (to the right).
        map.insert(ElementBounds::new(51, 200.0, 5.0, 10.0, 10.0));
        let q = VisibilityQuery::new(0.0, 0.0, 100.0, 100.0);
        let results = q.query(&map);
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].element_id, 50);
    }

    #[test]
    fn visibility_query_ids_correct() {
        let mut map = ViewportMap::new();
        map.insert(ElementBounds::new(60, 0.0, 0.0, 50.0, 50.0));
        map.insert(ElementBounds::new(61, 200.0, 200.0, 50.0, 50.0)); // outside
        map.insert(ElementBounds::new(62, 80.0, 80.0, 10.0, 10.0));
        let q = VisibilityQuery::new(0.0, 0.0, 100.0, 100.0);
        let mut ids = q.query_ids(&map);
        ids.sort();
        assert_eq!(ids, vec![60, 62]);
    }
}
