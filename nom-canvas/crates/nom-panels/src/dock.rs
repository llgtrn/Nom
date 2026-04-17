#![deny(unsafe_code)]

#[derive(Debug, Clone, PartialEq)]
pub enum RenderPrimitive {
    Rect { x: f32, y: f32, w: f32, h: f32, color: u32 },
    Text { x: f32, y: f32, text: String, size: f32, color: u32 },
    Line { x1: f32, y1: f32, x2: f32, y2: f32, color: u32 },
}

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

    pub fn render_bounds(&self, width: f32, height: f32) -> Vec<RenderPrimitive> {
        if !self.is_open { return vec![]; }
        let mut out = Vec::new();

        // Sidebar background rect
        let (x, y, w, h) = match self.position {
            DockPosition::Left   => (0.0,          0.0,           220.0, height),
            DockPosition::Right  => (width - 220.0, 0.0,          220.0, height),
            DockPosition::Bottom => (0.0,          height - 160.0, width, 160.0),
        };
        out.push(RenderPrimitive::Rect { x, y, w, h, color: 0x1e1e2e });

        // Panel entry tab text rows
        let tab_x = x + 8.0;
        for (i, entry) in self.entries.iter().enumerate() {
            if !entry.is_visible { continue; }
            let tab_y = y + 8.0 + i as f32 * 24.0;
            let is_active = self.active_index == Some(i);
            if is_active {
                out.push(RenderPrimitive::Rect {
                    x: x + 2.0,
                    y: tab_y - 2.0,
                    w: w - 4.0,
                    h: 20.0,
                    color: 0x313244,
                });
            }
            out.push(RenderPrimitive::Text {
                x: tab_x,
                y: tab_y,
                text: entry.id.clone(),
                size: 13.0,
                color: if is_active { 0xcdd6f4 } else { 0x6c7086 },
            });
        }

        out
    }
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

    #[test]
    fn dock_render_left_sidebar() {
        let mut dock = Dock::new(DockPosition::Left);
        dock.add_panel("file-tree", 248.0);
        dock.add_panel("search", 248.0);
        let prims = dock.render_bounds(1440.0, 900.0);

        // First primitive: background rect at x=0, w=220, full height
        match &prims[0] {
            RenderPrimitive::Rect { x, y, w, h, color } => {
                assert!((x - 0.0).abs() < 0.01);
                assert!((y - 0.0).abs() < 0.01);
                assert!((w - 220.0).abs() < 0.01);
                assert!((h - 900.0).abs() < 0.01);
                assert_eq!(*color, 0x1e1e2e);
            }
            _ => panic!("expected Rect"),
        }

        // Active tab (file-tree) should have a highlight rect followed by Text
        let has_active_rect = prims.iter().any(|p| matches!(p,
            RenderPrimitive::Rect { color: 0x313244, .. }
        ));
        assert!(has_active_rect, "active tab highlight rect missing");

        // Panel names appear as Text primitives
        let texts: Vec<&str> = prims.iter().filter_map(|p| {
            if let RenderPrimitive::Text { text, .. } = p { Some(text.as_str()) } else { None }
        }).collect();
        assert!(texts.contains(&"file-tree"));
        assert!(texts.contains(&"search"));
    }
}
