#![deny(unsafe_code)]

/// Which content tab is shown in the left panel.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LeftPanelTab {
    Library,
    NodePalette,
    FileTree,
    Outline,
}

/// State and geometry of the collapsible left panel.
#[derive(Debug, Clone)]
pub struct LeftPanelLayout {
    pub active_tab: LeftPanelTab,
    pub width: f32,
    pub collapsed: bool,
}

impl LeftPanelLayout {
    /// Create with defaults: Library tab, 240 px wide, expanded.
    pub fn new() -> Self {
        Self {
            active_tab: LeftPanelTab::Library,
            width: 240.0,
            collapsed: false,
        }
    }

    /// Switch to a different tab and return self for chaining.
    pub fn set_tab(mut self, tab: LeftPanelTab) -> Self {
        self.active_tab = tab;
        self
    }

    /// Toggle the collapsed state and return self for chaining.
    pub fn toggle_collapse(mut self) -> Self {
        self.collapsed = !self.collapsed;
        self
    }

    /// Effective pixel width: 0.0 when collapsed, otherwise `width`.
    pub fn effective_width(&self) -> f32 {
        if self.collapsed {
            0.0
        } else {
            self.width
        }
    }
}

impl Default for LeftPanelLayout {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_defaults() {
        let layout = LeftPanelLayout::new();
        assert_eq!(layout.active_tab, LeftPanelTab::Library);
        assert!((layout.width - 240.0).abs() < f32::EPSILON);
        assert!(!layout.collapsed);
    }

    #[test]
    fn set_tab_changes_tab() {
        let layout = LeftPanelLayout::new().set_tab(LeftPanelTab::FileTree);
        assert_eq!(layout.active_tab, LeftPanelTab::FileTree);
    }

    #[test]
    fn toggle_collapse_flips_state() {
        let layout = LeftPanelLayout::new().toggle_collapse();
        assert!(layout.collapsed);
        let layout = layout.toggle_collapse();
        assert!(!layout.collapsed);
    }

    #[test]
    fn effective_width_respects_collapsed() {
        let expanded = LeftPanelLayout::new();
        assert!((expanded.effective_width() - 240.0).abs() < f32::EPSILON);

        let collapsed = LeftPanelLayout::new().toggle_collapse();
        assert!((collapsed.effective_width()).abs() < f32::EPSILON);
    }
}
