//! Element grouping: multiple elements selected + transformed as one.
#![deny(unsafe_code)]

use std::collections::{HashMap, HashSet};
use crate::element::ElementId;

pub type GroupId = u64;

#[derive(Clone, Debug, PartialEq)]
pub struct Group {
    pub id: GroupId,
    pub member_ids: HashSet<ElementId>,
    pub locked: bool,
}

impl Group {
    pub fn new(id: GroupId) -> Self { Self { id, member_ids: HashSet::new(), locked: false } }
    pub fn add(&mut self, element_id: ElementId) { self.member_ids.insert(element_id); }
    pub fn remove(&mut self, element_id: ElementId) -> bool { self.member_ids.remove(&element_id) }
    pub fn contains(&self, element_id: ElementId) -> bool { self.member_ids.contains(&element_id) }
    pub fn len(&self) -> usize { self.member_ids.len() }
    pub fn is_empty(&self) -> bool { self.member_ids.is_empty() }
    pub fn set_locked(&mut self, locked: bool) { self.locked = locked; }
}

#[derive(Default)]
pub struct GroupRegistry {
    groups: HashMap<GroupId, Group>,
    element_to_group: HashMap<ElementId, GroupId>,
    next_id: GroupId,
}

impl GroupRegistry {
    pub fn new() -> Self { Self::default() }

    /// Create a new group containing the given elements.  Elements already
    /// in another group are MOVED to the new one (single-group membership).
    pub fn create_group(&mut self, members: &[ElementId]) -> GroupId {
        let id = self.next_id;
        self.next_id += 1;
        let mut group = Group::new(id);
        for &m in members {
            // Remove from prior group if any.
            if let Some(&prior_group_id) = self.element_to_group.get(&m) {
                if let Some(prior) = self.groups.get_mut(&prior_group_id) {
                    prior.remove(m);
                }
            }
            group.add(m);
            self.element_to_group.insert(m, id);
        }
        self.groups.insert(id, group);
        id
    }

    pub fn group_of(&self, element_id: ElementId) -> Option<GroupId> {
        self.element_to_group.get(&element_id).copied()
    }

    pub fn members_of(&self, group_id: GroupId) -> Option<&HashSet<ElementId>> {
        self.groups.get(&group_id).map(|g| &g.member_ids)
    }

    pub fn dissolve(&mut self, group_id: GroupId) -> bool {
        if let Some(group) = self.groups.remove(&group_id) {
            for m in group.member_ids {
                self.element_to_group.remove(&m);
            }
            true
        } else {
            false
        }
    }

    pub fn add_to_group(&mut self, group_id: GroupId, element_id: ElementId) -> bool {
        // Detach from previous group first.
        if let Some(&prior_group_id) = self.element_to_group.get(&element_id) {
            if prior_group_id == group_id { return false; /* no-op */ }
            if let Some(prior) = self.groups.get_mut(&prior_group_id) { prior.remove(element_id); }
        }
        if let Some(group) = self.groups.get_mut(&group_id) {
            group.add(element_id);
            self.element_to_group.insert(element_id, group_id);
            true
        } else {
            false
        }
    }

    pub fn remove_from_group(&mut self, element_id: ElementId) -> Option<GroupId> {
        let group_id = self.element_to_group.remove(&element_id)?;
        if let Some(group) = self.groups.get_mut(&group_id) { group.remove(element_id); }
        Some(group_id)
    }

    pub fn lock(&mut self, group_id: GroupId) -> bool {
        if let Some(g) = self.groups.get_mut(&group_id) { g.set_locked(true); true } else { false }
    }

    pub fn unlock(&mut self, group_id: GroupId) -> bool {
        if let Some(g) = self.groups.get_mut(&group_id) { g.set_locked(false); true } else { false }
    }

    pub fn is_locked(&self, group_id: GroupId) -> bool {
        self.groups.get(&group_id).map(|g| g.locked).unwrap_or(false)
    }

    pub fn len(&self) -> usize { self.groups.len() }
    pub fn all_groups(&self) -> Vec<&Group> { self.groups.values().collect() }
}

#[cfg(test)]
mod tests {
    use super::*;

    // --- Group unit tests ---

    #[test]
    fn group_new_is_empty() {
        let g = Group::new(1);
        assert_eq!(g.id, 1);
        assert!(g.is_empty());
        assert_eq!(g.len(), 0);
        assert!(!g.locked);
    }

    #[test]
    fn group_add_contains_len() {
        let mut g = Group::new(0);
        g.add(10);
        g.add(20);
        assert!(g.contains(10));
        assert!(g.contains(20));
        assert!(!g.contains(99));
        assert_eq!(g.len(), 2);
        assert!(!g.is_empty());
    }

    #[test]
    fn group_remove_existing_returns_true() {
        let mut g = Group::new(0);
        g.add(5);
        assert!(g.remove(5));
        assert!(!g.contains(5));
    }

    #[test]
    fn group_remove_missing_returns_false() {
        let mut g = Group::new(0);
        assert!(!g.remove(42));
    }

    #[test]
    fn group_set_locked() {
        let mut g = Group::new(0);
        assert!(!g.locked);
        g.set_locked(true);
        assert!(g.locked);
        g.set_locked(false);
        assert!(!g.locked);
    }

    // --- GroupRegistry unit tests ---

    #[test]
    fn registry_new_is_empty() {
        let r = GroupRegistry::new();
        assert_eq!(r.len(), 0);
        assert!(r.all_groups().is_empty());
    }

    #[test]
    fn create_group_assigns_monotonic_ids() {
        let mut r = GroupRegistry::new();
        let a = r.create_group(&[1, 2]);
        let b = r.create_group(&[3]);
        let c = r.create_group(&[]);
        assert!(b > a);
        assert!(c > b);
    }

    #[test]
    fn create_group_populates_reverse_map() {
        let mut r = GroupRegistry::new();
        let gid = r.create_group(&[10, 20, 30]);
        assert_eq!(r.group_of(10), Some(gid));
        assert_eq!(r.group_of(20), Some(gid));
        assert_eq!(r.group_of(30), Some(gid));
    }

    #[test]
    fn create_group_moves_element_from_prior_group() {
        let mut r = GroupRegistry::new();
        let g1 = r.create_group(&[1, 2]);
        let g2 = r.create_group(&[2, 3]); // element 2 moves from g1 to g2
        assert_eq!(r.group_of(2), Some(g2));
        // g1 should no longer contain element 2
        let members = r.members_of(g1).unwrap();
        assert!(!members.contains(&2));
    }

    #[test]
    fn group_of_hit_and_miss() {
        let mut r = GroupRegistry::new();
        let gid = r.create_group(&[7]);
        assert_eq!(r.group_of(7), Some(gid));
        assert_eq!(r.group_of(99), None);
    }

    #[test]
    fn members_of_returns_hashset() {
        let mut r = GroupRegistry::new();
        let gid = r.create_group(&[4, 5, 6]);
        let members = r.members_of(gid).expect("group must exist");
        assert!(members.contains(&4));
        assert!(members.contains(&5));
        assert!(members.contains(&6));
        assert_eq!(members.len(), 3);
    }

    #[test]
    fn members_of_unknown_group_returns_none() {
        let r = GroupRegistry::new();
        assert!(r.members_of(999).is_none());
    }

    #[test]
    fn dissolve_removes_group_and_reverse_entries() {
        let mut r = GroupRegistry::new();
        let gid = r.create_group(&[1, 2]);
        assert!(r.dissolve(gid));
        assert!(r.members_of(gid).is_none());
        assert_eq!(r.group_of(1), None);
        assert_eq!(r.group_of(2), None);
    }

    #[test]
    fn dissolve_unknown_returns_false() {
        let mut r = GroupRegistry::new();
        assert!(!r.dissolve(42));
    }

    #[test]
    fn add_to_group_ok_for_existing_group() {
        let mut r = GroupRegistry::new();
        let gid = r.create_group(&[]);
        assert!(r.add_to_group(gid, 55));
        assert_eq!(r.group_of(55), Some(gid));
    }

    #[test]
    fn add_to_group_false_for_unknown_group() {
        let mut r = GroupRegistry::new();
        assert!(!r.add_to_group(999, 1));
    }

    #[test]
    fn add_to_group_moves_element_from_prior_group() {
        let mut r = GroupRegistry::new();
        let g1 = r.create_group(&[10]);
        let g2 = r.create_group(&[]);
        assert!(r.add_to_group(g2, 10));
        assert_eq!(r.group_of(10), Some(g2));
        assert!(!r.members_of(g1).unwrap().contains(&10));
    }

    #[test]
    fn add_to_group_same_group_is_noop_returning_false() {
        let mut r = GroupRegistry::new();
        let gid = r.create_group(&[7]);
        assert!(!r.add_to_group(gid, 7));
        assert_eq!(r.group_of(7), Some(gid));
    }

    #[test]
    fn remove_from_group_returns_some_prior_id() {
        let mut r = GroupRegistry::new();
        let gid = r.create_group(&[3]);
        let result = r.remove_from_group(3);
        assert_eq!(result, Some(gid));
        assert_eq!(r.group_of(3), None);
        assert!(r.members_of(gid).unwrap().is_empty());
    }

    #[test]
    fn remove_from_group_unknown_element_returns_none() {
        let mut r = GroupRegistry::new();
        assert_eq!(r.remove_from_group(999), None);
    }

    #[test]
    fn lock_unlock_is_locked_round_trip() {
        let mut r = GroupRegistry::new();
        let gid = r.create_group(&[1]);
        assert!(!r.is_locked(gid));
        assert!(r.lock(gid));
        assert!(r.is_locked(gid));
        assert!(r.unlock(gid));
        assert!(!r.is_locked(gid));
    }

    #[test]
    fn lock_unknown_group_returns_false() {
        let mut r = GroupRegistry::new();
        assert!(!r.lock(999));
        assert!(!r.unlock(999));
        assert!(!r.is_locked(999));
    }

    #[test]
    fn len_and_all_groups_reflect_creates() {
        let mut r = GroupRegistry::new();
        assert_eq!(r.len(), 0);
        r.create_group(&[1]);
        r.create_group(&[2]);
        r.create_group(&[3]);
        assert_eq!(r.len(), 3);
        assert_eq!(r.all_groups().len(), 3);
    }
}
