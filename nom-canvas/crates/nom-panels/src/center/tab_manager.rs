//! Tab manager for the center editor area.

/// The kind of content a tab holds.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TabKind {
    CanvasEditor,
    CodeEditor,
    Preview,
    Terminal,
}

/// A single tab entry.
#[derive(Debug, Clone)]
pub struct Tab {
    pub id: String,
    pub title: String,
    pub kind: TabKind,
    pub dirty: bool,
    pub pinned: bool,
}

/// Manages the collection of open tabs and the active tab.
#[derive(Debug, Clone)]
pub struct TabManager {
    pub tabs: Vec<Tab>,
    pub active_id: Option<String>,
}

impl TabManager {
    /// Create an empty tab manager.
    pub fn new() -> Self {
        Self {
            tabs: vec![],
            active_id: None,
        }
    }

    /// Open a new tab (builder-style). If a tab with the same id already
    /// exists it is replaced and made active.
    pub fn open_tab(mut self, tab: Tab) -> Self {
        if let Some(pos) = self.tabs.iter().position(|t| t.id == tab.id) {
            self.tabs[pos] = tab.clone();
        } else {
            self.tabs.push(tab.clone());
        }
        self.active_id = Some(tab.id);
        self
    }

    /// Close the tab with the given id. Pinned tabs are not closed.
    /// If the closed tab was active, the preceding tab (or the first) becomes
    /// active. No-op when the id is not found.
    pub fn close_tab(mut self, id: &str) -> Self {
        let Some(pos) = self.tabs.iter().position(|t| t.id == id) else {
            return self;
        };
        if self.tabs[pos].pinned {
            return self;
        }
        self.tabs.remove(pos);
        if self.active_id.as_deref() == Some(id) {
            self.active_id = self
                .tabs
                .get(pos.saturating_sub(1))
                .or_else(|| self.tabs.first())
                .map(|t| t.id.clone());
        }
        self
    }

    /// Set the active tab by id. No-op if the id is not present.
    pub fn set_active(mut self, id: &str) -> Self {
        if self.tabs.iter().any(|t| t.id == id) {
            self.active_id = Some(id.to_owned());
        }
        self
    }

    /// Return a reference to the currently active tab, if any.
    pub fn active_tab(&self) -> Option<&Tab> {
        let id = self.active_id.as_deref()?;
        self.tabs.iter().find(|t| t.id == id)
    }

    /// Count of tabs with unsaved changes.
    pub fn dirty_count(&self) -> usize {
        self.tabs.iter().filter(|t| t.dirty).count()
    }
}

impl Default for TabManager {
    fn default() -> Self {
        Self::new()
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn make_tab(id: &str, kind: TabKind) -> Tab {
        Tab {
            id: id.to_owned(),
            title: id.to_owned(),
            kind,
            dirty: false,
            pinned: false,
        }
    }

    #[test]
    fn new_is_empty() {
        let tm = TabManager::new();
        assert!(tm.tabs.is_empty());
        assert!(tm.active_id.is_none());
    }

    #[test]
    fn open_tab_adds_and_activates() {
        let tm = TabManager::new().open_tab(make_tab("a", TabKind::CodeEditor));
        assert_eq!(tm.tabs.len(), 1);
        assert_eq!(tm.active_id.as_deref(), Some("a"));
    }

    #[test]
    fn close_tab_pinned_stays() {
        let mut tab = make_tab("pinned", TabKind::CanvasEditor);
        tab.pinned = true;
        let tm = TabManager::new()
            .open_tab(tab)
            .open_tab(make_tab("other", TabKind::Preview));
        let tm = tm.close_tab("pinned");
        // pinned tab must still be present
        assert!(tm.tabs.iter().any(|t| t.id == "pinned"));
    }

    #[test]
    fn set_active_updates_active_id() {
        let tm = TabManager::new()
            .open_tab(make_tab("a", TabKind::CodeEditor))
            .open_tab(make_tab("b", TabKind::Terminal))
            .set_active("a");
        assert_eq!(tm.active_id.as_deref(), Some("a"));
    }

    #[test]
    fn dirty_count_counts_dirty_tabs() {
        let mut t1 = make_tab("x", TabKind::CodeEditor);
        t1.dirty = true;
        let t2 = make_tab("y", TabKind::Preview);
        let tm = TabManager::new().open_tab(t1).open_tab(t2);
        assert_eq!(tm.dirty_count(), 1);
    }
}
