//! R-tree variant for assigning a stable `DrawOrder` to paint calls.
//!
//! Pattern replicated from Zed GPUI's `bounds_tree.rs` (MAX_CHILDREN=12).
//! Each leaf stores the next `DrawOrder`; internal nodes propagate `max_order`
//! so a query for "highest order that intersects `query`" prunes subtrees
//! whose `max_order` is lower than the running best.
//!
//! Ordering semantics (overlap-aware, not monotonic):
//! - The order assigned to a new rect is `topmost_intersecting(bounds) + 1`.
//! - Non-overlapping rects can share the same order — critical for GPU batch
//!   coalescing (e.g. 100 non-overlapping quads in one row → all order 1,
//!   grouped into one draw call).
//! - `topmost_intersecting` uses a `max_leaf` fast-path: if the globally
//!   highest-order leaf intersects the query, return its order in O(1).

use crate::geometry::Bounds;

const MAX_CHILDREN: usize = 12;

/// Draw order assigned to an inserted rect.
/// Non-overlapping rects MAY share the same order value.
pub type DrawOrder = u32;

#[derive(Clone, Copy, Debug)]
enum NodeKind {
    Leaf { order: DrawOrder },
    Internal { children: NodeChildren },
}

#[derive(Clone, Copy, Debug)]
struct NodeChildren {
    items: [u32; MAX_CHILDREN],
    len: u8,
}

impl NodeChildren {
    fn new() -> Self {
        Self {
            items: [0; MAX_CHILDREN],
            len: 0,
        }
    }

    fn push(&mut self, id: u32) -> bool {
        if (self.len as usize) >= MAX_CHILDREN {
            return false;
        }
        self.items[self.len as usize] = id;
        self.len += 1;
        true
    }

    fn as_slice(&self) -> &[u32] {
        &self.items[..self.len as usize]
    }
}

#[derive(Clone, Copy, Debug)]
struct Node {
    bounds: Bounds<i32>,
    max_order: DrawOrder,
    kind: NodeKind,
}

/// Spatial tree that assigns an overlap-aware `DrawOrder` to each inserted
/// rectangle.
///
/// The order of a new rect equals `topmost_intersecting(new_rect) + 1`.
/// Non-overlapping rects can share the same order, enabling GPU batch
/// coalescing. Querying any rectangle returns the **highest** order whose
/// bounds intersect the query (i.e. "what's painted on top here?").
#[derive(Debug, Default)]
pub struct BoundsTree {
    nodes: Vec<Node>,
    root: Option<u32>,
    /// Index of the leaf with the globally highest order (fast-path for
    /// `topmost_intersecting`).
    max_leaf: Option<u32>,
}

impl BoundsTree {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn clear(&mut self) {
        self.nodes.clear();
        self.root = None;
        self.max_leaf = None;
    }

    pub fn len(&self) -> usize {
        self.nodes.iter().filter(|n| matches!(n.kind, NodeKind::Leaf { .. })).count()
    }

    pub fn is_empty(&self) -> bool {
        self.root.is_none()
    }

    /// Insert a rectangle; return its assigned draw order.
    ///
    /// Order = `topmost_intersecting(bounds) + 1`, or 1 when nothing
    /// intersects. Non-overlapping rects will receive the same order (1 when
    /// the tree is otherwise empty), which allows GPU batching.
    ///
    /// # Panics
    /// Panics on `u32` overflow (> 4 billion overlapping stacking layers).
    pub fn insert(&mut self, bounds: Bounds<i32>) -> DrawOrder {
        let order = self
            .topmost_intersecting(bounds)
            .map_or(1, |o| o.checked_add(1).expect("BoundsTree order overflow"));

        let leaf_id = self.push_node(Node {
            bounds,
            max_order: order,
            kind: NodeKind::Leaf { order },
        });

        // Update max_leaf: track the leaf with the globally highest order for
        // the O(1) fast-path in topmost_intersecting.
        self.max_leaf = match self.max_leaf {
            None => Some(leaf_id),
            Some(old_id) if self.nodes[old_id as usize].max_order < order => Some(leaf_id),
            some => some,
        };

        match self.root {
            None => self.root = Some(leaf_id),
            Some(root_id) => {
                self.insert_into(root_id, leaf_id);
                self.update_max_order(root_id);
            }
        }
        order
    }

    fn insert_into(&mut self, node_id: u32, child: u32) {
        let kind = self.nodes[node_id as usize].kind;
        let child_bounds = self.nodes[child as usize].bounds;
        match kind {
            NodeKind::Leaf { .. } => {
                // Promote: create new internal node with old + child as children.
                let old = node_id as usize;
                let old_bounds = self.nodes[old].bounds;
                let old_max = self.nodes[old].max_order;
                let combined = rect_union(old_bounds, child_bounds);
                let max_order = old_max.max(self.nodes[child as usize].max_order);
                // Relocate the leaf to a new slot so node_id becomes the internal.
                let relocated = self.push_node(Node {
                    bounds: old_bounds,
                    max_order: old_max,
                    kind,
                });
                let mut children = NodeChildren::new();
                children.push(relocated);
                children.push(child);
                self.nodes[old].bounds = combined;
                self.nodes[old].max_order = max_order;
                self.nodes[old].kind = NodeKind::Internal { children };
                // Fix max_leaf pointer if we relocated.
                if self.max_leaf == Some(node_id) {
                    self.max_leaf = Some(relocated.max(child));
                }
            }
            NodeKind::Internal { mut children } => {
                if !children.push(child) {
                    // Overflow: push child into an internal sibling (simple overflow bucket).
                    // Replace the last child with a new internal holding (last, child).
                    let last_id = children.items[MAX_CHILDREN - 1];
                    let last = self.nodes[last_id as usize];
                    let combined = rect_union(last.bounds, child_bounds);
                    let max_order =
                        last.max_order.max(self.nodes[child as usize].max_order);
                    let mut grand = NodeChildren::new();
                    grand.push(last_id);
                    grand.push(child);
                    let internal_id = self.push_node(Node {
                        bounds: combined,
                        max_order,
                        kind: NodeKind::Internal { children: grand },
                    });
                    children.items[MAX_CHILDREN - 1] = internal_id;
                }
                // Read child's max_order first to avoid aliasing the mutable borrow below.
                let child_max_order = self.nodes[child as usize].max_order;
                let node = &mut self.nodes[node_id as usize];
                node.bounds = rect_union(node.bounds, child_bounds);
                node.max_order = node.max_order.max(child_max_order);
                node.kind = NodeKind::Internal { children };
            }
        }
    }

    fn update_max_order(&mut self, node_id: u32) {
        let kind = self.nodes[node_id as usize].kind;
        if let NodeKind::Internal { children } = kind {
            let mut bounds = self.nodes[children.items[0] as usize].bounds;
            let mut max_order = 0;
            for &c in children.as_slice() {
                let n = &self.nodes[c as usize];
                bounds = rect_union(bounds, n.bounds);
                if n.max_order > max_order {
                    max_order = n.max_order;
                }
            }
            let node = &mut self.nodes[node_id as usize];
            node.bounds = bounds;
            node.max_order = max_order;
        }
    }

    fn push_node(&mut self, node: Node) -> u32 {
        let id = self.nodes.len() as u32;
        self.nodes.push(node);
        id
    }

    /// Return the highest `DrawOrder` whose bounds intersect `query`, if any.
    ///
    /// Fast-path: if the globally highest-order leaf (`max_leaf`) intersects
    /// `query`, return its order immediately in O(1) without walking the tree.
    ///
    /// Slow-path uses an explicit `Vec<u32>` stack rather than recursion to
    /// avoid stack overflows on degenerate overflow-bucket chains (thousands of
    /// deeply nested internal nodes).
    pub fn topmost_intersecting(&self, query: Bounds<i32>) -> Option<DrawOrder> {
        let root = self.root?;

        // Fast-path: the globally-highest leaf is likely to be on top.
        // If it intersects `query` its order is the answer by definition.
        if let Some(max_id) = self.max_leaf {
            let max_node = &self.nodes[max_id as usize];
            if max_node.bounds.intersects(&query) {
                return Some(max_node.max_order);
            }
        }

        // Slow-path: explicit stack walk — no recursion, no stack-overflow risk.
        let mut best: Option<DrawOrder> = None;
        let mut stack: Vec<u32> = Vec::with_capacity(32);
        stack.push(root);
        while let Some(id) = stack.pop() {
            let node = &self.nodes[id as usize];
            // Prune: this subtree cannot beat the current best.
            if let Some(b) = best {
                if node.max_order <= b {
                    continue;
                }
            }
            if !node.bounds.intersects(&query) {
                continue;
            }
            match node.kind {
                NodeKind::Leaf { order } => {
                    if best.map_or(true, |b| order > b) {
                        best = Some(order);
                    }
                }
                NodeKind::Internal { children } => {
                    for &c in children.as_slice() {
                        stack.push(c);
                    }
                }
            }
        }
        best
    }
}

fn rect_union(a: Bounds<i32>, b: Bounds<i32>) -> Bounds<i32> {
    use crate::geometry::{Point, Size};
    let ox = a.origin.x.min(b.origin.x);
    let oy = a.origin.y.min(b.origin.y);
    let rx = a.right().max(b.right());
    let by = a.bottom().max(b.bottom());
    Bounds {
        origin: Point::new(ox, oy),
        size: Size::new(rx - ox, by - oy),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::geometry::{Point, Size};

    fn rect(x: i32, y: i32, w: i32, h: i32) -> Bounds<i32> {
        Bounds::new(Point::new(x, y), Size::new(w, h))
    }

    #[test]
    fn empty_tree_queries_return_none() {
        let t = BoundsTree::new();
        assert!(t.topmost_intersecting(rect(0, 0, 10, 10)).is_none());
    }

    /// Non-overlapping rects must be allowed to share the same order so the
    /// GPU renderer can batch them into a single draw call.
    #[test]
    fn non_overlapping_rects_can_reuse_order() {
        let mut t = BoundsTree::new();
        let a = t.insert(rect(0, 0, 10, 10));
        let b = t.insert(rect(20, 20, 10, 10));
        let c = t.insert(rect(40, 40, 10, 10));
        // All three are non-overlapping; all receive order 1 (no intersection
        // with any existing rect → topmost_intersecting returns None → order = 1).
        assert_eq!(a, 1, "first rect should be order 1");
        assert_eq!(b, 1, "non-overlapping rect should reuse order 1");
        assert_eq!(c, 1, "non-overlapping rect should reuse order 1");
    }

    #[test]
    fn topmost_intersecting_finds_newest() {
        let mut t = BoundsTree::new();
        t.insert(rect(0, 0, 100, 100)); // order 1
        t.insert(rect(0, 0, 50, 50));   // order 2 (overlaps first)
        t.insert(rect(200, 200, 10, 10)); // order 1 (no overlap with above)
        assert_eq!(t.topmost_intersecting(rect(10, 10, 5, 5)), Some(2));
        assert_eq!(t.topmost_intersecting(rect(205, 205, 1, 1)), Some(1));
        assert!(t.topmost_intersecting(rect(500, 500, 1, 1)).is_none());
    }

    #[test]
    fn many_insertions_still_queryable() {
        let mut t = BoundsTree::new();
        for i in 0..100 {
            t.insert(rect(i, i, 10, 10));
        }
        // Each successive rect at (i,i,10,10) overlaps the previous ones that
        // are still in range. The topmost at (50,50) will be one of the rects
        // near index 50 (they overlap each other and build up order).
        let found = t.topmost_intersecting(rect(50, 50, 1, 1));
        assert!(found.is_some(), "should find something");
    }

    // ── New tests required by the spec ────────────────────────────────────────

    /// Overlapping rects must receive strictly increasing orders so z-ordering
    /// is correct and later paints appear on top.
    #[test]
    fn overlapping_rects_get_strictly_increasing_orders() {
        let mut t = BoundsTree::new();
        let a = t.insert(rect(0, 0, 100, 100));
        let b = t.insert(rect(10, 10, 50, 50)); // overlaps a
        let c = t.insert(rect(20, 20, 30, 30)); // overlaps a and b
        assert!(b > a, "second overlapping rect must have higher order than first");
        assert!(c > b, "third overlapping rect must have higher order than second");
    }

    /// Non-overlapping rects across a large batch must all receive the same
    /// base order so the GPU can coalesce them into one draw call.
    #[test]
    fn large_batch_of_non_overlapping_rects_all_share_order() {
        let mut t = BoundsTree::new();
        // 50 rects placed side-by-side with 100px spacing — no overlaps.
        let orders: Vec<DrawOrder> = (0..50).map(|i| t.insert(rect(i * 100, 0, 90, 90))).collect();
        // All should have order 1 (no prior intersection for any of them).
        let first = orders[0];
        for (i, &o) in orders.iter().enumerate() {
            assert_eq!(o, first, "rect {i} should share order {first} but got {o}");
        }
    }

    /// A mixed sequence: first a layer of non-overlapping rects (all order 1),
    /// then one rect overlapping all of them (order 2).
    #[test]
    fn mixed_overlap_sequence() {
        let mut t = BoundsTree::new();
        // Three non-overlapping rects → all order 1.
        let a = t.insert(rect(0, 0, 10, 10));
        let b = t.insert(rect(20, 0, 10, 10));
        let c = t.insert(rect(40, 0, 10, 10));
        assert_eq!(a, 1);
        assert_eq!(b, 1);
        assert_eq!(c, 1);

        // A wide rect that covers all three → order must be 2.
        let d = t.insert(rect(0, 0, 50, 10));
        assert_eq!(d, 2, "rect overlapping all existing order-1 rects should be order 2");
    }

    /// Inserting 100 000 overlapping rects all at the same point creates a
    /// maximally degenerate overflow-bucket chain. The explicit-stack
    /// `topmost_intersecting` must return a result without stack-overflowing.
    #[test]
    fn deep_tree_does_not_stack_overflow() {
        let mut t = BoundsTree::new();
        // All rects share the same single point — every insert overlaps every
        // previous one, driving order up and creating a deeply nested internal
        // node chain via the overflow-bucket path.
        let n = 100_000u32;
        for _ in 0..n {
            t.insert(rect(0, 0, 1, 1));
        }
        // The topmost order must equal n (each insert adds 1 over the previous).
        let found = t.topmost_intersecting(rect(0, 0, 1, 1));
        assert_eq!(found, Some(n), "topmost order should equal number of inserted rects");
    }
}
