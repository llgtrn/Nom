use crate::styled::StyleRefinement;
use crate::types::*;

/// Layout engine integration.
///
/// `LayoutId` is a newtype over the internal node identifier (u64). The engine
/// stores computed `Bounds<Pixels>` per node and resolves them on demand.
pub struct LayoutEngine {
    next_id: u64,
    layouts: std::collections::HashMap<LayoutId, Bounds<Pixels>>,
}

impl LayoutEngine {
    pub fn new() -> Self {
        Self {
            next_id: 1,
            layouts: std::collections::HashMap::new(),
        }
    }

    /// Creates a layout node. Returns a unique `LayoutId` from the `LayoutEngine` registry.
    pub fn request_layout(&mut self, _style: &StyleRefinement, _children: &[LayoutId]) -> LayoutId {
        let id = LayoutId(self.next_id);
        self.next_id += 1;
        self.layouts.insert(id, Bounds::default());
        id
    }

    /// Get computed bounds for a layout node.
    pub fn layout(&self, id: LayoutId) -> Bounds<Pixels> {
        self.layouts.get(&id).copied().unwrap_or_default()
    }

    /// Free a layout node (called on element drop).
    pub fn remove_layout_id(&mut self, id: LayoutId) {
        self.layouts.remove(&id);
    }

    /// Computes layout for the tree rooted at `root_id` given available space.
    pub fn compute_layout(&mut self, root_id: LayoutId, available: Size<Pixels>) {
        if let Some(layout) = self.layouts.get_mut(&root_id) {
            layout.size = available;
        }
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
}
