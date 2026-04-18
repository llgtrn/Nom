/// A single tab entry in a tab bar.
#[derive(Debug, Clone, PartialEq)]
pub struct TabEntry {
    pub id: u64,
    pub label: String,
    pub closeable: bool,
    pub active: bool,
}

impl TabEntry {
    /// Create a new tab with `closeable = true` and `active = false`.
    pub fn new(id: u64, label: impl Into<String>) -> Self {
        Self {
            id,
            label: label.into(),
            closeable: true,
            active: false,
        }
    }

    /// Builder-style setter for `closeable`.
    pub fn with_closeable(mut self, c: bool) -> Self {
        self.closeable = c;
        self
    }

    /// Mark this tab as active.
    pub fn activate(&mut self) {
        self.active = true;
    }

    /// Mark this tab as inactive.
    pub fn deactivate(&mut self) {
        self.active = false;
    }
}

/// An ordered list of tabs with active-tab tracking.
#[derive(Debug, Clone, Default)]
pub struct TabBar {
    pub tabs: Vec<TabEntry>,
}

impl TabBar {
    /// Create an empty `TabBar`.
    pub fn new() -> Self {
        Self::default()
    }

    /// Append a tab to the bar.
    pub fn add_tab(&mut self, tab: TabEntry) {
        self.tabs.push(tab);
    }

    /// Remove a tab by `id` only if it is closeable.
    /// Does nothing if the tab is not found or is not closeable.
    pub fn close_tab(&mut self, id: u64) {
        if let Some(pos) = self.tabs.iter().position(|t| t.id == id && t.closeable) {
            self.tabs.remove(pos);
        }
    }

    /// Deactivate all tabs, then activate the tab with `id`.
    /// Does nothing if no tab with that `id` exists.
    pub fn activate_tab(&mut self, id: u64) {
        for tab in &mut self.tabs {
            tab.active = false;
        }
        if let Some(tab) = self.tabs.iter_mut().find(|t| t.id == id) {
            tab.active = true;
        }
    }

    /// Return a reference to the currently active tab, if any.
    pub fn active_tab(&self) -> Option<&TabEntry> {
        self.tabs.iter().find(|t| t.active)
    }

    /// Total number of tabs.
    pub fn tab_count(&self) -> usize {
        self.tabs.len()
    }

    /// Number of tabs that are closeable.
    pub fn closeable_count(&self) -> usize {
        self.tabs.iter().filter(|t| t.closeable).count()
    }
}

/// Full tab bar interaction state, including navigation history.
#[derive(Debug, Clone, Default)]
pub struct TabBarState {
    pub bar: TabBar,
    /// Stack of previously active tab IDs (most-recent last).
    pub history: Vec<u64>,
}

impl TabBarState {
    /// Create an empty `TabBarState`.
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a tab to the bar and activate it immediately.
    pub fn open(&mut self, tab: TabEntry) {
        let id = tab.id;
        self.bar.add_tab(tab);
        self.bar.activate_tab(id);
    }

    /// Record the currently active tab in history, then activate `id`.
    pub fn switch_to(&mut self, id: u64) {
        if let Some(current) = self.bar.active_tab() {
            let current_id = current.id;
            self.history.push(current_id);
        }
        self.bar.activate_tab(id);
    }

    /// Pop the last entry from history and activate it.
    /// Returns the ID that was activated, or `None` if history is empty.
    pub fn back(&mut self) -> Option<u64> {
        let prev = self.history.pop()?;
        self.bar.activate_tab(prev);
        Some(prev)
    }

    /// Number of entries in the navigation history.
    pub fn history_len(&self) -> usize {
        self.history.len()
    }
}

#[cfg(test)]
mod tab_bar_tests {
    use super::*;

    #[test]
    fn tab_entry_activate_deactivate() {
        let mut tab = TabEntry::new(1, "File");
        assert!(!tab.active);
        tab.activate();
        assert!(tab.active);
        tab.deactivate();
        assert!(!tab.active);
    }

    #[test]
    fn tab_entry_with_closeable() {
        let tab = TabEntry::new(2, "Home").with_closeable(false);
        assert!(!tab.closeable);
        let tab2 = TabEntry::new(3, "Settings").with_closeable(true);
        assert!(tab2.closeable);
    }

    #[test]
    fn tab_bar_add_and_count() {
        let mut bar = TabBar::new();
        assert_eq!(bar.tab_count(), 0);
        bar.add_tab(TabEntry::new(1, "A"));
        bar.add_tab(TabEntry::new(2, "B"));
        assert_eq!(bar.tab_count(), 2);
    }

    #[test]
    fn tab_bar_close_removes_closeable() {
        let mut bar = TabBar::new();
        bar.add_tab(TabEntry::new(1, "Closeable"));
        bar.add_tab(TabEntry::new(2, "Pinned").with_closeable(false));
        bar.close_tab(2); // should NOT be removed
        assert_eq!(bar.tab_count(), 2);
        bar.close_tab(1); // SHOULD be removed
        assert_eq!(bar.tab_count(), 1);
        assert_eq!(bar.tabs[0].id, 2);
    }

    #[test]
    fn tab_bar_activate_sets_active() {
        let mut bar = TabBar::new();
        bar.add_tab(TabEntry::new(1, "A"));
        bar.add_tab(TabEntry::new(2, "B"));
        bar.activate_tab(1);
        assert!(bar.tabs[0].active);
        assert!(!bar.tabs[1].active);
        bar.activate_tab(2);
        assert!(!bar.tabs[0].active);
        assert!(bar.tabs[1].active);
    }

    #[test]
    fn tab_bar_active_tab() {
        let mut bar = TabBar::new();
        assert!(bar.active_tab().is_none());
        bar.add_tab(TabEntry::new(10, "Alpha"));
        bar.activate_tab(10);
        let active = bar.active_tab().expect("should have active tab");
        assert_eq!(active.id, 10);
        assert_eq!(active.label, "Alpha");
    }

    #[test]
    fn tab_bar_state_open_activates() {
        let mut state = TabBarState::new();
        state.open(TabEntry::new(1, "First"));
        state.open(TabEntry::new(2, "Second"));
        let active = state.bar.active_tab().expect("active tab");
        assert_eq!(active.id, 2);
    }

    #[test]
    fn tab_bar_state_switch_records_history() {
        let mut state = TabBarState::new();
        state.open(TabEntry::new(1, "A"));
        state.open(TabEntry::new(2, "B"));
        // Current active is 2; switch to 1 should record 2 in history
        state.switch_to(1);
        assert_eq!(state.history_len(), 1);
        assert_eq!(state.history[0], 2);
        assert_eq!(state.bar.active_tab().map(|t| t.id), Some(1));
    }

    #[test]
    fn tab_bar_state_back_navigates() {
        let mut state = TabBarState::new();
        state.open(TabEntry::new(1, "A"));
        state.open(TabEntry::new(2, "B"));
        state.switch_to(1); // active=1, history=[2]
        let prev = state.back();
        assert_eq!(prev, Some(2));
        assert_eq!(state.history_len(), 0);
        assert_eq!(state.bar.active_tab().map(|t| t.id), Some(2));
    }
}
