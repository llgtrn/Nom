/// Spatial index for canvas elements using an R-tree.
///
/// Provides O(log n) region queries and nearest-neighbour lookup.
/// Uses the `rstar` crate with `AABB<[f32; 2]>` envelopes.

use rstar::{RTree, RTreeObject, PointDistance, AABB};

use crate::elements::ElementBounds;

// ─── R-tree envelope ────────────────────────────────────────────────────────

/// Thin wrapper around `ElementBounds` that satisfies rstar's `RTreeObject`
/// and `PointDistance` traits.
pub struct CanvasElementEnvelope {
    pub id: u64,
    pub aabb: AABB<[f32; 2]>,
}

impl RTreeObject for CanvasElementEnvelope {
    type Envelope = AABB<[f32; 2]>;

    fn envelope(&self) -> Self::Envelope {
        self.aabb
    }
}

impl PointDistance for CanvasElementEnvelope {
    fn distance_2(&self, point: &[f32; 2]) -> f32 {
        self.aabb.distance_2(point)
    }
}

// rstar requires `PartialEq` for removal
impl PartialEq for CanvasElementEnvelope {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

// ─── SpatialIndex ────────────────────────────────────────────────────────────

/// O(log n) spatial index for canvas elements.
pub struct SpatialIndex {
    tree: RTree<CanvasElementEnvelope>,
}

impl SpatialIndex {
    pub fn new() -> Self {
        Self {
            tree: RTree::new(),
        }
    }

    /// Insert an element into the index.
    pub fn insert(&mut self, bounds: ElementBounds) {
        self.tree.insert(CanvasElementEnvelope {
            id: bounds.id,
            aabb: AABB::from_corners(bounds.min, bounds.max),
        });
    }

    /// Remove an element from the index.  `bounds` must match the value used
    /// when the element was inserted (needed by rstar for envelope lookup).
    pub fn remove(&mut self, id: u64, bounds: ElementBounds) {
        let env = CanvasElementEnvelope {
            id,
            aabb: AABB::from_corners(bounds.min, bounds.max),
        };
        self.tree.remove(&env);
    }

    /// Return all element IDs whose envelopes intersect `[min, max]`.
    pub fn query_in_bounds(&self, min: [f32; 2], max: [f32; 2]) -> Vec<u64> {
        let envelope = AABB::from_corners(min, max);
        self.tree
            .locate_in_envelope_intersecting(&envelope)
            .map(|e| e.id)
            .collect()
    }

    /// Return the ID of the element nearest to `pt`, or `None` if the index is
    /// empty or the nearest element is farther than `max_dist`.
    pub fn nearest(&self, pt: [f32; 2], max_dist: f32) -> Option<u64> {
        self.tree
            .nearest_neighbor(&pt)
            .filter(|e| e.aabb.distance_2(&pt).sqrt() <= max_dist)
            .map(|e| e.id)
    }

    /// Number of elements in the index.
    pub fn len(&self) -> usize {
        self.tree.size()
    }

    /// Returns `true` if the index contains no elements.
    pub fn is_empty(&self) -> bool {
        self.tree.size() == 0
    }
}

impl Default for SpatialIndex {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_bounds(id: u64, min: [f32; 2], max: [f32; 2]) -> ElementBounds {
        ElementBounds { id, min, max }
    }

    #[test]
    fn empty_index_len_zero() {
        let idx = SpatialIndex::new();
        assert_eq!(idx.len(), 0);
        assert!(idx.is_empty());
    }

    #[test]
    fn insert_increments_len() {
        let mut idx = SpatialIndex::new();
        idx.insert(make_bounds(1, [0.0, 0.0], [10.0, 10.0]));
        assert_eq!(idx.len(), 1);
        idx.insert(make_bounds(2, [20.0, 20.0], [30.0, 30.0]));
        assert_eq!(idx.len(), 2);
    }

    #[test]
    fn query_in_bounds_returns_overlapping_ids() {
        let mut idx = SpatialIndex::new();
        idx.insert(make_bounds(1, [0.0, 0.0], [50.0, 50.0]));
        idx.insert(make_bounds(2, [100.0, 100.0], [200.0, 200.0]));
        idx.insert(make_bounds(3, [25.0, 25.0], [75.0, 75.0]));

        let mut found = idx.query_in_bounds([0.0, 0.0], [60.0, 60.0]);
        found.sort();
        // Element 1 (0-50) and element 3 (25-75) both overlap [0-60]
        assert!(found.contains(&1), "expected id 1, got {:?}", found);
        assert!(found.contains(&3), "expected id 3, got {:?}", found);
        assert!(!found.contains(&2), "id 2 should not appear, got {:?}", found);
    }

    #[test]
    fn query_in_bounds_empty_result() {
        let mut idx = SpatialIndex::new();
        idx.insert(make_bounds(1, [200.0, 200.0], [300.0, 300.0]));
        let found = idx.query_in_bounds([0.0, 0.0], [10.0, 10.0]);
        assert!(found.is_empty());
    }

    #[test]
    fn remove_decrements_len() {
        let mut idx = SpatialIndex::new();
        let b = make_bounds(1, [0.0, 0.0], [10.0, 10.0]);
        idx.insert(b);
        assert_eq!(idx.len(), 1);
        idx.remove(1, make_bounds(1, [0.0, 0.0], [10.0, 10.0]));
        assert_eq!(idx.len(), 0);
    }

    #[test]
    fn remove_then_query_returns_nothing() {
        let mut idx = SpatialIndex::new();
        idx.insert(make_bounds(1, [0.0, 0.0], [50.0, 50.0]));
        idx.remove(1, make_bounds(1, [0.0, 0.0], [50.0, 50.0]));
        let found = idx.query_in_bounds([0.0, 0.0], [50.0, 50.0]);
        assert!(found.is_empty());
    }

    #[test]
    fn nearest_returns_closest_id() {
        let mut idx = SpatialIndex::new();
        idx.insert(make_bounds(1, [0.0, 0.0], [10.0, 10.0]));
        idx.insert(make_bounds(2, [100.0, 100.0], [110.0, 110.0]));

        // Query near element 1
        let near = idx.nearest([5.0, 5.0], 50.0);
        assert_eq!(near, Some(1));
    }

    #[test]
    fn nearest_beyond_max_dist_returns_none() {
        let mut idx = SpatialIndex::new();
        idx.insert(make_bounds(1, [100.0, 100.0], [110.0, 110.0]));

        // Query at [0,0] with max_dist=5 — element is ~95 away
        let near = idx.nearest([0.0, 0.0], 5.0);
        assert_eq!(near, None);
    }

    #[test]
    fn nearest_empty_index_returns_none() {
        let idx = SpatialIndex::new();
        assert_eq!(idx.nearest([0.0, 0.0], 100.0), None);
    }

    #[test]
    fn default_is_empty() {
        let idx = SpatialIndex::default();
        assert!(idx.is_empty());
    }

    #[test]
    fn multiple_inserts_all_queryable() {
        let mut idx = SpatialIndex::new();
        for i in 0..10_u64 {
            let base = i as f32 * 20.0;
            idx.insert(make_bounds(i + 1, [base, 0.0], [base + 15.0, 15.0]));
        }
        assert_eq!(idx.len(), 10);

        // Query the whole space
        let found = idx.query_in_bounds([0.0, 0.0], [200.0, 20.0]);
        assert_eq!(found.len(), 10);
    }

    #[test]
    fn spatial_index_insert_and_query() {
        let mut idx = SpatialIndex::new();
        idx.insert(make_bounds(10, [5.0, 5.0], [15.0, 15.0]));
        idx.insert(make_bounds(20, [50.0, 50.0], [60.0, 60.0]));
        assert_eq!(idx.len(), 2);
        // Query region that covers only element 10
        let found = idx.query_in_bounds([0.0, 0.0], [20.0, 20.0]);
        assert!(found.contains(&10), "expected id 10, got {:?}", found);
        assert!(!found.contains(&20), "id 20 should not appear, got {:?}", found);
    }

    #[test]
    fn spatial_index_radius_query() {
        let mut idx = SpatialIndex::new();
        idx.insert(make_bounds(1, [0.0, 0.0], [10.0, 10.0]));
        idx.insert(make_bounds(2, [500.0, 500.0], [510.0, 510.0]));
        // nearest within a large radius picks the nearby element
        let near = idx.nearest([5.0, 5.0], 200.0);
        assert_eq!(near, Some(1), "expected element 1 as nearest");
        // nearest within a tiny radius excludes the far element
        let none = idx.nearest([5.0, 5.0], 1.0);
        // element 1 bounds distance_2 to [5,5] is 0 (point is inside) → distance = 0 ≤ 1
        assert_eq!(none, Some(1), "point inside bounds has distance 0");
    }

    #[test]
    fn spatial_index_empty_returns_empty() {
        let idx = SpatialIndex::new();
        let found = idx.query_in_bounds([0.0, 0.0], [1000.0, 1000.0]);
        assert!(found.is_empty(), "empty index must return no results");
        assert_eq!(idx.nearest([0.0, 0.0], 9999.0), None);
    }
}
