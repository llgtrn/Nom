//! Block-tree traversal helpers.
//!
//! All queries take a caller-supplied `store_fn` closure of type
//! `Fn(BlockId) -> Option<&BlockModel<Props>>` so we don't own storage.
//! Returns traversal results without allocating unless necessary.
#![deny(unsafe_code)]

use crate::block_model::{BlockId, BlockModel};
use std::collections::{HashSet, VecDeque};

/// Depth-first in-order descendant collection (self + recurse children).
pub fn descendants_of<'a, P, F>(root: BlockId, store_fn: F) -> Vec<BlockId>
where
    F: Fn(BlockId) -> Option<&'a BlockModel<P>>,
    P: 'a,
{
    let mut out = Vec::new();
    let mut stack = vec![root];
    while let Some(id) = stack.pop() {
        out.push(id);
        if let Some(block) = store_fn(id) {
            // Push children in reverse so leftmost visits first (in-order).
            for child in block.children.iter().rev() {
                stack.push(*child);
            }
        }
    }
    out
}

/// Breadth-first traversal starting at `root`, returning IDs level-by-level.
pub fn breadth_first<'a, P, F>(root: BlockId, store_fn: F) -> Vec<BlockId>
where
    F: Fn(BlockId) -> Option<&'a BlockModel<P>>,
    P: 'a,
{
    let mut out = Vec::new();
    let mut queue = VecDeque::new();
    queue.push_back(root);
    while let Some(id) = queue.pop_front() {
        out.push(id);
        if let Some(block) = store_fn(id) {
            for child in &block.children {
                queue.push_back(*child);
            }
        }
    }
    out
}

/// Walk up from `leaf` by finding, at each step, the first block whose
/// `children` list contains the current id.  Stops when no parent is found.
pub fn ancestors_of<'a, P, F>(leaf: BlockId, all_ids: &[BlockId], store_fn: F) -> Vec<BlockId>
where
    F: Fn(BlockId) -> Option<&'a BlockModel<P>>,
    P: 'a,
{
    let mut out = Vec::new();
    let mut current = leaf;
    loop {
        let parent = all_ids.iter().copied().find(|candidate| {
            if *candidate == current {
                return false;
            }
            store_fn(*candidate)
                .map(|b| b.children.contains(&current))
                .unwrap_or(false)
        });
        match parent {
            Some(p) => {
                out.push(p);
                current = p;
            }
            None => break,
        }
    }
    out
}

/// Number of edges from `root` to `target` (None if not reachable).
pub fn depth_from<'a, P, F>(root: BlockId, target: BlockId, store_fn: F) -> Option<u32>
where
    F: Fn(BlockId) -> Option<&'a BlockModel<P>>,
    P: 'a,
{
    if root == target {
        return Some(0);
    }
    let mut queue = VecDeque::new();
    let mut visited = HashSet::new();
    queue.push_back((root, 0u32));
    visited.insert(root);
    while let Some((id, depth)) = queue.pop_front() {
        if let Some(block) = store_fn(id) {
            for child in &block.children {
                if *child == target {
                    return Some(depth + 1);
                }
                if visited.insert(*child) {
                    queue.push_back((*child, depth + 1));
                }
            }
        }
    }
    None
}

/// Count all reachable descendants from `root` (does NOT include root itself).
pub fn subtree_size<'a, P, F>(root: BlockId, store_fn: F) -> usize
where
    F: Fn(BlockId) -> Option<&'a BlockModel<P>>,
    P: 'a,
{
    let all = descendants_of(root, store_fn);
    all.len().saturating_sub(1)
}

/// Is `ancestor` an ancestor of `descendant`?  True if descendant is in the
/// subtree rooted at ancestor and ancestor != descendant.
pub fn is_ancestor_of<'a, P, F>(ancestor: BlockId, descendant: BlockId, store_fn: F) -> bool
where
    F: Fn(BlockId) -> Option<&'a BlockModel<P>>,
    P: 'a,
{
    if ancestor == descendant {
        return false;
    }
    descendants_of(ancestor, store_fn)
        .into_iter()
        .any(|id| id == descendant)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::block_model::BlockModel;
    use std::collections::HashMap;

    /// Build the test tree:
    /// ```text
    /// A (1)
    /// ├── B (2)
    /// │   ├── D (4)
    /// │   └── E (5)
    /// └── C (3)
    ///     └── F (6)
    /// ```
    fn make_tree() -> HashMap<BlockId, BlockModel<()>> {
        let mut map: HashMap<BlockId, BlockModel<()>> = HashMap::new();

        let mut a = BlockModel::new(1, "nom:test", ());
        a.children = vec![2, 3];
        map.insert(1, a);

        let mut b = BlockModel::new(2, "nom:test", ());
        b.children = vec![4, 5];
        map.insert(2, b);

        let mut c = BlockModel::new(3, "nom:test", ());
        c.children = vec![6];
        map.insert(3, c);

        map.insert(4, BlockModel::new(4, "nom:test", ()));
        map.insert(5, BlockModel::new(5, "nom:test", ()));
        map.insert(6, BlockModel::new(6, "nom:test", ()));

        map
    }

    #[test]
    fn descendants_of_root_returns_full_in_order() {
        let tree = make_tree();
        let store = |id: BlockId| tree.get(&id);
        assert_eq!(descendants_of(1, store), vec![1, 2, 4, 5, 3, 6]);
    }

    #[test]
    fn descendants_of_leaf_returns_self_only() {
        let tree = make_tree();
        let store = |id: BlockId| tree.get(&id);
        assert_eq!(descendants_of(4, store), vec![4]);
    }

    #[test]
    fn breadth_first_from_root_level_order() {
        let tree = make_tree();
        let store = |id: BlockId| tree.get(&id);
        assert_eq!(breadth_first(1, store), vec![1, 2, 3, 4, 5, 6]);
    }

    #[test]
    fn breadth_first_from_mid_node() {
        let tree = make_tree();
        let store = |id: BlockId| tree.get(&id);
        assert_eq!(breadth_first(2, store), vec![2, 4, 5]);
    }

    #[test]
    fn ancestors_of_leaf_d_returns_b_then_a() {
        let tree = make_tree();
        let store = |id: BlockId| tree.get(&id);
        let all: Vec<BlockId> = (1..=6).collect();
        assert_eq!(ancestors_of(4, &all, store), vec![2, 1]);
    }

    #[test]
    fn ancestors_of_root_returns_empty() {
        let tree = make_tree();
        let store = |id: BlockId| tree.get(&id);
        let all: Vec<BlockId> = (1..=6).collect();
        assert_eq!(ancestors_of(1, &all, store), vec![]);
    }

    #[test]
    fn depth_from_root_to_d_is_two() {
        let tree = make_tree();
        let store = |id: BlockId| tree.get(&id);
        assert_eq!(depth_from(1, 4, store), Some(2));
    }

    #[test]
    fn depth_from_self_is_zero() {
        let tree = make_tree();
        let store = |id: BlockId| tree.get(&id);
        assert_eq!(depth_from(1, 1, store), Some(0));
    }

    #[test]
    fn depth_from_sibling_branch_is_none() {
        let tree = make_tree();
        let store = |id: BlockId| tree.get(&id);
        assert_eq!(depth_from(4, 6, store), None);
    }

    #[test]
    fn subtree_size_of_root_is_five() {
        let tree = make_tree();
        let store = |id: BlockId| tree.get(&id);
        assert_eq!(subtree_size(1, store), 5);
    }

    #[test]
    fn subtree_size_of_leaf_is_zero() {
        let tree = make_tree();
        let store = |id: BlockId| tree.get(&id);
        assert_eq!(subtree_size(4, store), 0);
    }

    #[test]
    fn is_ancestor_of_root_over_d_is_true() {
        let tree = make_tree();
        let store = |id: BlockId| tree.get(&id);
        assert!(is_ancestor_of(1, 4, store));
    }

    #[test]
    fn is_ancestor_of_d_over_root_is_false() {
        let tree = make_tree();
        let store = |id: BlockId| tree.get(&id);
        assert!(!is_ancestor_of(4, 1, store));
    }

    #[test]
    fn is_ancestor_of_self_is_false() {
        let tree = make_tree();
        let store = |id: BlockId| tree.get(&id);
        assert!(!is_ancestor_of(1, 1, store));
    }
}
