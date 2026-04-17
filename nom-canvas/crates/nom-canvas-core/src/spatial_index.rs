//! Grid-based spatial index for canvas elements.
//!
//! The canvas is divided into cells of [`DEFAULT_GRID_SIZE`] model units on
//! each side (matching the AFFiNE pattern of `DEFAULT_GRID_SIZE = 3000`).
//! Each element is registered in every cell its bounding box overlaps.
//!
//! Typical operations:
//! - [`SpatialIndex::insert`] — add a new element.
//! - [`SpatialIndex::remove`] — remove an element.
//! - [`SpatialIndex::update`] — re-register after the element moved/resized.
//! - [`SpatialIndex::search`] — return all elements overlapping a query rectangle.

#![deny(unsafe_code)]

use std::collections::HashMap;
use nom_gpui::{Bounds, Pixels};
#[cfg(test)]
use nom_gpui::{Point, Size};
use crate::element::ElementId;

// ── constants ─────────────────────────────────────────────────────────────────

/// Side length of each grid cell in model units (logical pixels).
///
/// Matches the AFFiNE `DEFAULT_GRID_SIZE` value.
pub const DEFAULT_GRID_SIZE: f32 = 3000.0;

// ── SpatialIndex ─────────────────────────────────────────────────────────────

/// Grid-based spatial index.
///
/// Elements are stored in all grid cells they overlap.  The grid uses
/// signed integer cell coordinates so the canvas can extend in any direction
/// from the origin.
pub struct SpatialIndex {
    /// Side length of one cell in model units.
    grid_size: f32,
    /// Map from `(col, row)` cell coordinate to the set of element IDs in
    /// that cell.
    cells: HashMap<(i32, i32), Vec<ElementId>>,
    /// Per-element cached bounds, used by `remove` and `update` to locate
    /// the old cells without a linear scan.
    bounds_cache: HashMap<ElementId, Bounds<Pixels>>,
}

impl SpatialIndex {
    /// Create a new index with the given cell size.
    pub fn new(grid_size: f32) -> Self {
        Self {
            grid_size,
            cells: HashMap::new(),
            bounds_cache: HashMap::new(),
        }
    }

    /// Insert `id` into every cell overlapped by `bounds`.
    ///
    /// If `id` is already present the behaviour is unspecified; call
    /// [`update`](SpatialIndex::update) instead.
    pub fn insert(&mut self, id: ElementId, bounds: Bounds<Pixels>) {
        self.bounds_cache.insert(id, bounds);
        for cell in self.cells_for_bounds(bounds) {
            self.cells.entry(cell).or_default().push(id);
        }
    }

    /// Remove `id` from all cells it currently occupies.
    ///
    /// Does nothing if `id` was never inserted.
    pub fn remove(&mut self, id: ElementId) {
        if let Some(old_bounds) = self.bounds_cache.remove(&id) {
            for cell in self.cells_for_bounds(old_bounds) {
                if let Some(bucket) = self.cells.get_mut(&cell) {
                    bucket.retain(|&existing| existing != id);
                    // Prune empty buckets to avoid unbounded memory growth as
                    // elements move around the infinite canvas.
                    if bucket.is_empty() {
                        self.cells.remove(&cell);
                    }
                }
            }
        }
    }

    /// Re-register `id` under `new_bounds`, removing it from cells that no
    /// longer overlap.
    pub fn update(&mut self, id: ElementId, new_bounds: Bounds<Pixels>) {
        self.remove(id);
        self.insert(id, new_bounds);
    }

    /// Return all element IDs whose bounding boxes overlap `bound`.
    ///
    /// The optional `filter` predicate lets callers exclude specific IDs
    /// (e.g. the element currently being dragged).
    ///
    /// The returned Vec is deduplicated and sorted by raw ID value for
    /// deterministic ordering.
    pub fn search(
        &self,
        bound: Bounds<Pixels>,
        filter: Option<&dyn Fn(ElementId) -> bool>,
    ) -> Vec<ElementId> {
        let mut seen: HashMap<ElementId, ()> = HashMap::new();

        for cell in self.cells_for_bounds(bound) {
            if let Some(bucket) = self.cells.get(&cell) {
                for &id in bucket {
                    if let Some(ref f) = filter {
                        if !f(id) {
                            continue;
                        }
                    }
                    // Bounds-overlap refinement: an element registered in a
                    // cell that overlaps the query rectangle may itself not
                    // overlap if it was inserted into the cell due to its
                    // bounding box spanning a larger area.
                    if let Some(elem_bounds) = self.bounds_cache.get(&id) {
                        if bounds_overlap(*elem_bounds, bound) {
                            seen.insert(id, ());
                        }
                    }
                }
            }
        }

        let mut result: Vec<ElementId> = seen.into_keys().collect();
        result.sort();
        result
    }

    // ── private ──────────────────────────────────────────────────────────────

    /// Return every `(col, row)` cell that `b` overlaps.
    fn cells_for_bounds(&self, b: Bounds<Pixels>) -> Vec<(i32, i32)> {
        let gs = self.grid_size;
        let x0 = b.origin.x.0;
        let y0 = b.origin.y.0;
        let x1 = x0 + b.size.width.0;
        let y1 = y0 + b.size.height.0;

        // Cell index: floor(coord / grid_size).  Using `floor` means the cell
        // at index 0 covers [0, gs), index 1 covers [gs, 2*gs), and negative
        // coords go into negative-index cells.
        let col_min = (x0 / gs).floor() as i32;
        let col_max = (x1 / gs).floor() as i32;
        let row_min = (y0 / gs).floor() as i32;
        let row_max = (y1 / gs).floor() as i32;

        let mut cells = Vec::new();
        for col in col_min..=col_max {
            for row in row_min..=row_max {
                cells.push((col, row));
            }
        }
        cells
    }
}

// ── axis-aligned overlap test ─────────────────────────────────────────────────

/// Returns `true` when two axis-aligned bounding boxes overlap (share at
/// least one point).
fn bounds_overlap(a: Bounds<Pixels>, b: Bounds<Pixels>) -> bool {
    let a_right = a.origin.x.0 + a.size.width.0;
    let a_bottom = a.origin.y.0 + a.size.height.0;
    let b_right = b.origin.x.0 + b.size.width.0;
    let b_bottom = b.origin.y.0 + b.size.height.0;

    a.origin.x.0 <= b_right
        && b.origin.x.0 <= a_right
        && a.origin.y.0 <= b_bottom
        && b.origin.y.0 <= a_bottom
}

// ── helper ────────────────────────────────────────────────────────────────────

#[cfg(test)]
fn make_bounds(x: f32, y: f32, w: f32, h: f32) -> Bounds<Pixels> {
    Bounds {
        origin: Point { x: Pixels(x), y: Pixels(y) },
        size: Size { width: Pixels(w), height: Pixels(h) },
    }
}

// ── tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::element::ElementId; // type alias for u64 — used as type annotations below

    #[test]
    fn insert_and_search_returns_element() {
        let mut idx = SpatialIndex::new(DEFAULT_GRID_SIZE);
        let id: ElementId = 42;
        // Small element in the first grid cell.
        idx.insert(id, make_bounds(10.0, 10.0, 50.0, 50.0));

        // Query that overlaps the element.
        let results = idx.search(make_bounds(0.0, 0.0, 100.0, 100.0), None);
        assert!(results.contains(&id), "element not found after insert");
    }

    #[test]
    fn search_with_filter_drops_ids() {
        let mut idx = SpatialIndex::new(DEFAULT_GRID_SIZE);
        let a: ElementId = 1;
        let b: ElementId = 2;

        idx.insert(a, make_bounds(0.0, 0.0, 100.0, 100.0));
        idx.insert(b, make_bounds(0.0, 0.0, 100.0, 100.0));

        // Filter that rejects `b`.
        let results = idx.search(
            make_bounds(0.0, 0.0, 100.0, 100.0),
            Some(&|id: ElementId| id != b),
        );

        assert!(results.contains(&a), "element a should pass the filter");
        assert!(!results.contains(&b), "element b should be dropped by the filter");
    }

    #[test]
    fn update_changes_cell_membership() {
        let mut idx = SpatialIndex::new(DEFAULT_GRID_SIZE);
        let id: ElementId = 7;

        // Insert in the first cell.
        idx.insert(id, make_bounds(100.0, 100.0, 50.0, 50.0));

        // Move the element into a second cell far away.
        let new_bounds = make_bounds(5000.0, 5000.0, 50.0, 50.0);
        idx.update(id, new_bounds);

        // Searching the original area must come up empty.
        let old_results = idx.search(make_bounds(0.0, 0.0, 500.0, 500.0), None);
        assert!(
            !old_results.contains(&id),
            "element should no longer appear in its old position"
        );

        // Searching the new area must find it.
        let new_results = idx.search(make_bounds(4900.0, 4900.0, 200.0, 200.0), None);
        assert!(
            new_results.contains(&id),
            "element should appear at its new position"
        );
    }

    #[test]
    fn many_elements_in_one_cell_all_returned() {
        let mut idx = SpatialIndex::new(DEFAULT_GRID_SIZE);

        let ids: Vec<ElementId> = (0u64..20).collect();
        for &id in &ids {
            // All fit inside a single 3000×3000 cell.
            idx.insert(id, make_bounds(10.0, 10.0, 10.0, 10.0));
        }

        let results = idx.search(make_bounds(0.0, 0.0, 100.0, 100.0), None);

        for id in &ids {
            assert!(
                results.contains(id),
                "element {} was not returned by search",
                id
            );
        }
        assert_eq!(results.len(), ids.len(), "result count should match insert count");
    }
}
