#![deny(unsafe_code)]

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DockPosition { Left, Right, Bottom }

pub struct PanelSizeState {
    pub size: Option<f32>,   // fixed pixel size
    pub flex: Option<f32>,   // proportional (0.0–1.0), overrides size when set
}

impl PanelSizeState {
    pub fn fixed(px: f32) -> Self { Self { size: Some(px), flex: None } }
    pub fn flex(ratio: f32) -> Self { Self { size: None, flex: Some(ratio.clamp(0.0, 1.0)) } }
    pub fn effective_size(&self, container: f32) -> f32 {
        if let Some(f) = self.flex { f * container }
        else { self.size.unwrap_or(0.0) }
    }
}

pub struct PanelEntry {
    pub id: String,
    pub size_state: PanelSizeState,
    pub is_visible: bool,
}

pub struct Dock {
    pub position: DockPosition,
    pub entries: Vec<PanelEntry>,
    pub active_index: Option<usize>,
    pub is_open: bool,
}

impl Dock {
    pub fn new(position: DockPosition) -> Self {
        Self { position, entries: vec![], active_index: None, is_open: true }
    }

    pub fn add_panel(&mut self, id: impl Into<String>, size_px: f32) {
        let id = id.into();
        self.entries.push(PanelEntry {
            id,
            size_state: PanelSizeState::fixed(size_px),
            is_visible: true,
        });
        if self.active_index.is_none() {
            self.active_index = Some(0);
        }
    }

    pub fn activate(&mut self, id: &str) -> bool {
        if let Some(idx) = self.entries.iter().position(|e| e.id == id) {
            self.active_index = Some(idx);
            self.is_open = true;
            true
        } else {
            false
        }
    }

    pub fn active_panel_id(&self) -> Option<&str> {
        self.active_index.and_then(|i| self.entries.get(i)).map(|e| e.id.as_str())
    }

    pub fn toggle(&mut self) { self.is_open = !self.is_open; }
    pub fn panel_count(&self) -> usize { self.entries.len() }
}

pub trait Panel {
    fn id(&self) -> &str;
    fn title(&self) -> &str;
    fn default_size(&self) -> f32;
    fn position(&self) -> DockPosition;
    fn is_visible(&self) -> bool { true }
    fn activation_priority(&self) -> u32 { 100 }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dock_add_and_activate() {
        let mut dock = Dock::new(DockPosition::Left);
        dock.add_panel("file-tree", 248.0);
        dock.add_panel("search", 248.0);
        assert_eq!(dock.active_panel_id(), Some("file-tree"));
        assert!(dock.activate("search"));
        assert_eq!(dock.active_panel_id(), Some("search"));
    }

    #[test]
    fn size_state_flex() {
        let s = PanelSizeState::flex(0.3);
        assert!((s.effective_size(1000.0) - 300.0).abs() < 0.01);
    }

    #[test]
    fn dock_toggle() {
        let mut dock = Dock::new(DockPosition::Bottom);
        assert!(dock.is_open);
        dock.toggle();
        assert!(!dock.is_open);
    }
}
