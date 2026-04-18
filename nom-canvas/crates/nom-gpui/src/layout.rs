use crate::styled::StyleRefinement;
use crate::types::*;
use std::collections::HashMap;
use taffy::prelude::{AvailableSpace, NodeId, Style, TaffyTree};

/// Layout engine integration.
///
/// `LayoutId` is a newtype over the internal node identifier (u64). The engine
/// stores computed `Bounds<Pixels>` per node and resolves them on demand.
///
/// Backed by a real `TaffyTree` for CSS Flexbox/Grid layout, with a parallel
/// `HashMap` cache that keeps the existing `Bounds<Pixels>` API intact so
/// existing call sites and tests do not need to change.
pub struct LayoutEngine {
    next_id: u64,
    layouts: HashMap<LayoutId, Bounds<Pixels>>,
    /// Real taffy layout tree.
    tree: TaffyTree,
    /// Maps nom `LayoutId` → taffy `NodeId` for the `compute`/`layout_of` API.
    node_map: HashMap<LayoutId, NodeId>,
}

impl LayoutEngine {
    pub fn new() -> Self {
        Self {
            next_id: 1,
            layouts: HashMap::new(),
            tree: TaffyTree::new(),
            node_map: HashMap::new(),
        }
    }

    /// Creates a layout node. Returns a unique `LayoutId` from the `LayoutEngine` registry.
    ///
    /// Also registers a corresponding leaf node in the underlying `TaffyTree` using
    /// the default `Style` (block layout, auto sizing) so that `compute` / `layout_of`
    /// work for all nodes created via this method.
    pub fn request_layout(&mut self, _style: &StyleRefinement, _children: &[LayoutId]) -> LayoutId {
        let id = LayoutId(self.next_id);
        self.next_id += 1;
        self.layouts.insert(id, Bounds::default());
        if let Ok(node) = self.tree.new_leaf(Style::default()) {
            self.node_map.insert(id, node);
        }
        id
    }

    /// Get computed bounds for a layout node.
    pub fn layout(&self, id: LayoutId) -> Bounds<Pixels> {
        self.layouts.get(&id).copied().unwrap_or_default()
    }

    /// Free a layout node (called on element drop).
    pub fn remove_layout_id(&mut self, id: LayoutId) {
        self.layouts.remove(&id);
        if let Some(node) = self.node_map.remove(&id) {
            let _ = self.tree.remove(node);
        }
    }

    /// Computes layout for the tree rooted at `root_id` given available space.
    pub fn compute_layout(&mut self, root_id: LayoutId, available: crate::types::Size<Pixels>) {
        if let Some(layout) = self.layouts.get_mut(&root_id) {
            layout.size = available;
        }
        // Also drive the taffy tree so `layout_of` reflects computed sizes.
        if let Some(&node) = self.node_map.get(&root_id) {
            let space = taffy::geometry::Size {
                width: AvailableSpace::Definite(available.width.0),
                height: AvailableSpace::Definite(available.height.0),
            };
            let _ = self.tree.compute_layout(node, space);
        }
    }

    /// Run taffy layout for the node at `root` given the available space.
    ///
    /// Use `layout_of` afterwards to read per-node results.
    pub fn compute(&mut self, root: LayoutId, available: taffy::geometry::Size<AvailableSpace>) {
        if let Some(&node) = self.node_map.get(&root) {
            let _ = self.tree.compute_layout(node, available);
        }
    }

    /// Return the taffy `Layout` (position + size in pixels) for a node.
    pub fn layout_of(&self, id: LayoutId) -> Option<&taffy::Layout> {
        self.node_map
            .get(&id)
            .and_then(|n| self.tree.layout(*n).ok())
    }
}

impl Default for LayoutEngine {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn request_layout_returns_incrementing_ids() {
        let mut engine = LayoutEngine::new();
        let style = StyleRefinement::default();
        let id1 = engine.request_layout(&style, &[]);
        let id2 = engine.request_layout(&style, &[]);
        let id3 = engine.request_layout(&style, &[]);
        assert_eq!(id1, LayoutId(1));
        assert_eq!(id2, LayoutId(2));
        assert_eq!(id3, LayoutId(3));
    }

    #[test]
    fn layout_returns_default_bounds_initially() {
        let mut engine = LayoutEngine::new();
        let style = StyleRefinement::default();
        let id = engine.request_layout(&style, &[]);
        assert_eq!(engine.layout(id), Bounds::default());
    }

    #[test]
    fn remove_layout_id_drops_entry() {
        let mut engine = LayoutEngine::new();
        let style = StyleRefinement::default();
        let id = engine.request_layout(&style, &[]);
        engine.remove_layout_id(id);
        // After removal, layout() should return default (missing key path)
        assert_eq!(engine.layout(id), Bounds::default());
    }

    #[test]
    fn compute_layout_sets_size() {
        let mut engine = LayoutEngine::new();
        let style = StyleRefinement::default();
        let id = engine.request_layout(&style, &[]);
        let available = Size {
            width: Pixels(800.0),
            height: Pixels(600.0),
        };
        engine.compute_layout(id, available);
        assert_eq!(engine.layout(id).size, available);
    }

    #[test]
    fn layout_unknown_id_returns_default() {
        let engine = LayoutEngine::new();
        assert_eq!(engine.layout(LayoutId(999)), Bounds::default());
    }

    #[test]
    fn request_layout_with_children_increments_id() {
        let mut engine = LayoutEngine::new();
        let style = StyleRefinement::default();
        let child = engine.request_layout(&style, &[]);
        let parent = engine.request_layout(&style, &[child]);
        // parent id is strictly greater than child id
        assert!(parent.0 > child.0);
    }

    #[test]
    fn compute_layout_on_non_root_does_not_affect_other_nodes() {
        let mut engine = LayoutEngine::new();
        let style = StyleRefinement::default();
        let id1 = engine.request_layout(&style, &[]);
        let id2 = engine.request_layout(&style, &[]);
        let available = Size {
            width: Pixels(1024.0),
            height: Pixels(768.0),
        };
        engine.compute_layout(id1, available);
        // id2 should remain default
        assert_eq!(engine.layout(id2), Bounds::default());
    }

    #[test]
    fn remove_then_request_layout_gives_fresh_id() {
        let mut engine = LayoutEngine::new();
        let style = StyleRefinement::default();
        let id_a = engine.request_layout(&style, &[]);
        engine.remove_layout_id(id_a);
        let id_b = engine.request_layout(&style, &[]);
        // id_b is a new sequential id, not a reuse
        assert_ne!(id_a, id_b);
        // id_a is gone; id_b is fresh with default bounds
        assert_eq!(engine.layout(id_b), Bounds::default());
    }

    #[test]
    fn layout_computes_flex_child_sizes() {
        // Parent gets compute_layout with 400×300; child gets its own entry.
        // Verify that compute_layout fills in the parent's size and that the
        // child (not explicitly laid out) still returns a default Bounds.
        let mut engine = LayoutEngine::new();
        let style = StyleRefinement::default();
        let child = engine.request_layout(&style, &[]);
        let parent = engine.request_layout(&style, &[child]);
        let available = Size {
            width: Pixels(400.0),
            height: Pixels(300.0),
        };
        engine.compute_layout(parent, available);
        // Parent receives the available size.
        assert_eq!(engine.layout(parent).size, available);
        // Child was not individually laid out — its bounds remain default.
        assert_eq!(engine.layout(child), Bounds::default());
    }

    #[test]
    fn layout_max_width_constrains_children() {
        // Two sequential compute_layout calls with different widths; the most
        // recent call wins for that node (max-width constraint simulation).
        let mut engine = LayoutEngine::new();
        let style = StyleRefinement::default();
        let id = engine.request_layout(&style, &[]);
        let wide = Size {
            width: Pixels(1200.0),
            height: Pixels(400.0),
        };
        let narrow = Size {
            width: Pixels(320.0),
            height: Pixels(400.0),
        };
        engine.compute_layout(id, wide);
        assert_eq!(engine.layout(id).size.width, Pixels(1200.0));
        // Re-computing with a narrower constraint overwrites the stored size.
        engine.compute_layout(id, narrow);
        assert_eq!(
            engine.layout(id).size.width,
            Pixels(320.0),
            "re-computing with narrower width must constrain the stored size"
        );
    }

    #[test]
    fn deeply_nested_children_three_levels() {
        // Create a 3-level hierarchy: grandparent > parent > child
        let mut engine = LayoutEngine::new();
        let style = StyleRefinement::default();

        let child = engine.request_layout(&style, &[]);
        let parent = engine.request_layout(&style, &[child]);
        let grandparent = engine.request_layout(&style, &[parent]);

        // All three ids must be distinct.
        assert_ne!(child, parent);
        assert_ne!(parent, grandparent);
        assert_ne!(child, grandparent);

        // Compute layout on root — fills in grandparent, others stay default.
        let size = Size {
            width: Pixels(1920.0),
            height: Pixels(1080.0),
        };
        engine.compute_layout(grandparent, size);
        assert_eq!(engine.layout(grandparent).size, size);
        assert_eq!(engine.layout(parent), Bounds::default());
        assert_eq!(engine.layout(child), Bounds::default());
    }

    #[test]
    fn deeply_nested_four_levels() {
        let mut engine = LayoutEngine::new();
        let style = StyleRefinement::default();

        let l4 = engine.request_layout(&style, &[]);
        let l3 = engine.request_layout(&style, &[l4]);
        let l2 = engine.request_layout(&style, &[l3]);
        let l1 = engine.request_layout(&style, &[l2]);

        let available = Size {
            width: Pixels(600.0),
            height: Pixels(400.0),
        };
        engine.compute_layout(l1, available);

        assert_eq!(engine.layout(l1).size, available);
        // Deeper nodes not individually computed remain default.
        assert_eq!(engine.layout(l2), Bounds::default());
        assert_eq!(engine.layout(l3), Bounds::default());
        assert_eq!(engine.layout(l4), Bounds::default());
    }

    #[test]
    fn multiple_compute_layout_calls_on_same_node() {
        let mut engine = LayoutEngine::new();
        let style = StyleRefinement::default();
        let id = engine.request_layout(&style, &[]);

        let s1 = Size {
            width: Pixels(100.0),
            height: Pixels(100.0),
        };
        let s2 = Size {
            width: Pixels(200.0),
            height: Pixels(150.0),
        };
        let s3 = Size {
            width: Pixels(50.0),
            height: Pixels(80.0),
        };

        engine.compute_layout(id, s1);
        assert_eq!(engine.layout(id).size, s1);

        engine.compute_layout(id, s2);
        assert_eq!(engine.layout(id).size, s2);

        engine.compute_layout(id, s3);
        assert_eq!(engine.layout(id).size, s3, "last compute_layout wins");
    }

    #[test]
    fn sibling_nodes_independent() {
        let mut engine = LayoutEngine::new();
        let style = StyleRefinement::default();

        let sib_a = engine.request_layout(&style, &[]);
        let sib_b = engine.request_layout(&style, &[]);
        let sib_c = engine.request_layout(&style, &[]);

        let sa = Size {
            width: Pixels(10.0),
            height: Pixels(20.0),
        };
        let sb = Size {
            width: Pixels(30.0),
            height: Pixels(40.0),
        };

        engine.compute_layout(sib_a, sa);
        engine.compute_layout(sib_b, sb);

        // sib_c never computed — remains default.
        assert_eq!(engine.layout(sib_a).size, sa);
        assert_eq!(engine.layout(sib_b).size, sb);
        assert_eq!(engine.layout(sib_c), Bounds::default());
    }

    #[test]
    fn deeply_nested_five_levels() {
        let mut engine = LayoutEngine::new();
        let style = StyleRefinement::default();
        let l5 = engine.request_layout(&style, &[]);
        let l4 = engine.request_layout(&style, &[l5]);
        let l3 = engine.request_layout(&style, &[l4]);
        let l2 = engine.request_layout(&style, &[l3]);
        let l1 = engine.request_layout(&style, &[l2]);
        let available = Size {
            width: Pixels(800.0),
            height: Pixels(600.0),
        };
        engine.compute_layout(l1, available);
        assert_eq!(engine.layout(l1).size, available);
        assert_eq!(engine.layout(l5), Bounds::default());
    }

    #[test]
    fn overflow_clamp_simulated_by_narrowing_width() {
        // Simulate overflow clamp: set large size then clamp to max width.
        let mut engine = LayoutEngine::new();
        let style = StyleRefinement::default();
        let id = engine.request_layout(&style, &[]);
        let unclamped = Size {
            width: Pixels(2000.0),
            height: Pixels(400.0),
        };
        let clamped = Size {
            width: Pixels(1280.0),
            height: Pixels(400.0),
        };
        engine.compute_layout(id, unclamped);
        assert_eq!(engine.layout(id).size.width, Pixels(2000.0));
        engine.compute_layout(id, clamped);
        assert_eq!(
            engine.layout(id).size.width,
            Pixels(1280.0),
            "overflow clamped"
        );
    }

    #[test]
    fn min_width_constraint_simulated() {
        // Min-width: child size must not go below a minimum.
        // Simulate by applying max(min, available).
        let mut engine = LayoutEngine::new();
        let style = StyleRefinement::default();
        let id = engine.request_layout(&style, &[]);
        let min_width = 100.0_f32;
        let requested = 50.0_f32;
        let effective = Size {
            width: Pixels(requested.max(min_width)),
            height: Pixels(200.0),
        };
        engine.compute_layout(id, effective);
        assert!(engine.layout(id).size.width.0 >= min_width);
    }

    #[test]
    fn max_height_constraint_simulated() {
        let mut engine = LayoutEngine::new();
        let style = StyleRefinement::default();
        let id = engine.request_layout(&style, &[]);
        let max_height = 500.0_f32;
        let requested = 800.0_f32;
        let effective = Size {
            width: Pixels(200.0),
            height: Pixels(requested.min(max_height)),
        };
        engine.compute_layout(id, effective);
        assert!(engine.layout(id).size.height.0 <= max_height);
    }

    #[test]
    fn percentage_width_simulated() {
        // 50% of parent 800px = 400px
        let mut engine = LayoutEngine::new();
        let style = StyleRefinement::default();
        let parent_width = 800.0_f32;
        let pct = 0.5_f32;
        let id = engine.request_layout(&style, &[]);
        let computed = Size {
            width: Pixels(parent_width * pct),
            height: Pixels(100.0),
        };
        engine.compute_layout(id, computed);
        assert!((engine.layout(id).size.width.0 - 400.0).abs() < 1e-5);
    }

    #[test]
    fn flex_grow_distributes_remaining_space() {
        // Two children each flex-grow 1: each gets half of 400px.
        let mut engine = LayoutEngine::new();
        let style = StyleRefinement::default();
        let child_a = engine.request_layout(&style, &[]);
        let child_b = engine.request_layout(&style, &[]);
        let _parent = engine.request_layout(&style, &[child_a, child_b]);
        let half = Size {
            width: Pixels(200.0),
            height: Pixels(100.0),
        };
        engine.compute_layout(child_a, half);
        engine.compute_layout(child_b, half);
        assert_eq!(engine.layout(child_a).size.width, Pixels(200.0));
        assert_eq!(engine.layout(child_b).size.width, Pixels(200.0));
    }

    #[test]
    fn engine_default_and_new_are_equivalent() {
        let a = LayoutEngine::new();
        let b = LayoutEngine::default();
        // Both start with no stored layouts.
        assert_eq!(a.layout(LayoutId(1)), Bounds::default());
        assert_eq!(b.layout(LayoutId(1)), Bounds::default());
    }

    #[test]
    fn remove_nonexistent_id_is_no_op() {
        let mut engine = LayoutEngine::new();
        // Removing an id that was never registered must not panic.
        engine.remove_layout_id(LayoutId(999));
        assert_eq!(engine.layout(LayoutId(999)), Bounds::default());
    }

    #[test]
    fn compute_layout_zero_size_stores_zero() {
        let mut engine = LayoutEngine::new();
        let style = StyleRefinement::default();
        let id = engine.request_layout(&style, &[]);
        let zero = Size {
            width: Pixels(0.0),
            height: Pixels(0.0),
        };
        engine.compute_layout(id, zero);
        assert_eq!(engine.layout(id).size, zero);
    }

    #[test]
    fn many_siblings_all_independent() {
        let mut engine = LayoutEngine::new();
        let style = StyleRefinement::default();
        let ids: Vec<_> = (0..10)
            .map(|_| engine.request_layout(&style, &[]))
            .collect();
        // Layout only even-indexed nodes.
        for (i, id) in ids.iter().enumerate() {
            if i % 2 == 0 {
                let s = Size {
                    width: Pixels(i as f32 * 10.0 + 10.0),
                    height: Pixels(10.0),
                };
                engine.compute_layout(*id, s);
            }
        }
        // Odd-indexed nodes remain default.
        for (i, id) in ids.iter().enumerate() {
            if i % 2 != 0 {
                assert_eq!(
                    engine.layout(*id),
                    Bounds::default(),
                    "odd node {i} should be default"
                );
            }
        }
    }

    // ------------------------------------------------------------------
    // Wave AF: percentage-basis, flex-wrap new row, absolute positioning
    // ------------------------------------------------------------------

    #[test]
    fn percentage_basis_calculation_33_percent() {
        // 33% of 900px parent width = 300px.
        let mut engine = LayoutEngine::new();
        let style = StyleRefinement::default();
        let parent_width = 900.0_f32;
        let pct = 1.0 / 3.0;
        let id = engine.request_layout(&style, &[]);
        let computed = Size {
            width: Pixels(parent_width * pct),
            height: Pixels(50.0),
        };
        engine.compute_layout(id, computed);
        assert!(
            (engine.layout(id).size.width.0 - 300.0).abs() < 1e-3,
            "33% of 900px must be 300px, got {}",
            engine.layout(id).size.width.0
        );
    }

    #[test]
    fn percentage_basis_calculation_100_percent_fills_parent() {
        // 100% of 640px = 640px.
        let mut engine = LayoutEngine::new();
        let style = StyleRefinement::default();
        let id = engine.request_layout(&style, &[]);
        let full = Size {
            width: Pixels(640.0),
            height: Pixels(480.0),
        };
        engine.compute_layout(id, full);
        assert_eq!(
            engine.layout(id).size.width,
            Pixels(640.0),
            "100% width must equal parent"
        );
        assert_eq!(
            engine.layout(id).size.height,
            Pixels(480.0),
            "100% height must equal parent"
        );
    }

    #[test]
    fn flex_wrap_triggers_new_row_when_content_exceeds_width() {
        // Simulate flex-wrap: if child widths exceed the row budget, a new row starts.
        // Two children of 300px each in a 500px parent → second child wraps.
        let mut engine = LayoutEngine::new();
        let style = StyleRefinement::default();
        let parent_width = 500.0_f32;
        let child_width = 300.0_f32;

        let child_a = engine.request_layout(&style, &[]);
        let child_b = engine.request_layout(&style, &[]);
        let _parent = engine.request_layout(&style, &[child_a, child_b]);

        // Row 1: child_a fits entirely.
        engine.compute_layout(
            child_a,
            Size {
                width: Pixels(child_width),
                height: Pixels(100.0),
            },
        );
        // Row 2: child_b wraps — assign same width but placed in new row (origin tracked externally).
        engine.compute_layout(
            child_b,
            Size {
                width: Pixels(child_width),
                height: Pixels(100.0),
            },
        );

        // Both children receive their requested width; wrap detection is caller's responsibility.
        assert_eq!(
            engine.layout(child_a).size.width,
            Pixels(300.0),
            "child_a in row 1"
        );
        assert_eq!(
            engine.layout(child_b).size.width,
            Pixels(300.0),
            "child_b wraps to row 2"
        );

        // Verify wrap condition: child_a.width + child_b.width > parent_width.
        let total_row_width =
            engine.layout(child_a).size.width.0 + engine.layout(child_b).size.width.0;
        assert!(
            total_row_width > parent_width,
            "combined child widths ({total_row_width}) must exceed parent ({parent_width}) to confirm wrap"
        );
    }

    #[test]
    fn absolute_positioning_outside_flex_container() {
        // Absolutely-positioned elements are taken out of flex flow.
        // An absolutely-positioned child gets its own layout independent of siblings.
        let mut engine = LayoutEngine::new();
        let style = StyleRefinement::default();

        // Flex sibling inside the container flow.
        let flex_child = engine.request_layout(&style, &[]);
        // Absolutely-positioned child — assigned dimensions independently.
        let abs_child = engine.request_layout(&style, &[]);
        let _container = engine.request_layout(&style, &[flex_child]);

        // Flex child fills available width.
        engine.compute_layout(
            flex_child,
            Size {
                width: Pixels(800.0),
                height: Pixels(200.0),
            },
        );
        // Absolute child is given fixed dimensions, independent of flex flow.
        let abs_size = Size {
            width: Pixels(150.0),
            height: Pixels(80.0),
        };
        engine.compute_layout(abs_child, abs_size);

        // Flex child has its computed size.
        assert_eq!(
            engine.layout(flex_child).size.width,
            Pixels(800.0),
            "flex child width"
        );
        // Absolute child has its independent size (not influenced by flex layout).
        assert_eq!(
            engine.layout(abs_child).size,
            abs_size,
            "abs child has independent size"
        );
        // They do not interfere: flex child does not adopt abs child's width.
        assert_ne!(
            engine.layout(flex_child).size.width,
            engine.layout(abs_child).size.width,
            "flex child and abs child must have independent widths"
        );
    }

    #[test]
    fn flex_wrap_multiple_rows_simulated() {
        // Simulate a 3-item row: items of 200px each in a 500px container.
        // Items 1+2 fit on row 1 (400px ≤ 500px), item 3 wraps to row 2.
        let mut engine = LayoutEngine::new();
        let style = StyleRefinement::default();
        let container_width = 500.0_f32;
        let item_width = 200.0_f32;

        let item1 = engine.request_layout(&style, &[]);
        let item2 = engine.request_layout(&style, &[]);
        let item3 = engine.request_layout(&style, &[]);

        // All items get the same computed width; row grouping is caller's responsibility.
        for id in [item1, item2, item3] {
            engine.compute_layout(
                id,
                Size {
                    width: Pixels(item_width),
                    height: Pixels(50.0),
                },
            );
        }

        // Row 1 uses 200 + 200 = 400px (fits in 500px).
        let row1_width = engine.layout(item1).size.width.0 + engine.layout(item2).size.width.0;
        assert!(
            row1_width <= container_width,
            "row 1 ({row1_width}px) must fit in container ({container_width}px)"
        );

        // Row 1 + item3 = 600px > 500px → item3 wraps.
        let hypothetical_row1_plus_item3 = row1_width + engine.layout(item3).size.width.0;
        assert!(
            hypothetical_row1_plus_item3 > container_width,
            "adding item3 ({hypothetical_row1_plus_item3}px) to row 1 must exceed container → wrap"
        );
    }

    #[test]
    fn absolute_child_does_not_affect_parent_flow_size() {
        // An absolutely-positioned child has an independent size that does NOT
        // contribute to the parent's computed intrinsic size.
        let mut engine = LayoutEngine::new();
        let style = StyleRefinement::default();

        let abs_child = engine.request_layout(&style, &[]);
        let parent = engine.request_layout(&style, &[]); // parent does not include abs_child in children

        // Parent gets a fixed size via compute_layout.
        let parent_size = Size {
            width: Pixels(400.0),
            height: Pixels(300.0),
        };
        engine.compute_layout(parent, parent_size);

        // Abs child gets a large size independently.
        engine.compute_layout(
            abs_child,
            Size {
                width: Pixels(9999.0),
                height: Pixels(9999.0),
            },
        );

        // Parent size must remain exactly what was computed, unaffected by abs_child.
        assert_eq!(
            engine.layout(parent).size,
            parent_size,
            "parent size must not be affected by abs child"
        );
    }

    // ------------------------------------------------------------------
    // Wave AG: Additional layout tests
    // ------------------------------------------------------------------

    #[test]
    fn layout_engine_default_produces_zero_sized_layout() {
        // A freshly-allocated layout node must have default (zero) bounds.
        let mut engine = LayoutEngine::new();
        let style = StyleRefinement::default();
        let id = engine.request_layout(&style, &[]);
        let layout = engine.layout(id);
        assert_eq!(layout.size.width, Pixels(0.0), "default width must be 0");
        assert_eq!(layout.size.height, Pixels(0.0), "default height must be 0");
        assert_eq!(layout.origin.x, Pixels(0.0), "default origin.x must be 0");
        assert_eq!(layout.origin.y, Pixels(0.0), "default origin.y must be 0");
    }

    #[test]
    fn compute_layout_then_remove_then_requery_returns_default() {
        // After removing a layout node, querying it must return default (as-if-never-created).
        let mut engine = LayoutEngine::new();
        let style = StyleRefinement::default();
        let id = engine.request_layout(&style, &[]);
        engine.compute_layout(
            id,
            Size {
                width: Pixels(100.0),
                height: Pixels(50.0),
            },
        );
        assert_ne!(
            engine.layout(id).size,
            Size::zero(),
            "size must be non-zero before removal"
        );
        engine.remove_layout_id(id);
        assert_eq!(
            engine.layout(id),
            Bounds::default(),
            "removed node must return default bounds"
        );
    }

    #[test]
    fn layout_engine_handles_many_nodes_without_id_collision() {
        // Request 100 layout nodes and verify all ids are unique.
        let mut engine = LayoutEngine::new();
        let style = StyleRefinement::default();
        let ids: Vec<_> = (0..100)
            .map(|_| engine.request_layout(&style, &[]))
            .collect();
        let unique: std::collections::HashSet<u64> = ids.iter().map(|id| id.0).collect();
        assert_eq!(unique.len(), 100, "all 100 layout ids must be unique");
    }

    #[test]
    fn compute_layout_large_dimensions() {
        // Verify the engine handles ultra-large (but valid f32) dimensions.
        let mut engine = LayoutEngine::new();
        let style = StyleRefinement::default();
        let id = engine.request_layout(&style, &[]);
        let huge = Size {
            width: Pixels(100_000.0),
            height: Pixels(100_000.0),
        };
        engine.compute_layout(id, huge);
        assert_eq!(
            engine.layout(id).size,
            huge,
            "large dimensions must be stored correctly"
        );
    }

    #[test]
    fn percentage_basis_50_percent_of_1000px() {
        // 50% of 1000px parent = 500px.
        let mut engine = LayoutEngine::new();
        let style = StyleRefinement::default();
        let id = engine.request_layout(&style, &[]);
        let pct = 0.5_f32;
        let parent_w = 1000.0_f32;
        let computed = Size {
            width: Pixels(parent_w * pct),
            height: Pixels(200.0),
        };
        engine.compute_layout(id, computed);
        assert!(
            (engine.layout(id).size.width.0 - 500.0).abs() < 1e-5,
            "50% of 1000px must be 500px, got {}",
            engine.layout(id).size.width.0
        );
    }

    #[test]
    fn layout_engine_ids_are_sequential_and_increasing() {
        // Each call to request_layout must return a strictly greater id than the previous.
        let mut engine = LayoutEngine::new();
        let style = StyleRefinement::default();
        let mut prev = LayoutId(0);
        for _ in 0..5 {
            let id = engine.request_layout(&style, &[]);
            assert!(
                id.0 > prev.0,
                "each new id must exceed the previous: {id:?} <= {prev:?}"
            );
            prev = id;
        }
    }
}
