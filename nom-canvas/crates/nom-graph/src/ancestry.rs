/// Ordered list of ancestors from a node up to the root.
/// The first element is the start node; the last element is the root.
#[derive(Debug, Clone, Default)]
pub struct AncestryChain {
    pub nodes: Vec<u64>,
}

impl AncestryChain {
    pub fn new() -> Self {
        Self { nodes: Vec::new() }
    }

    pub fn push(&mut self, node_id: u64) {
        self.nodes.push(node_id);
    }

    /// Number of nodes in the chain (start node inclusive).
    pub fn depth(&self) -> usize {
        self.nodes.len()
    }

    pub fn contains(&self, node_id: u64) -> bool {
        self.nodes.contains(&node_id)
    }

    /// The root node is the last element in the chain.
    pub fn root(&self) -> Option<u64> {
        self.nodes.last().copied()
    }

    pub fn iter(&self) -> impl Iterator<Item = u64> + '_ {
        self.nodes.iter().copied()
    }
}

// ---------------------------------------------------------------------------

/// A map from child node → parent node.
#[derive(Debug, Clone, Default)]
pub struct ParentMap {
    parents: std::collections::HashMap<u64, u64>,
}

impl ParentMap {
    pub fn new() -> Self {
        Self {
            parents: std::collections::HashMap::new(),
        }
    }

    pub fn set_parent(&mut self, node: u64, parent: u64) {
        self.parents.insert(node, parent);
    }

    pub fn parent_of(&self, node: u64) -> Option<u64> {
        self.parents.get(&node).copied()
    }

    /// All nodes whose direct parent is `parent`.
    pub fn children_of(&self, parent: u64) -> Vec<u64> {
        self.parents
            .iter()
            .filter_map(|(&child, &p)| if p == parent { Some(child) } else { None })
            .collect()
    }

    /// Number of (child → parent) entries in the map.
    pub fn len(&self) -> usize {
        self.parents.len()
    }

    pub fn is_empty(&self) -> bool {
        self.parents.is_empty()
    }
}

// ---------------------------------------------------------------------------

/// Queries ancestor / descendant relationships backed by a `ParentMap`.
pub struct AncestorQuery {
    map: ParentMap,
}

impl AncestorQuery {
    pub fn new(map: ParentMap) -> Self {
        Self { map }
    }

    /// Walk parent links from `start` until there is no parent, building the
    /// chain in order (start → … → root).
    pub fn chain_to_root(&self, start: u64) -> AncestryChain {
        let mut chain = AncestryChain::new();
        let mut current = start;
        // Guard against cycles: stop if we visit the same node twice.
        let mut visited = std::collections::HashSet::new();
        loop {
            if !visited.insert(current) {
                break; // cycle detected
            }
            chain.push(current);
            match self.map.parent_of(current) {
                Some(parent) => current = parent,
                None => break,
            }
        }
        chain
    }

    /// Returns `true` when `ancestor` appears in the chain from `descendant`
    /// to the root.
    pub fn is_ancestor_of(&self, ancestor: u64, descendant: u64) -> bool {
        self.chain_to_root(descendant).contains(ancestor)
    }

    /// Depth of `node` = length of its chain to root (the node itself counts
    /// as depth 1; a root with no parent has depth 1).
    pub fn depth(&self, node: u64) -> usize {
        self.chain_to_root(node).depth()
    }
}

// ---------------------------------------------------------------------------

/// A pre-collected set of all descendants of a given root node (BFS order,
/// root not included).
pub struct DescendantIter {
    pub descendants: Vec<u64>,
}

impl DescendantIter {
    /// BFS from `root` using `ParentMap::children_of`.  The root itself is
    /// *not* included in the result.
    pub fn collect(map: &ParentMap, root: u64) -> DescendantIter {
        let mut descendants = Vec::new();
        let mut queue = std::collections::VecDeque::new();
        queue.push_back(root);
        let mut seen = std::collections::HashSet::new();
        seen.insert(root);
        while let Some(node) = queue.pop_front() {
            let children = map.children_of(node);
            for child in children {
                if seen.insert(child) {
                    descendants.push(child);
                    queue.push_back(child);
                }
            }
        }
        DescendantIter { descendants }
    }

    pub fn len(&self) -> usize {
        self.descendants.len()
    }

    pub fn is_empty(&self) -> bool {
        self.descendants.is_empty()
    }

    pub fn contains(&self, node: u64) -> bool {
        self.descendants.contains(&node)
    }
}

// ---------------------------------------------------------------------------

#[cfg(test)]
mod ancestry_tests {
    use super::*;

    fn build_map() -> ParentMap {
        //  root(1) → child(2) → grandchild(3)
        //          ↘ child(4)
        let mut m = ParentMap::new();
        m.set_parent(2, 1);
        m.set_parent(3, 2);
        m.set_parent(4, 1);
        m
    }

    #[test]
    fn test_parent_map_set_and_get() {
        let mut m = ParentMap::new();
        m.set_parent(10, 5);
        assert_eq!(m.parent_of(10), Some(5));
        assert_eq!(m.parent_of(5), None);
        assert_eq!(m.len(), 1);
    }

    #[test]
    fn test_parent_map_children_of() {
        let m = build_map();
        let mut children = m.children_of(1);
        children.sort();
        assert_eq!(children, vec![2, 4]);
        assert_eq!(m.children_of(3), vec![]);
    }

    #[test]
    fn test_ancestor_query_chain_length() {
        let q = AncestorQuery::new(build_map());
        // grandchild(3) → child(2) → root(1): length 3
        assert_eq!(q.chain_to_root(3).depth(), 3);
        // root(1) has no parent: length 1
        assert_eq!(q.chain_to_root(1).depth(), 1);
    }

    #[test]
    fn test_ancestor_query_is_ancestor_true() {
        let q = AncestorQuery::new(build_map());
        assert!(q.is_ancestor_of(1, 3)); // 1 is ancestor of 3
        assert!(q.is_ancestor_of(2, 3)); // 2 is direct parent of 3
        assert!(q.is_ancestor_of(1, 4));
    }

    #[test]
    fn test_ancestor_query_is_ancestor_false() {
        let q = AncestorQuery::new(build_map());
        assert!(!q.is_ancestor_of(3, 1)); // 3 is not ancestor of 1
        assert!(!q.is_ancestor_of(4, 3)); // 4 and 3 are siblings
    }

    #[test]
    fn test_ancestor_query_depth() {
        let q = AncestorQuery::new(build_map());
        assert_eq!(q.depth(1), 1);
        assert_eq!(q.depth(2), 2);
        assert_eq!(q.depth(3), 3);
        assert_eq!(q.depth(4), 2);
    }

    #[test]
    fn test_descendant_iter_collect_single_level() {
        let m = build_map();
        let d = DescendantIter::collect(&m, 2);
        // Only grandchild(3) is below child(2)
        assert_eq!(d.len(), 1);
        assert!(d.contains(3));
    }

    #[test]
    fn test_descendant_iter_collect_multi_level() {
        let m = build_map();
        let d = DescendantIter::collect(&m, 1);
        // children 2, 4 and grandchild 3
        assert_eq!(d.len(), 3);
        assert!(d.contains(2));
        assert!(d.contains(3));
        assert!(d.contains(4));
        assert!(!d.contains(1)); // root not included
    }

    #[test]
    fn test_ancestry_chain_root() {
        let q = AncestorQuery::new(build_map());
        let chain = q.chain_to_root(3);
        // chain: [3, 2, 1]; root is last = 1
        assert_eq!(chain.root(), Some(1));
        let chain_root = q.chain_to_root(1);
        assert_eq!(chain_root.root(), Some(1));
    }
}
