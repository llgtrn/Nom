/// Spatial index for canvas elements using an R-tree.
///
/// Provides O(log n) region queries and nearest-neighbour lookup.
/// Uses the `rstar` crate with `AABB<[f32; 2]>` envelopes.
use rstar::{PointDistance, RTree, RTreeObject, AABB};

use crate::elements::ElementBounds;

// ─── R-tree envelope ────────────────────────────────────────────────────────

/// Thin wrapper around `ElementBounds` that satisfies rstar's `RTreeObject`
/// and `PointDistance` traits.
#[derive(Debug)]
pub struct CanvasElementEnvelope {
    /// Element identifier.
    pub id: u64,
    /// Axis-aligned bounding box stored in the R-tree.
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
#[derive(Debug)]
pub struct SpatialIndex {
    tree: RTree<CanvasElementEnvelope>,
}

impl SpatialIndex {
    /// Creates a new empty spatial index.
    pub fn new() -> Self {
        Self { tree: RTree::new() }
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
        assert!(
            !found.contains(&2),
            "id 2 should not appear, got {:?}",
            found
        );
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
        assert!(
            !found.contains(&20),
            "id 20 should not appear, got {:?}",
            found
        );
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

    // ── new tests ────────────────────────────────────────────────────────────

    #[test]
    fn spatial_index_empty_query_returns_empty() {
        let idx = SpatialIndex::new();
        let found = idx.query_in_bounds([10.0, 10.0], [500.0, 500.0]);
        assert!(found.is_empty(), "query on empty index must return empty");
    }

    #[test]
    fn spatial_index_insert_and_count() {
        let mut idx = SpatialIndex::new();
        idx.insert(make_bounds(1, [0.0, 0.0], [10.0, 10.0]));
        idx.insert(make_bounds(2, [20.0, 20.0], [30.0, 30.0]));
        idx.insert(make_bounds(3, [40.0, 40.0], [50.0, 50.0]));
        assert_eq!(idx.len(), 3, "expected len=3 after 3 inserts");
    }

    #[test]
    fn spatial_index_remove_decrements() {
        let mut idx = SpatialIndex::new();
        idx.insert(make_bounds(1, [0.0, 0.0], [10.0, 10.0]));
        idx.insert(make_bounds(2, [20.0, 20.0], [30.0, 30.0]));
        idx.insert(make_bounds(3, [40.0, 40.0], [50.0, 50.0]));
        assert_eq!(idx.len(), 3);
        idx.remove(2, make_bounds(2, [20.0, 20.0], [30.0, 30.0]));
        assert_eq!(idx.len(), 2, "expected len=2 after removing one element");
    }

    #[test]
    fn spatial_index_range_query_partial() {
        // Insert A and B in non-overlapping regions; query rect overlaps only A.
        let mut idx = SpatialIndex::new();
        idx.insert(make_bounds(10, [0.0, 0.0], [20.0, 20.0])); // A
        idx.insert(make_bounds(20, [200.0, 200.0], [220.0, 220.0])); // B
        let found = idx.query_in_bounds([0.0, 0.0], [25.0, 25.0]);
        assert!(found.contains(&10), "expected A in results");
        assert!(!found.contains(&20), "B should not be in results");
    }

    #[test]
    fn spatial_index_point_query() {
        // Query with a zero-size rect at the centre of an element — should hit it.
        let mut idx = SpatialIndex::new();
        idx.insert(make_bounds(5, [10.0, 10.0], [30.0, 30.0]));
        let center = [20.0, 20.0]; // centre of element 5
        let found = idx.query_in_bounds(center, center);
        assert!(
            found.contains(&5),
            "point query at element centre must return the element"
        );
    }

    #[test]
    fn spatial_index_all_in_bounds() {
        let mut idx = SpatialIndex::new();
        for i in 1_u64..=5 {
            let base = i as f32 * 30.0;
            idx.insert(make_bounds(i, [base, base], [base + 20.0, base + 20.0]));
        }
        // Query the entire region containing all five elements.
        let found = idx.query_in_bounds([0.0, 0.0], [500.0, 500.0]);
        assert_eq!(found.len(), 5, "expected all 5 elements returned");
    }

    #[test]
    fn spatial_index_overlapping_elements() {
        // Two elements whose bounding boxes overlap; query overlapping region → both returned.
        let mut idx = SpatialIndex::new();
        idx.insert(make_bounds(1, [0.0, 0.0], [50.0, 50.0]));
        idx.insert(make_bounds(2, [30.0, 30.0], [80.0, 80.0]));
        let found = idx.query_in_bounds([35.0, 35.0], [45.0, 45.0]);
        assert!(found.contains(&1), "overlapping element 1 must be returned");
        assert!(found.contains(&2), "overlapping element 2 must be returned");
    }

    #[test]
    fn spatial_index_bulk_insert_and_len() {
        let mut idx = SpatialIndex::new();
        for i in 1_u64..=50 {
            let base = i as f32 * 10.0;
            idx.insert(make_bounds(i, [base, 0.0], [base + 8.0, 8.0]));
        }
        assert_eq!(idx.len(), 50, "expected 50 elements after bulk insert");
    }

    #[test]
    fn spatial_index_bulk_remove_all() {
        let mut idx = SpatialIndex::new();
        let bounds_list: Vec<_> = (1_u64..=5)
            .map(|i| {
                let base = i as f32 * 20.0;
                make_bounds(i, [base, base], [base + 10.0, base + 10.0])
            })
            .collect();
        for b in &bounds_list {
            idx.insert(*b);
        }
        assert_eq!(idx.len(), 5);
        for b in &bounds_list {
            idx.remove(b.id, *b);
        }
        assert!(
            idx.is_empty(),
            "index must be empty after removing all elements"
        );
    }

    #[test]
    fn spatial_index_nearest_among_many() {
        let mut idx = SpatialIndex::new();
        // Insert 5 elements at various positions.
        idx.insert(make_bounds(1, [0.0, 0.0], [10.0, 10.0])); // centre (5,5)
        idx.insert(make_bounds(2, [100.0, 0.0], [110.0, 10.0])); // centre (105,5)
        idx.insert(make_bounds(3, [0.0, 100.0], [10.0, 110.0])); // centre (5,105)
        idx.insert(make_bounds(4, [50.0, 50.0], [60.0, 60.0])); // centre (55,55)
        idx.insert(make_bounds(5, [200.0, 200.0], [210.0, 210.0])); // far away
                                                                    // Query near element 1
        let near = idx.nearest([3.0, 3.0], 100.0);
        assert_eq!(near, Some(1), "nearest to (3,3) should be element 1");
    }

    #[test]
    fn spatial_index_no_match_in_narrow_region() {
        let mut idx = SpatialIndex::new();
        idx.insert(make_bounds(1, [100.0, 100.0], [200.0, 200.0]));
        idx.insert(make_bounds(2, [300.0, 300.0], [400.0, 400.0]));
        // Query a region between them — neither element overlaps [210,210]→[290,290]
        let found = idx.query_in_bounds([210.0, 210.0], [290.0, 290.0]);
        assert!(found.is_empty(), "no elements in the gap region");
    }

    #[test]
    fn spatial_index_reinsert_after_remove() {
        let mut idx = SpatialIndex::new();
        let b = make_bounds(42, [0.0, 0.0], [20.0, 20.0]);
        idx.insert(b);
        idx.remove(42, b);
        assert!(idx.is_empty(), "should be empty after remove");
        idx.insert(b);
        assert_eq!(idx.len(), 1, "should have 1 element after re-insert");
        let found = idx.query_in_bounds([5.0, 5.0], [15.0, 15.0]);
        assert!(found.contains(&42), "re-inserted element must be queryable");
    }

    #[test]
    fn spatial_index_remove_nonexistent_is_safe() {
        // Removing an ID that was never inserted should not panic.
        let mut idx = SpatialIndex::new();
        idx.insert(make_bounds(1, [0.0, 0.0], [10.0, 10.0]));
        // This should not panic even if the element doesn't exist.
        idx.remove(999, make_bounds(999, [100.0, 100.0], [110.0, 110.0]));
        assert_eq!(idx.len(), 1, "original element must still be present");
    }

    #[test]
    fn spatial_index_range_query_touching_edge() {
        // Element exactly touches the query boundary.
        let mut idx = SpatialIndex::new();
        idx.insert(make_bounds(1, [50.0, 50.0], [100.0, 100.0]));
        // Query region whose right edge = element's left edge (50,50).
        let found = idx.query_in_bounds([0.0, 0.0], [50.0, 50.0]);
        // Touching (degenerate overlap) should be included.
        assert!(
            found.contains(&1),
            "element touching query boundary must be returned"
        );
    }

    // ── nearest-k query ──────────────────────────────────────────────────────

    #[test]
    fn nearest_k_returns_closest_elements() {
        let mut idx = SpatialIndex::new();
        idx.insert(make_bounds(1, [0.0, 0.0], [2.0, 2.0])); // near
        idx.insert(make_bounds(2, [10.0, 10.0], [12.0, 12.0])); // medium
        idx.insert(make_bounds(3, [50.0, 50.0], [52.0, 52.0])); // far
                                                                // nearest from [1, 1] should be element 1 (inside it).
        let nearest = idx.nearest([1.0, 1.0], 100.0);
        assert_eq!(nearest, Some(1));
    }

    #[test]
    fn nearest_with_many_elements_returns_one() {
        let mut idx = SpatialIndex::new();
        for i in 0..20_u64 {
            let base = i as f32 * 15.0 + 50.0;
            idx.insert(make_bounds(i + 1, [base, base], [base + 10.0, base + 10.0]));
        }
        // Should return something, not None
        let nearest = idx.nearest([55.0, 55.0], 500.0);
        assert!(nearest.is_some());
    }

    #[test]
    fn nearest_exceeds_max_dist_none() {
        let mut idx = SpatialIndex::new();
        idx.insert(make_bounds(1, [200.0, 200.0], [210.0, 210.0]));
        let result = idx.nearest([0.0, 0.0], 10.0);
        assert_eq!(result, None, "element beyond max_dist must not be returned");
    }

    #[test]
    fn query_large_region_returns_all() {
        let mut idx = SpatialIndex::new();
        for i in 1_u64..=8 {
            let base = i as f32 * 10.0;
            idx.insert(make_bounds(i, [base, base], [base + 5.0, base + 5.0]));
        }
        let found = idx.query_in_bounds([-1000.0, -1000.0], [1000.0, 1000.0]);
        assert_eq!(
            found.len(),
            8,
            "large query region must return all elements"
        );
    }

    #[test]
    fn insert_and_query_negative_coordinates() {
        let mut idx = SpatialIndex::new();
        idx.insert(make_bounds(1, [-100.0, -100.0], [-50.0, -50.0]));
        let found = idx.query_in_bounds([-200.0, -200.0], [0.0, 0.0]);
        assert!(
            found.contains(&1),
            "element at negative coords must be queryable"
        );
    }

    #[test]
    fn spatial_index_len_accurate_after_multiple_removes() {
        let mut idx = SpatialIndex::new();
        for i in 1_u64..=6 {
            let base = i as f32 * 20.0;
            idx.insert(make_bounds(i, [base, base], [base + 10.0, base + 10.0]));
        }
        assert_eq!(idx.len(), 6);
        idx.remove(1, make_bounds(1, [20.0, 20.0], [30.0, 30.0]));
        idx.remove(3, make_bounds(3, [60.0, 60.0], [70.0, 70.0]));
        idx.remove(5, make_bounds(5, [100.0, 100.0], [110.0, 110.0]));
        assert_eq!(idx.len(), 3);
    }

    #[test]
    fn is_empty_returns_true_initially() {
        assert!(SpatialIndex::new().is_empty());
    }

    #[test]
    fn is_empty_false_after_insert() {
        let mut idx = SpatialIndex::new();
        idx.insert(make_bounds(1, [0.0, 0.0], [1.0, 1.0]));
        assert!(!idx.is_empty());
    }

    #[test]
    fn query_single_element_exact_aabb() {
        let mut idx = SpatialIndex::new();
        idx.insert(make_bounds(7, [10.0, 20.0], [30.0, 40.0]));
        // Query exactly the element's own bounds.
        let found = idx.query_in_bounds([10.0, 20.0], [30.0, 40.0]);
        assert!(found.contains(&7));
    }

    #[test]
    fn nearest_to_point_inside_element_is_zero_dist() {
        let mut idx = SpatialIndex::new();
        idx.insert(make_bounds(1, [0.0, 0.0], [100.0, 100.0]));
        // Point inside the element has distance_2 = 0 to the AABB.
        let near = idx.nearest([50.0, 50.0], 0.5);
        assert_eq!(near, Some(1));
    }

    #[test]
    fn query_zero_size_region_at_element_corner() {
        let mut idx = SpatialIndex::new();
        idx.insert(make_bounds(1, [0.0, 0.0], [20.0, 20.0]));
        // Zero-size query at element's corner.
        let found = idx.query_in_bounds([0.0, 0.0], [0.0, 0.0]);
        assert!(
            found.contains(&1),
            "corner-touching zero-size query must hit element"
        );
    }

    #[test]
    fn two_elements_same_position_both_returnable() {
        let mut idx = SpatialIndex::new();
        idx.insert(make_bounds(1, [10.0, 10.0], [30.0, 30.0]));
        idx.insert(make_bounds(2, [10.0, 10.0], [30.0, 30.0]));
        let found = idx.query_in_bounds([10.0, 10.0], [30.0, 30.0]);
        assert!(found.contains(&1), "element 1 must be returned");
        assert!(found.contains(&2), "element 2 must be returned");
    }

    /// Simulate concurrent inserts by inserting 100 elements in sequence and
    /// verifying all are queryable.
    #[test]
    fn concurrent_insert_simulation_100_elements() {
        let mut idx = SpatialIndex::new();
        for i in 1_u64..=100 {
            let base = i as f32 * 5.0;
            idx.insert(make_bounds(i, [base, base], [base + 4.0, base + 4.0]));
        }
        assert_eq!(idx.len(), 100, "all 100 elements must be in index");
        // Query a large region to find all elements.
        let found = idx.query_in_bounds([0.0, 0.0], [600.0, 600.0]);
        assert_eq!(found.len(), 100, "query must return all 100 elements");
    }

    /// Query after 100 removes: index is empty and query returns nothing.
    #[test]
    fn query_after_100_removes() {
        let mut idx = SpatialIndex::new();
        let all_bounds: Vec<_> = (1_u64..=100)
            .map(|i| {
                let base = i as f32 * 5.0;
                make_bounds(i, [base, base], [base + 4.0, base + 4.0])
            })
            .collect();
        for b in &all_bounds {
            idx.insert(*b);
        }
        assert_eq!(idx.len(), 100);
        for b in &all_bounds {
            idx.remove(b.id, *b);
        }
        assert!(idx.is_empty(), "index must be empty after 100 removes");
        let found = idx.query_in_bounds([0.0, 0.0], [600.0, 600.0]);
        assert!(found.is_empty(), "query on empty index must return nothing");
    }

    /// Query with zero-area bounds at a point not covered by any element returns empty.
    #[test]
    fn query_zero_area_bounds_no_match() {
        let mut idx = SpatialIndex::new();
        idx.insert(make_bounds(1, [100.0, 100.0], [200.0, 200.0]));
        // Zero-area query at a point outside element 1.
        let found = idx.query_in_bounds([50.0, 50.0], [50.0, 50.0]);
        assert!(
            found.is_empty(),
            "zero-area query outside element must return empty"
        );
    }

    /// Sequential inserts then query: each new element is immediately findable.
    #[test]
    fn sequential_insert_immediately_queryable() {
        let mut idx = SpatialIndex::new();
        for i in 1_u64..=10 {
            let base = i as f32 * 20.0;
            idx.insert(make_bounds(i, [base, 0.0], [base + 10.0, 10.0]));
            // The newly inserted element must be findable right away.
            let found = idx.query_in_bounds([base, 0.0], [base + 10.0, 10.0]);
            assert!(
                found.contains(&i),
                "element {i} must be queryable immediately after insert"
            );
        }
    }

    /// Verify that removing half the elements leaves the other half queryable.
    #[test]
    fn remove_half_leaves_other_half_queryable() {
        let mut idx = SpatialIndex::new();
        for i in 1_u64..=10 {
            let base = i as f32 * 20.0;
            idx.insert(make_bounds(i, [base, 0.0], [base + 15.0, 15.0]));
        }
        // Remove odd ids.
        for i in (1_u64..=10).step_by(2) {
            let base = i as f32 * 20.0;
            idx.remove(i, make_bounds(i, [base, 0.0], [base + 15.0, 15.0]));
        }
        assert_eq!(idx.len(), 5, "5 elements must remain after removing odds");
        // Even ids must still be queryable.
        for i in (2_u64..=10).step_by(2) {
            let base = i as f32 * 20.0;
            let found = idx.query_in_bounds([base, 0.0], [base + 15.0, 15.0]);
            assert!(
                found.contains(&i),
                "even element {i} must still be queryable"
            );
        }
    }

    /// nearest on a single-element index returns that element.
    #[test]
    fn nearest_single_element_always_returned() {
        let mut idx = SpatialIndex::new();
        idx.insert(make_bounds(1, [0.0, 0.0], [10.0, 10.0]));
        let near = idx.nearest([5.0, 5.0], 1000.0);
        assert_eq!(near, Some(1), "single element must be the nearest");
    }

    // ── Wave AH: additional spatial_index tests ──────────────────────────────

    /// Insert 100 elements, all are queryable via a large region.
    #[test]
    fn spatial_index_insert_100_elements_all_queryable() {
        let mut idx = SpatialIndex::new();
        for i in 1_u64..=100 {
            let base = i as f32 * 6.0;
            idx.insert(make_bounds(i, [base, base], [base + 5.0, base + 5.0]));
        }
        assert_eq!(idx.len(), 100);
        let found = idx.query_in_bounds([0.0, 0.0], [700.0, 700.0]);
        assert_eq!(
            found.len(),
            100,
            "all 100 inserted elements must be queryable"
        );
    }

    /// query_in_bounds returns only elements whose AABB intersects the region.
    #[test]
    fn spatial_index_query_region_returns_intersecting() {
        let mut idx = SpatialIndex::new();
        idx.insert(make_bounds(1, [0.0, 0.0], [30.0, 30.0]));
        idx.insert(make_bounds(2, [100.0, 100.0], [130.0, 130.0]));
        idx.insert(make_bounds(3, [15.0, 15.0], [45.0, 45.0]));
        // Region [10,10]→[40,40] intersects elements 1 and 3 but not 2.
        let found = idx.query_in_bounds([10.0, 10.0], [40.0, 40.0]);
        assert!(found.contains(&1), "element 1 must intersect query region");
        assert!(found.contains(&3), "element 3 must intersect query region");
        assert!(
            !found.contains(&2),
            "element 2 must not intersect query region"
        );
    }

    /// query_in_bounds on a region with no elements returns empty vec.
    #[test]
    fn spatial_index_query_empty_region_returns_empty() {
        let mut idx = SpatialIndex::new();
        idx.insert(make_bounds(1, [500.0, 500.0], [600.0, 600.0]));
        // Query a region far from all elements.
        let found = idx.query_in_bounds([0.0, 0.0], [50.0, 50.0]);
        assert!(found.is_empty(), "no elements in this region");
    }

    /// After removing an element, it is not found in subsequent queries.
    #[test]
    fn spatial_index_remove_element_not_found_after() {
        let mut idx = SpatialIndex::new();
        idx.insert(make_bounds(7, [0.0, 0.0], [20.0, 20.0]));
        idx.remove(7, make_bounds(7, [0.0, 0.0], [20.0, 20.0]));
        let found = idx.query_in_bounds([0.0, 0.0], [20.0, 20.0]);
        assert!(!found.contains(&7), "removed element must not be found");
        assert!(
            idx.is_empty(),
            "index must be empty after removing sole element"
        );
    }

    /// Updating an element's position (remove + re-insert) moves it in the index.
    #[test]
    fn spatial_index_update_position_moves_element() {
        let mut idx = SpatialIndex::new();
        idx.insert(make_bounds(1, [0.0, 0.0], [20.0, 20.0]));
        // Move element 1 to a new position.
        idx.remove(1, make_bounds(1, [0.0, 0.0], [20.0, 20.0]));
        idx.insert(make_bounds(1, [200.0, 200.0], [220.0, 220.0]));
        // Old position must not return the element.
        let old = idx.query_in_bounds([0.0, 0.0], [20.0, 20.0]);
        assert!(!old.contains(&1), "element must not appear at old position");
        // New position must return the element.
        let new = idx.query_in_bounds([200.0, 200.0], [220.0, 220.0]);
        assert!(new.contains(&1), "element must appear at new position");
    }

    /// nearest_neighbor returns the correct closest element.
    #[test]
    fn spatial_index_nearest_neighbor_correct() {
        let mut idx = SpatialIndex::new();
        idx.insert(make_bounds(1, [0.0, 0.0], [10.0, 10.0])); // closest to (2,2)
        idx.insert(make_bounds(2, [80.0, 80.0], [90.0, 90.0])); // far
        idx.insert(make_bounds(3, [40.0, 40.0], [50.0, 50.0])); // medium
        let near = idx.nearest([2.0, 2.0], 200.0);
        assert_eq!(near, Some(1), "nearest to (2,2) must be element 1");
    }

    /// Bulk insert 10 elements and verify count.
    #[test]
    fn spatial_index_bulk_insert_10_elements() {
        let mut idx = SpatialIndex::new();
        for i in 1_u64..=10 {
            idx.insert(make_bounds(
                i,
                [i as f32 * 5.0, 0.0],
                [i as f32 * 5.0 + 4.0, 4.0],
            ));
        }
        assert_eq!(idx.len(), 10, "must have 10 elements after bulk insert");
    }

    /// Query with a very large region returns all elements.
    #[test]
    fn spatial_index_query_large_region_covers_all() {
        let mut idx = SpatialIndex::new();
        for i in 1_u64..=7 {
            let base = i as f32 * 50.0;
            idx.insert(make_bounds(i, [base, base], [base + 30.0, base + 30.0]));
        }
        let found = idx.query_in_bounds([-1e6, -1e6], [1e6, 1e6]);
        assert_eq!(found.len(), 7, "large region must cover all 7 elements");
    }

    /// Query a zero-area point region that coincides with a single element.
    #[test]
    fn spatial_index_query_point_region_single_element() {
        let mut idx = SpatialIndex::new();
        idx.insert(make_bounds(42, [10.0, 10.0], [30.0, 30.0]));
        idx.insert(make_bounds(43, [100.0, 100.0], [120.0, 120.0]));
        // Point query inside element 42.
        let found = idx.query_in_bounds([20.0, 20.0], [20.0, 20.0]);
        assert!(
            found.contains(&42),
            "point inside element 42 must return it"
        );
        assert!(!found.contains(&43), "element 43 must not be returned");
    }

    /// After removing all elements, is_empty returns true and query returns empty.
    #[test]
    fn spatial_index_clear_empties() {
        let mut idx = SpatialIndex::new();
        let bounds: Vec<_> = (1_u64..=5)
            .map(|i| make_bounds(i, [i as f32 * 10.0, 0.0], [i as f32 * 10.0 + 8.0, 8.0]))
            .collect();
        for b in &bounds {
            idx.insert(*b);
        }
        assert_eq!(idx.len(), 5);
        for b in &bounds {
            idx.remove(b.id, *b);
        }
        assert!(
            idx.is_empty(),
            "index must report empty after all elements removed"
        );
        assert_eq!(idx.len(), 0, "len must be 0 after clearing");
        assert!(
            idx.query_in_bounds([0.0, 0.0], [1000.0, 1000.0]).is_empty(),
            "query on cleared index must return empty"
        );
    }

    /// len() returns the correct count after a series of inserts.
    #[test]
    fn spatial_index_count_correct() {
        let mut idx = SpatialIndex::new();
        for i in 1_u64..=15 {
            idx.insert(make_bounds(
                i,
                [i as f32, i as f32],
                [i as f32 + 1.0, i as f32 + 1.0],
            ));
        }
        assert_eq!(idx.len(), 15, "count must match the number of inserts");
    }

    /// Re-inserting the same ID after removal does not create a duplicate.
    #[test]
    fn spatial_index_no_duplicates_after_reinsert() {
        let mut idx = SpatialIndex::new();
        let b = make_bounds(99, [0.0, 0.0], [10.0, 10.0]);
        idx.insert(b);
        idx.remove(99, b);
        idx.insert(b);
        // After remove + reinsert there must be exactly one entry.
        assert_eq!(
            idx.len(),
            1,
            "must have exactly 1 entry after remove+reinsert"
        );
        let found = idx.query_in_bounds([0.0, 0.0], [10.0, 10.0]);
        // Count occurrences of id 99 — must be 1.
        let count_99 = found.iter().filter(|&&id| id == 99).count();
        assert_eq!(
            count_99, 1,
            "re-inserted element must appear exactly once in query"
        );
    }

    // ── Wave AJ: spatial_index additional tests ──────────────────────────────

    /// Inserting many small elements into a large canvas — all are queryable.
    #[test]
    fn spatial_index_small_elements_in_large_canvas() {
        let mut idx = SpatialIndex::new();
        // 50 tiny 1x1 elements scattered across a 10000x10000 canvas.
        for i in 0..50_u64 {
            let base = i as f32 * 200.0;
            idx.insert(make_bounds(i + 1, [base, base], [base + 1.0, base + 1.0]));
        }
        assert_eq!(idx.len(), 50);
        let found = idx.query_in_bounds([0.0, 0.0], [10001.0, 10001.0]);
        assert_eq!(
            found.len(),
            50,
            "all small elements in large canvas must be queryable"
        );
    }

    /// Inserting one large element into a small canvas region — it is found.
    #[test]
    fn spatial_index_large_elements_in_small_canvas() {
        let mut idx = SpatialIndex::new();
        // One element covering the entire canvas.
        idx.insert(make_bounds(1, [0.0, 0.0], [5000.0, 5000.0]));
        // Also insert a tiny element in the corner.
        idx.insert(make_bounds(2, [1.0, 1.0], [2.0, 2.0]));
        assert_eq!(idx.len(), 2);
        // Query a tiny region: must return the large element too (it overlaps).
        let found = idx.query_in_bounds([0.5, 0.5], [1.5, 1.5]);
        assert!(
            found.contains(&1),
            "large element must overlap tiny query region"
        );
        assert!(found.contains(&2), "tiny element must be in query region");
    }

    /// Range query returns only elements fully or partially inside the rectangle.
    #[test]
    fn spatial_index_range_query_all_inside_rect() {
        let mut idx = SpatialIndex::new();
        // 4 elements, 3 inside query rect, 1 outside.
        idx.insert(make_bounds(1, [0.0, 0.0], [10.0, 10.0]));
        idx.insert(make_bounds(2, [5.0, 5.0], [15.0, 15.0]));
        idx.insert(make_bounds(3, [8.0, 8.0], [18.0, 18.0]));
        idx.insert(make_bounds(4, [500.0, 500.0], [510.0, 510.0]));
        let found = idx.query_in_bounds([0.0, 0.0], [20.0, 20.0]);
        assert!(found.contains(&1), "element 1 must be in range");
        assert!(found.contains(&2), "element 2 must be in range");
        assert!(found.contains(&3), "element 3 must be in range");
        assert!(!found.contains(&4), "element 4 must not be in range");
    }

    /// No false positives: elements clearly outside the query region are not returned.
    #[test]
    fn spatial_index_no_false_positives() {
        let mut idx = SpatialIndex::new();
        // All elements clustered around (1000, 1000).
        for i in 1_u64..=10 {
            let base = 1000.0 + i as f32 * 5.0;
            idx.insert(make_bounds(i, [base, base], [base + 4.0, base + 4.0]));
        }
        // Query in a completely different region.
        let found = idx.query_in_bounds([0.0, 0.0], [100.0, 100.0]);
        assert!(
            found.is_empty(),
            "no elements should be found in far-away region"
        );
    }

    /// Insert at boundary coordinates (very large positive and negative values).
    #[test]
    fn spatial_index_insert_at_boundary() {
        let mut idx = SpatialIndex::new();
        // Elements at extreme coordinates.
        idx.insert(make_bounds(1, [-1e6, -1e6], [-999999.0, -999999.0]));
        idx.insert(make_bounds(2, [1e6, 1e6], [1000001.0, 1000001.0]));
        assert_eq!(idx.len(), 2);
        // Query near element 1.
        let found1 = idx.query_in_bounds([-1e6 - 1.0, -1e6 - 1.0], [-999998.0, -999998.0]);
        assert!(
            found1.contains(&1),
            "element at extreme negative coords must be queryable"
        );
        // Query near element 2.
        let found2 = idx.query_in_bounds([999999.0, 999999.0], [1e6 + 2.0, 1e6 + 2.0]);
        assert!(
            found2.contains(&2),
            "element at extreme positive coords must be queryable"
        );
    }

    /// Removing a non-existent element does not panic and leaves existing elements intact.
    #[test]
    fn spatial_index_remove_nonexistent_safe() {
        let mut idx = SpatialIndex::new();
        idx.insert(make_bounds(1, [0.0, 0.0], [10.0, 10.0]));
        idx.insert(make_bounds(2, [20.0, 20.0], [30.0, 30.0]));
        // Remove an id that was never inserted — must not panic.
        idx.remove(9999, make_bounds(9999, [50.0, 50.0], [60.0, 60.0]));
        // Original elements must still be present.
        assert_eq!(
            idx.len(),
            2,
            "removing non-existent must not affect existing elements"
        );
        let found = idx.query_in_bounds([0.0, 0.0], [35.0, 35.0]);
        assert!(found.contains(&1), "element 1 must still be present");
        assert!(found.contains(&2), "element 2 must still be present");
    }

    /// Updating bounds (remove + re-insert at new location).
    #[test]
    fn spatial_index_update_bounds() {
        let mut idx = SpatialIndex::new();
        idx.insert(make_bounds(1, [0.0, 0.0], [20.0, 20.0]));
        // Update bounds: move element 1 to [100, 100]→[120, 120].
        idx.remove(1, make_bounds(1, [0.0, 0.0], [20.0, 20.0]));
        idx.insert(make_bounds(1, [100.0, 100.0], [120.0, 120.0]));
        // Must not be found at old location.
        let old = idx.query_in_bounds([0.0, 0.0], [20.0, 20.0]);
        assert!(!old.contains(&1), "element 1 must not appear at old bounds");
        // Must be found at new location.
        let new = idx.query_in_bounds([100.0, 100.0], [120.0, 120.0]);
        assert!(new.contains(&1), "element 1 must appear at updated bounds");
    }

    /// Insert many elements and verify they are still correct after many operations.
    #[test]
    fn spatial_index_rebalance_after_many_inserts() {
        let mut idx = SpatialIndex::new();
        // Insert 200 elements.
        for i in 1_u64..=200 {
            let base = i as f32 * 3.0;
            idx.insert(make_bounds(i, [base, base], [base + 2.0, base + 2.0]));
        }
        assert_eq!(idx.len(), 200, "must have 200 elements after bulk insert");
        // Remove 100 elements.
        for i in 1_u64..=100 {
            let base = i as f32 * 3.0;
            idx.remove(i, make_bounds(i, [base, base], [base + 2.0, base + 2.0]));
        }
        assert_eq!(idx.len(), 100, "must have 100 elements after removing half");
        // Re-insert the 100 removed elements at new positions.
        for i in 1_u64..=100 {
            let base = (i + 300) as f32 * 3.0;
            idx.insert(make_bounds(i, [base, base], [base + 2.0, base + 2.0]));
        }
        assert_eq!(idx.len(), 200, "must have 200 elements after re-inserts");
        // Large query must return all 200.
        let found = idx.query_in_bounds([-1.0, -1.0], [10000.0, 10000.0]);
        assert_eq!(
            found.len(),
            200,
            "all 200 elements must be queryable after rebalance"
        );
    }

    /// k-nearest-neighbors: querying nearest returns the single closest.
    #[test]
    fn spatial_index_k_nearest_neighbors() {
        let mut idx = SpatialIndex::new();
        idx.insert(make_bounds(1, [0.0, 0.0], [5.0, 5.0])); // closest to query
        idx.insert(make_bounds(2, [30.0, 30.0], [35.0, 35.0])); // medium distance
        idx.insert(make_bounds(3, [100.0, 100.0], [105.0, 105.0])); // far
                                                                    // Query nearest to (2, 2) — element 1 must win.
        let near = idx.nearest([2.0, 2.0], 200.0);
        assert_eq!(near, Some(1), "nearest to (2,2) must be element 1");
        // Query nearest to (102, 102) — element 3 must win.
        let near3 = idx.nearest([102.0, 102.0], 200.0);
        assert_eq!(near3, Some(3), "nearest to (102,102) must be element 3");
    }

    /// Query with max_dist=0: only elements containing the point (distance=0) are returned.
    #[test]
    fn spatial_index_nearest_zero_dist_requires_containment() {
        let mut idx = SpatialIndex::new();
        idx.insert(make_bounds(1, [0.0, 0.0], [20.0, 20.0])); // contains (10,10)
        idx.insert(make_bounds(2, [50.0, 50.0], [70.0, 70.0])); // does not contain (10,10)
                                                                // nearest with max_dist=0: only element whose AABB contains (10,10) qualifies.
        let near = idx.nearest([10.0, 10.0], 0.0);
        assert_eq!(
            near,
            Some(1),
            "only containing element must be returned with max_dist=0"
        );
    }

    /// Insert after a bulk removal restores correct len and queryability.
    #[test]
    fn spatial_index_insert_after_bulk_removal() {
        let mut idx = SpatialIndex::new();
        let bounds_list: Vec<_> = (1_u64..=10)
            .map(|i| make_bounds(i, [i as f32 * 10.0, 0.0], [i as f32 * 10.0 + 8.0, 8.0]))
            .collect();
        for b in &bounds_list {
            idx.insert(*b);
        }
        assert_eq!(idx.len(), 10);
        for b in &bounds_list {
            idx.remove(b.id, *b);
        }
        assert!(idx.is_empty());
        // Re-insert 3 new elements.
        idx.insert(make_bounds(100, [0.0, 0.0], [5.0, 5.0]));
        idx.insert(make_bounds(101, [10.0, 0.0], [15.0, 5.0]));
        idx.insert(make_bounds(102, [20.0, 0.0], [25.0, 5.0]));
        assert_eq!(idx.len(), 3, "must have 3 elements after re-insert");
        let found = idx.query_in_bounds([0.0, 0.0], [30.0, 10.0]);
        assert_eq!(
            found.len(),
            3,
            "all 3 re-inserted elements must be queryable"
        );
    }

    // ── Wave AL: additional spatial_index coverage ───────────────────────────

    /// Query returns elements whose AABB intersects the query rect.
    #[test]
    fn query_returns_elements_intersecting_query_rect() {
        let mut idx = SpatialIndex::new();
        idx.insert(make_bounds(1, [0.0, 0.0], [40.0, 40.0]));
        idx.insert(make_bounds(2, [50.0, 0.0], [90.0, 40.0]));
        idx.insert(make_bounds(3, [200.0, 200.0], [300.0, 300.0]));
        // Query rect overlaps elements 1 and 2 but not 3.
        let found = idx.query_in_bounds([20.0, 0.0], [70.0, 40.0]);
        assert!(found.contains(&1), "element 1 intersects query rect");
        assert!(found.contains(&2), "element 2 intersects query rect");
        assert!(
            !found.contains(&3),
            "element 3 does not intersect query rect"
        );
    }

    /// Update element bounds (remove + re-insert) changes query results.
    #[test]
    fn update_element_bounds_changes_query_results() {
        let mut idx = SpatialIndex::new();
        idx.insert(make_bounds(7, [0.0, 0.0], [20.0, 20.0]));
        // Element 7 is found at original location.
        let before = idx.query_in_bounds([0.0, 0.0], [20.0, 20.0]);
        assert!(before.contains(&7), "element must be at original location");
        // Update: move element 7 to [500, 500]→[520, 520].
        idx.remove(7, make_bounds(7, [0.0, 0.0], [20.0, 20.0]));
        idx.insert(make_bounds(7, [500.0, 500.0], [520.0, 520.0]));
        // Old location must be empty.
        let at_old = idx.query_in_bounds([0.0, 0.0], [20.0, 20.0]);
        assert!(
            !at_old.contains(&7),
            "element must not appear at old location"
        );
        // New location must contain the element.
        let at_new = idx.query_in_bounds([500.0, 500.0], [520.0, 520.0]);
        assert!(at_new.contains(&7), "element must appear at new location");
    }

    /// Remove element excludes it from all subsequent queries.
    #[test]
    fn remove_element_excludes_from_queries() {
        let mut idx = SpatialIndex::new();
        idx.insert(make_bounds(3, [10.0, 10.0], [30.0, 30.0]));
        idx.insert(make_bounds(4, [50.0, 50.0], [70.0, 70.0]));
        idx.remove(3, make_bounds(3, [10.0, 10.0], [30.0, 30.0]));
        // Query the full space — element 3 must not appear.
        let found = idx.query_in_bounds([0.0, 0.0], [100.0, 100.0]);
        assert!(
            !found.contains(&3),
            "removed element must not appear in queries"
        );
        assert!(found.contains(&4), "remaining element must still appear");
    }

    /// Bulk insert 100 elements then query all — count matches.
    #[test]
    fn bulk_insert_100_elements_query_count_matches() {
        let mut idx = SpatialIndex::new();
        for i in 1_u64..=100 {
            let base = i as f32 * 8.0;
            idx.insert(make_bounds(i, [base, base], [base + 6.0, base + 6.0]));
        }
        assert_eq!(
            idx.len(),
            100,
            "index must contain 100 elements after bulk insert"
        );
        let found = idx.query_in_bounds([0.0, 0.0], [900.0, 900.0]);
        assert_eq!(found.len(), 100, "query must return all 100 elements");
    }

    /// Query with zero-size rect at a point not inside any element returns zero hits.
    #[test]
    fn query_zero_size_rect_no_match_returns_zero_hits() {
        let mut idx = SpatialIndex::new();
        idx.insert(make_bounds(1, [100.0, 100.0], [200.0, 200.0]));
        idx.insert(make_bounds(2, [300.0, 300.0], [400.0, 400.0]));
        // Zero-size query at a point between the two elements.
        let found = idx.query_in_bounds([250.0, 250.0], [250.0, 250.0]);
        assert!(
            found.is_empty(),
            "zero-size query between elements must return zero hits"
        );
    }
}
