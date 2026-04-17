//! R-tree variant for assigning a stable `DrawOrder` to paint calls.
//!
//! Pattern replicated from Zed GPUI's `bounds_tree.rs` (MAX_CHILDREN=12).
//! Each leaf stores the next `DrawOrder`; internal nodes propagate `max_order`
//! so a query for "highest order that intersects `query`" prunes subtrees
//! whose `max_order` is lower than the running best.

use crate::geometry::Bounds;

const MAX_CHILDREN: usize = 12;

/// Monotonically increasing draw order assigned to each inserted rect.
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

/// Spatial tree that assigns a monotonically-increasing `DrawOrder` to each
/// inserted rectangle. Querying any rectangle returns the **highest** order
/// whose bounds intersect the query (i.e. "what's painted on top here?").
#[derive(Debug, Default)]
pub struct BoundsTree {
    nodes: Vec<Node>,
    root: Option<u32>,
    max_leaf: Option<u32>,
    next_order: DrawOrder,
}

impl BoundsTree {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn clear(&mut self) {
        self.nodes.clear();
        self.root = None;
        self.max_leaf = None;
        self.next_order = 0;
    }

    pub fn len(&self) -> usize {
        self.nodes.iter().filter(|n| matches!(n.kind, NodeKind::Leaf { .. })).count()
    }

    pub fn is_empty(&self) -> bool {
        self.root.is_none()
    }

    /// Insert a rectangle; return its assigned draw order.
    pub fn insert(&mut self, bounds: Bounds<i32>) -> DrawOrder {
        let order = self.next_order;
        self.next_order = self.next_order.saturating_add(1);
        let leaf_id = self.push_node(Node {
            bounds,
            max_order: order,
            kind: NodeKind::Leaf { order },
        });
        self.max_leaf = Some(leaf_id);
        match self.root {
            None => self.root = Some(leaf_id),
            Some(root_id) => {
                // Simple strategy: if root is a leaf, promote it to an internal node;
                // otherwise append to root's child list (split when full).
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
                // Fix root's max_leaf pointer if we relocated.
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
    pub fn topmost_intersecting(&self, query: Bounds<i32>) -> Option<DrawOrder> {
        let root = self.root?;
        let mut best: Option<DrawOrder> = None;
        self.walk(root, query, &mut best);
        best
    }

    fn walk(&self, node_id: u32, query: Bounds<i32>, best: &mut Option<DrawOrder>) {
        let node = &self.nodes[node_id as usize];
        if let Some(b) = best {
            if node.max_order <= *b {
                return;
            }
        }
        if !node.bounds.intersects(&query) {
            return;
        }
        match node.kind {
            NodeKind::Leaf { order } => {
                if best.map_or(true, |b| order > b) {
                    *best = Some(order);
                }
            }
            NodeKind::Internal { children } => {
                for &c in children.as_slice() {
                    self.walk(c, query, best);
                }
            }
        }
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

    #[test]
    fn insertion_order_is_monotonic() {
        let mut t = BoundsTree::new();
        assert_eq!(t.insert(rect(0, 0, 10, 10)), 0);
        assert_eq!(t.insert(rect(20, 20, 10, 10)), 1);
        assert_eq!(t.insert(rect(40, 40, 10, 10)), 2);
    }

    #[test]
    fn topmost_intersecting_finds_newest() {
        let mut t = BoundsTree::new();
        t.insert(rect(0, 0, 100, 100)); // 0
        t.insert(rect(0, 0, 50, 50)); // 1 (overlaps)
        t.insert(rect(200, 200, 10, 10)); // 2 (doesn't overlap)
        assert_eq!(t.topmost_intersecting(rect(10, 10, 5, 5)), Some(1));
        assert_eq!(t.topmost_intersecting(rect(205, 205, 1, 1)), Some(2));
        assert!(t.topmost_intersecting(rect(500, 500, 1, 1)).is_none());
    }

    #[test]
    fn many_insertions_still_queryable() {
        let mut t = BoundsTree::new();
        for i in 0..100 {
            t.insert(rect(i, i, 10, 10));
        }
        // Query in the middle: should find the topmost rect whose origin <= 50.
        let found = t.topmost_intersecting(rect(50, 50, 1, 1)).unwrap();
        assert!(found >= 41 && found <= 50);
    }
}
