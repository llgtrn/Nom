#![deny(unsafe_code)]

use nom_gpui::scene::{Quad, Scene};
use nom_gpui::types::{Bounds, ContentMask, Corners, Edges, Hsla, Pixels, Point, Size};
use nom_theme::tokens;

/// Convert a linear RGBA color (as used by `nom_theme::tokens`) to HSLA
/// understood by `nom_gpui::scene::Quad`.
pub fn rgba_to_hsla(c: [f32; 4]) -> Hsla {
    let (r, g, b, a) = (c[0], c[1], c[2], c[3]);
    let max = r.max(g).max(b);
    let min = r.min(g).min(b);
    let delta = max - min;
    let l = (max + min) / 2.0;
    let s = if delta == 0.0 {
        0.0
    } else {
        delta / (1.0 - (2.0 * l - 1.0).abs())
    };
    let h = if delta == 0.0 {
        0.0
    } else if max == r {
        60.0 * (((g - b) / delta).rem_euclid(6.0))
    } else if max == g {
        60.0 * ((b - r) / delta + 2.0)
    } else {
        60.0 * ((r - g) / delta + 4.0)
    };
    Hsla::new(h, s, l, a)
}

/// Helper: build a filled rectangle quad at the given pixel bounds.
pub fn fill_quad(x: f32, y: f32, w: f32, h: f32, color: [f32; 4]) -> Quad {
    Quad {
        bounds: Bounds {
            origin: Point { x: Pixels(x), y: Pixels(y) },
            size: Size { width: Pixels(w), height: Pixels(h) },
        },
        background: Some(rgba_to_hsla(color)),
        border_color: None,
        border_widths: Edges::default(),
        corner_radii: Corners::default(),
        content_mask: ContentMask { bounds: Bounds::default() },
    }
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

    /// Paint the dock chrome (sidebar background + active-tab highlight)
    /// into the shared GPU scene.
    pub fn paint_scene(&self, width: f32, height: f32, scene: &mut Scene) {
        if !self.is_open { return; }

        let (x, y, w, h) = match self.position {
            DockPosition::Left   => (0.0,            0.0,             220.0, height),
            DockPosition::Right  => (width - 220.0,  0.0,             220.0, height),
            DockPosition::Bottom => (0.0,            height - 160.0,  width, 160.0),
        };

        // Sidebar background with a 1px border on the inside edge.
        let border_edges = match self.position {
            DockPosition::Left   => Edges { left: Pixels(0.0), right: Pixels(1.0), top: Pixels(0.0), bottom: Pixels(0.0) },
            DockPosition::Right  => Edges { left: Pixels(1.0), right: Pixels(0.0), top: Pixels(0.0), bottom: Pixels(0.0) },
            DockPosition::Bottom => Edges { left: Pixels(0.0), right: Pixels(0.0), top: Pixels(1.0), bottom: Pixels(0.0) },
        };
        scene.push_quad(Quad {
            bounds: Bounds {
                origin: Point { x: Pixels(x), y: Pixels(y) },
                size: Size { width: Pixels(w), height: Pixels(h) },
            },
            background: Some(rgba_to_hsla(tokens::BG)),
            border_color: Some(rgba_to_hsla(tokens::BORDER)),
            border_widths: border_edges,
            corner_radii: Corners::default(),
            content_mask: ContentMask { bounds: Bounds::default() },
        });

        // Active-tab highlight quad (one per visible entry that's active).
        for (i, entry) in self.entries.iter().enumerate() {
            if !entry.is_visible { continue; }
            if self.active_index != Some(i) { continue; }
            let tab_y = y + 8.0 + i as f32 * 24.0;
            scene.push_quad(fill_quad(x + 2.0, tab_y - 2.0, w - 4.0, 20.0, tokens::FOCUS));
        }
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
    fn dock_paint_scene_left_sidebar() {
        let mut dock = Dock::new(DockPosition::Left);
        dock.add_panel("file-tree", 248.0);
        dock.add_panel("search", 248.0);
        let mut scene = Scene::new();
        dock.paint_scene(1440.0, 900.0, &mut scene);

        // Background quad + 1 active-tab highlight = 2 quads.
        assert!(scene.quads.len() >= 2);

        // First quad is the background at x=0 spanning full height.
        let bg = &scene.quads[0];
        assert_eq!(bg.bounds.origin.x, Pixels(0.0));
        assert_eq!(bg.bounds.origin.y, Pixels(0.0));
        assert_eq!(bg.bounds.size.width, Pixels(220.0));
        assert_eq!(bg.bounds.size.height, Pixels(900.0));
        assert!(bg.background.is_some());
        assert!(bg.border_color.is_some());
    }

    #[test]
    fn dock_paint_scene_closed_is_empty() {
        let mut dock = Dock::new(DockPosition::Left);
        dock.add_panel("x", 248.0);
        dock.is_open = false;
        let mut scene = Scene::new();
        dock.paint_scene(800.0, 600.0, &mut scene);
        assert_eq!(scene.quads.len(), 0);
    }

    #[test]
    fn rgba_to_hsla_black_and_white() {
        let black = rgba_to_hsla([0.0, 0.0, 0.0, 1.0]);
        assert_eq!(black.l, 0.0);
        assert_eq!(black.a, 1.0);
        let white = rgba_to_hsla([1.0, 1.0, 1.0, 1.0]);
        assert!((white.l - 1.0).abs() < 1e-6);
    }
}
