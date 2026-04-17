//! Toolbar panel — active flavour indicator and action buttons.

use smallvec::SmallVec;

/// A single action button on the toolbar.
#[derive(Debug, Clone)]
pub struct ToolbarAction {
    pub id: String,
    pub label: String,
    pub icon: String,
    pub enabled: bool,
}

impl ToolbarAction {
    pub fn new(id: impl Into<String>, label: impl Into<String>, icon: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            label: label.into(),
            icon: icon.into(),
            enabled: true,
        }
    }
}

/// Toolbar panel state.
#[derive(Debug)]
pub struct Toolbar {
    pub height_px: f32,
    pub active_flavour: Option<String>,
    pub actions: SmallVec<[ToolbarAction; 8]>,
}

impl Toolbar {
    pub fn new() -> Self {
        Self {
            height_px: 48.0,
            active_flavour: None,
            actions: SmallVec::new(),
        }
    }

    /// Update the active block flavour shown in the toolbar.
    pub fn set_active_block(&mut self, flavour: &str) {
        self.active_flavour = Some(flavour.to_string());
    }

    /// Look up a toolbar action by its id.
    pub fn action_by_id(&self, id: &str) -> Option<&ToolbarAction> {
        self.actions.iter().find(|a| a.id == id)
    }

    /// Stub paint method — rendering lives in the GPU layer.
    pub fn paint(&self) {}
}

impl Default for Toolbar {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_height_is_48() {
        let t = Toolbar::new();
        assert_eq!(t.height_px, 48.0);
        assert!(t.active_flavour.is_none());
    }

    #[test]
    fn set_active_block_stores_flavour() {
        let mut t = Toolbar::new();
        t.set_active_block("code");
        assert_eq!(t.active_flavour.as_deref(), Some("code"));
    }

    #[test]
    fn action_by_id_hit_and_miss() {
        let mut t = Toolbar::new();
        t.actions.push(ToolbarAction::new("run", "Run", "play"));
        assert!(t.action_by_id("run").is_some());
        assert!(t.action_by_id("missing").is_none());
    }
}
