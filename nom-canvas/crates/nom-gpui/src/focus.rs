use crate::types::*;
use std::collections::HashMap;
use std::sync::{
    atomic::{AtomicUsize, Ordering},
    Arc,
};

/// Focus handle — manages keyboard focus for UI elements
/// Pattern: Zed FocusHandle (SlotMap + Arc<AtomicUsize> ref count)
#[derive(Debug, Clone)]
pub struct FocusHandle {
    pub id: FocusId,
    ref_count: Arc<AtomicUsize>,
    pub tab_index: isize,
    pub tab_stop: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct FocusId(pub u64);

impl FocusHandle {
    pub fn new(id: FocusId) -> Self {
        Self {
            id,
            ref_count: Arc::new(AtomicUsize::new(1)),
            tab_index: 0,
            tab_stop: true,
        }
    }

    pub fn is_focused(&self, focus_manager: &FocusManager) -> bool {
        focus_manager.focused == Some(self.id)
    }

    pub fn ref_count(&self) -> usize {
        self.ref_count.load(Ordering::SeqCst)
    }
}

impl PartialEq for FocusHandle {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

/// Tracks focused element within a window
pub struct FocusManager {
    pub focused: Option<FocusId>,
    next_id: u64,
    handles: HashMap<FocusId, FocusHandle>,
}

impl FocusManager {
    pub fn new() -> Self {
        Self {
            focused: None,
            next_id: 1,
            handles: HashMap::new(),
        }
    }

    pub fn create_handle(&mut self) -> FocusHandle {
        let id = FocusId(self.next_id);
        self.next_id += 1;
        let handle = FocusHandle::new(id);
        self.handles.insert(id, handle.clone());
        handle
    }

    pub fn focus(&mut self, handle: &FocusHandle) {
        self.focused = Some(handle.id);
    }

    pub fn blur(&mut self) {
        self.focused = None;
    }

    pub fn is_focused(&self, handle: &FocusHandle) -> bool {
        self.focused == Some(handle.id)
    }

    /// Tab-order traversal: focus next tab-stop element
    pub fn focus_next(&mut self) {
        let ids: Vec<FocusId> = {
            let mut sorted: Vec<_> = self
                .handles
                .values()
                .filter(|h| h.tab_stop)
                .map(|h| (h.tab_index, h.id))
                .collect();
            sorted.sort_by_key(|(ti, _)| *ti);
            sorted.into_iter().map(|(_, id)| id).collect()
        };
        if ids.is_empty() {
            return;
        }
        let next = match self.focused {
            None => ids[0],
            Some(current) => {
                let pos = ids.iter().position(|&id| id == current);
                match pos {
                    None => ids[0],
                    Some(i) => ids[(i + 1) % ids.len()],
                }
            }
        };
        self.focused = Some(next);
    }
}

impl Default for FocusManager {
    fn default() -> Self {
        Self::new()
    }
}

// Suppress unused import — Vec2 imported via types::* glob, may be needed by future extensions
#[allow(unused_imports)]
use Vec2 as _;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn create_handle_gives_unique_ids() {
        let mut fm = FocusManager::new();
        let h1 = fm.create_handle();
        let h2 = fm.create_handle();
        let h3 = fm.create_handle();
        assert_ne!(h1.id, h2.id);
        assert_ne!(h2.id, h3.id);
        assert_ne!(h1.id, h3.id);
    }

    #[test]
    fn focus_sets_focused() {
        let mut fm = FocusManager::new();
        let h = fm.create_handle();
        assert!(fm.focused.is_none());
        fm.focus(&h);
        assert_eq!(fm.focused, Some(h.id));
    }

    #[test]
    fn is_focused_returns_correct_value() {
        let mut fm = FocusManager::new();
        let h1 = fm.create_handle();
        let h2 = fm.create_handle();
        fm.focus(&h1);
        assert!(fm.is_focused(&h1));
        assert!(!fm.is_focused(&h2));
    }

    #[test]
    fn focus_next_cycles_through_handles() {
        let mut fm = FocusManager::new();
        let h1 = fm.create_handle();
        let h2 = fm.create_handle();
        let h3 = fm.create_handle();

        // No current focus — should land on first tab-ordered handle
        fm.focus_next();
        let first = fm.focused.unwrap();

        fm.focus_next();
        let second = fm.focused.unwrap();
        assert_ne!(first, second);

        fm.focus_next();
        let third = fm.focused.unwrap();
        assert_ne!(second, third);

        // Fourth call wraps back to first
        fm.focus_next();
        assert_eq!(fm.focused.unwrap(), first);

        // Keep compiler happy — all handles referenced
        let _ = (h1.id, h2.id, h3.id);
    }

    #[test]
    fn blur_clears_focus() {
        let mut fm = FocusManager::new();
        let h = fm.create_handle();
        fm.focus(&h);
        fm.blur();
        assert!(fm.focused.is_none());
    }

    #[test]
    fn focus_handle_new_is_unfocused() {
        let mut fm = FocusManager::new();
        let h = fm.create_handle();
        // A freshly created handle must not be focused
        assert!(!h.is_focused(&fm));
    }

    #[test]
    fn focus_handle_focus_sets_focused() {
        let mut fm = FocusManager::new();
        let h = fm.create_handle();
        fm.focus(&h);
        assert!(h.is_focused(&fm));
    }

    #[test]
    fn focus_handle_blur_clears() {
        let mut fm = FocusManager::new();
        let h = fm.create_handle();
        fm.focus(&h);
        assert!(h.is_focused(&fm));
        fm.blur();
        assert!(!h.is_focused(&fm));
    }

    #[test]
    fn focus_state_focused_id_matches() {
        let mut fm = FocusManager::new();
        let h1 = fm.create_handle();
        let h2 = fm.create_handle();
        fm.focus(&h2);
        // FocusManager.focused tracks the id of the focused handle
        assert_eq!(fm.focused, Some(h2.id));
        assert_ne!(fm.focused, Some(h1.id));
    }

    #[test]
    fn focus_handle_ref_count_starts_at_one() {
        let h = FocusHandle::new(FocusId(42));
        assert_eq!(h.ref_count(), 1);
    }

    #[test]
    fn focus_handle_ref_count_shared_between_clones() {
        let h = FocusHandle::new(FocusId(99));
        let _clone = h.clone();
        // Both handles share the same Arc; strong count is at least 1
        assert!(h.ref_count() >= 1);
        // The clone reports the same count as the original
        assert_eq!(h.ref_count(), _clone.ref_count());
    }

    #[test]
    fn focus_handle_equality_by_id() {
        let h1 = FocusHandle::new(FocusId(1));
        let h2 = FocusHandle::new(FocusId(1));
        let h3 = FocusHandle::new(FocusId(2));
        assert_eq!(h1, h2);
        assert_ne!(h1, h3);
    }

    #[test]
    fn focus_manager_default_has_no_focus() {
        let fm = FocusManager::default();
        assert!(fm.focused.is_none());
    }

    #[test]
    fn focus_manager_focus_next_from_none_lands_on_first() {
        let mut fm = FocusManager::new();
        let _h = fm.create_handle();
        assert!(fm.focused.is_none());
        fm.focus_next();
        assert!(fm.focused.is_some());
    }
}
