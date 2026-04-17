//! Z-order helpers for canvas elements.
//!
//! Ordering is represented as a `Vec<ElementId>` where index 0 is back-most
//! and index len-1 is front-most.  All helpers preserve the invariant:
//! every ElementId appears exactly once.
#![deny(unsafe_code)]

use crate::element::ElementId;

/// Move `id` to the back (index 0).  Returns true if `id` existed.
pub fn send_to_back(order: &mut Vec<ElementId>, id: ElementId) -> bool {
    let Some(pos) = order.iter().position(|x| *x == id) else { return false };
    let v = order.remove(pos);
    order.insert(0, v);
    true
}

/// Move `id` to the front (index len-1).  Returns true if `id` existed.
pub fn bring_to_front(order: &mut Vec<ElementId>, id: ElementId) -> bool {
    let Some(pos) = order.iter().position(|x| *x == id) else { return false };
    let v = order.remove(pos);
    order.push(v);
    true
}

/// Swap with the next element above (higher index).  Returns true if moved.
pub fn raise(order: &mut Vec<ElementId>, id: ElementId) -> bool {
    let Some(pos) = order.iter().position(|x| *x == id) else { return false };
    if pos + 1 >= order.len() { return false; }
    order.swap(pos, pos + 1);
    true
}

/// Swap with the next element below (lower index).  Returns true if moved.
pub fn lower(order: &mut Vec<ElementId>, id: ElementId) -> bool {
    let Some(pos) = order.iter().position(|x| *x == id) else { return false };
    if pos == 0 { return false; }
    order.swap(pos, pos - 1);
    true
}

/// Insert `id` immediately above `above_of` (i.e. at index of above_of + 1).
/// If `above_of` not present, appends to the end.  Returns the final index.
pub fn insert_above(order: &mut Vec<ElementId>, id: ElementId, above_of: ElementId) -> usize {
    // Remove `id` first if already present so that re-lookup of above_of is correct.
    let _ = remove(order, id);
    let Some(pos) = order.iter().position(|x| *x == above_of) else {
        order.push(id);
        return order.len() - 1;
    };
    let target = (pos + 1).min(order.len());
    order.insert(target, id);
    target
}

/// Insert `id` immediately below `below_of`.  Returns final index.
pub fn insert_below(order: &mut Vec<ElementId>, id: ElementId, below_of: ElementId) -> usize {
    // Remove `id` first so re-lookup of below_of is accurate.
    let _ = remove(order, id);
    let Some(pos) = order.iter().position(|x| *x == below_of) else {
        order.insert(0, id);
        return 0;
    };
    order.insert(pos, id);
    pos
}

/// Remove `id` from order.  Returns true if it existed.
pub fn remove(order: &mut Vec<ElementId>, id: ElementId) -> bool {
    let Some(pos) = order.iter().position(|x| *x == id) else { return false };
    order.remove(pos);
    true
}

/// Push `id` to back (highest z) only if absent.  Returns true if inserted.
pub fn push_front_if_absent(order: &mut Vec<ElementId>, id: ElementId) -> bool {
    if order.contains(&id) { return false; }
    order.push(id);
    true
}

/// Return the z-index of `id`, or None if not present.
pub fn z_index_of(order: &[ElementId], id: ElementId) -> Option<usize> {
    order.iter().position(|x| *x == id)
}

/// Given an order vec and a predicate, return ids satisfying it in their
/// current z-order.
pub fn filter_in_order<F: Fn(ElementId) -> bool>(order: &[ElementId], pred: F) -> Vec<ElementId> {
    order.iter().copied().filter(|id| pred(*id)).collect()
}

/// Reorder `group_members` so they are contiguous + preserve their own
/// relative order, placed just above the max-index member.
pub fn gather_group(order: &mut Vec<ElementId>, group_members: &[ElementId]) -> bool {
    if group_members.is_empty() { return false; }
    let mut positions: Vec<(ElementId, usize)> = Vec::new();
    for &m in group_members {
        if let Some(p) = order.iter().position(|x| *x == m) {
            positions.push((m, p));
        }
    }
    if positions.is_empty() { return false; }
    // Preserve relative order of members (sort by current position).
    positions.sort_by_key(|(_, p)| *p);
    let max_pos = positions.last().unwrap().1;
    // Remove all members from order.
    order.retain(|x| !group_members.contains(x));
    // Insert block at clamped position.
    let insert_at = max_pos.min(order.len());
    for (i, (id, _)) in positions.iter().enumerate() {
        order.insert(insert_at + i, *id);
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make(ids: &[u64]) -> Vec<ElementId> {
        ids.to_vec()
    }

    #[test]
    fn send_to_back_moves_to_index_0() {
        let mut order = make(&[1, 2, 3, 4]);
        assert!(send_to_back(&mut order, 3));
        assert_eq!(order, [3, 1, 2, 4]);
    }

    #[test]
    fn send_to_back_preserves_other_elements() {
        let mut order = make(&[10, 20, 30]);
        send_to_back(&mut order, 20);
        assert_eq!(order[0], 20);
        assert_eq!(order.len(), 3);
        assert!(order.contains(&10));
        assert!(order.contains(&30));
    }

    #[test]
    fn send_to_back_returns_false_for_missing() {
        let mut order = make(&[1, 2, 3]);
        assert!(!send_to_back(&mut order, 99));
        assert_eq!(order, [1, 2, 3]);
    }

    #[test]
    fn bring_to_front_moves_to_last() {
        let mut order = make(&[1, 2, 3, 4]);
        assert!(bring_to_front(&mut order, 2));
        assert_eq!(order, [1, 3, 4, 2]);
    }

    #[test]
    fn bring_to_front_returns_false_for_missing() {
        let mut order = make(&[1, 2]);
        assert!(!bring_to_front(&mut order, 5));
    }

    #[test]
    fn raise_moves_up_by_1() {
        let mut order = make(&[1, 2, 3]);
        assert!(raise(&mut order, 2));
        assert_eq!(order, [1, 3, 2]);
    }

    #[test]
    fn raise_noop_at_top() {
        let mut order = make(&[1, 2, 3]);
        assert!(!raise(&mut order, 3));
        assert_eq!(order, [1, 2, 3]);
    }

    #[test]
    fn lower_moves_down_by_1() {
        let mut order = make(&[1, 2, 3]);
        assert!(lower(&mut order, 2));
        assert_eq!(order, [2, 1, 3]);
    }

    #[test]
    fn lower_noop_at_bottom() {
        let mut order = make(&[1, 2, 3]);
        assert!(!lower(&mut order, 1));
        assert_eq!(order, [1, 2, 3]);
    }

    #[test]
    fn insert_above_existing_target_inserts_at_plus_1() {
        let mut order = make(&[1, 2, 3]);
        let idx = insert_above(&mut order, 99, 2);
        assert_eq!(order, [1, 2, 99, 3]);
        assert_eq!(idx, 2);
    }

    #[test]
    fn insert_above_missing_target_appends() {
        let mut order = make(&[1, 2, 3]);
        let idx = insert_above(&mut order, 99, 42);
        assert_eq!(*order.last().unwrap(), 99);
        assert_eq!(idx, order.len() - 1);
    }

    #[test]
    fn insert_above_already_present_id_removes_then_reinserts() {
        let mut order = make(&[1, 2, 3, 4]);
        // Move 1 above 3 — 1 is already present
        insert_above(&mut order, 1, 3);
        // 1 should appear exactly once, right after 3
        let pos_1 = order.iter().position(|&x| x == 1).unwrap();
        let pos_3 = order.iter().position(|&x| x == 3).unwrap();
        assert_eq!(pos_1, pos_3 + 1);
        assert_eq!(order.iter().filter(|&&x| x == 1).count(), 1);
    }

    #[test]
    fn insert_below_existing_target_inserts_at_target_position() {
        let mut order = make(&[1, 2, 3]);
        let idx = insert_below(&mut order, 99, 2);
        assert_eq!(order, [1, 99, 2, 3]);
        assert_eq!(idx, 1);
    }

    #[test]
    fn insert_below_missing_target_prepends() {
        let mut order = make(&[1, 2, 3]);
        let idx = insert_below(&mut order, 99, 42);
        assert_eq!(order[0], 99);
        assert_eq!(idx, 0);
    }

    #[test]
    fn remove_returns_true_and_false() {
        let mut order = make(&[1, 2, 3]);
        assert!(remove(&mut order, 2));
        assert_eq!(order, [1, 3]);
        assert!(!remove(&mut order, 99));
    }

    #[test]
    fn push_front_if_absent_true_on_new_false_on_duplicate() {
        let mut order = make(&[1, 2]);
        assert!(push_front_if_absent(&mut order, 3));
        assert_eq!(*order.last().unwrap(), 3);
        assert!(!push_front_if_absent(&mut order, 1));
        assert_eq!(order.len(), 3);
    }

    #[test]
    fn z_index_of_hit_and_miss() {
        let order = make(&[10, 20, 30]);
        assert_eq!(z_index_of(&order, 20), Some(1));
        assert_eq!(z_index_of(&order, 99), None);
    }

    #[test]
    fn filter_in_order_preserves_z_order() {
        let order = make(&[1, 2, 3, 4, 5]);
        let result = filter_in_order(&order, |id| id % 2 == 1);
        assert_eq!(result, [1, 3, 5]);
    }

    #[test]
    fn gather_group_makes_members_contiguous_preserving_relative_order() {
        // Order: [1, 2, 3, 4, 5]; group = [1, 3, 5] (scattered)
        let mut order = make(&[1, 2, 3, 4, 5]);
        assert!(gather_group(&mut order, &[1, 3, 5]));
        // All group members must be contiguous.
        let positions: Vec<usize> = [1u64, 3, 5]
            .iter()
            .map(|id| order.iter().position(|x| x == id).unwrap())
            .collect();
        // Contiguous means max - min == count - 1
        let min = *positions.iter().min().unwrap();
        let max = *positions.iter().max().unwrap();
        assert_eq!(max - min, 2);
        // Relative order preserved: 1 < 3 < 5 in positions
        assert!(positions[0] < positions[1]);
        assert!(positions[1] < positions[2]);
        // Non-members still present exactly once
        assert_eq!(order.iter().filter(|&&x| x == 2).count(), 1);
        assert_eq!(order.iter().filter(|&&x| x == 4).count(), 1);
    }

    #[test]
    fn gather_group_empty_returns_false() {
        let mut order = make(&[1, 2, 3]);
        assert!(!gather_group(&mut order, &[]));
    }
}
