//! Panel-shell layout solver.
//!
//! Takes a viewport `(w, h)` and a `PanelTree` describing the docked panels
//! (sidebar, toolbar, statusbar, content area), produces concrete pixel
//! rects for each region.
#![deny(unsafe_code)]

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Rect { pub x: f32, pub y: f32, pub w: f32, pub h: f32 }

impl Rect {
    pub fn new(x: f32, y: f32, w: f32, h: f32) -> Self { Self { x, y, w, h } }
    pub fn contains(&self, px: f32, py: f32) -> bool {
        px >= self.x && px < self.x + self.w && py >= self.y && py < self.y + self.h
    }
    pub fn area(&self) -> f32 { (self.w.max(0.0)) * (self.h.max(0.0)) }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Dock { Left, Right, Top, Bottom, Center }

#[derive(Clone, Debug, PartialEq)]
pub struct PanelNode {
    pub name: String,
    pub dock: Dock,
    /// For Left/Right: width in px.  For Top/Bottom: height in px.  Ignored for Center.
    pub size_px: f32,
    pub visible: bool,
    pub min_size_px: f32,
    pub max_size_px: Option<f32>,
}

impl PanelNode {
    pub fn sidebar(name: impl Into<String>, width: f32) -> Self {
        Self { name: name.into(), dock: Dock::Left, size_px: width, visible: true, min_size_px: 120.0, max_size_px: Some(480.0) }
    }
    pub fn toolbar(name: impl Into<String>, height: f32) -> Self {
        Self { name: name.into(), dock: Dock::Top, size_px: height, visible: true, min_size_px: 32.0, max_size_px: Some(96.0) }
    }
    pub fn statusbar(name: impl Into<String>, height: f32) -> Self {
        Self { name: name.into(), dock: Dock::Bottom, size_px: height, visible: true, min_size_px: 16.0, max_size_px: Some(48.0) }
    }
    pub fn right_pane(name: impl Into<String>, width: f32) -> Self {
        Self { name: name.into(), dock: Dock::Right, size_px: width, visible: true, min_size_px: 120.0, max_size_px: Some(600.0) }
    }
    pub fn center(name: impl Into<String>) -> Self {
        Self { name: name.into(), dock: Dock::Center, size_px: 0.0, visible: true, min_size_px: 0.0, max_size_px: None }
    }
    pub fn effective_size(&self) -> f32 {
        let clamped_max = self.max_size_px.unwrap_or(self.size_px);
        self.size_px.max(self.min_size_px).min(clamped_max)
    }
    pub fn set_visible(&mut self, visible: bool) { self.visible = visible; }
}

#[derive(Clone, Debug, PartialEq, Default)]
pub struct PanelTree { pub panels: Vec<PanelNode> }

impl PanelTree {
    pub fn new() -> Self { Self::default() }
    pub fn add(&mut self, node: PanelNode) { self.panels.push(node); }
    pub fn by_name(&self, name: &str) -> Option<&PanelNode> { self.panels.iter().find(|p| p.name == name) }

    /// Compute layout rects for every panel in the tree.  Docking priority:
    /// Top → Bottom → Left → Right → Center (the center fills what remains).
    pub fn solve(&self, viewport_w: f32, viewport_h: f32) -> Vec<(String, Rect)> {
        let mut x = 0f32;
        let mut y = 0f32;
        let mut w = viewport_w.max(0.0);
        let mut h = viewport_h.max(0.0);
        let mut out: Vec<(String, Rect)> = Vec::new();

        // Top
        for p in self.panels.iter().filter(|p| p.visible && p.dock == Dock::Top) {
            let s = p.effective_size().min(h);
            out.push((p.name.clone(), Rect::new(x, y, w, s)));
            y += s; h -= s;
        }
        // Bottom
        for p in self.panels.iter().filter(|p| p.visible && p.dock == Dock::Bottom) {
            let s = p.effective_size().min(h);
            out.push((p.name.clone(), Rect::new(x, y + h - s, w, s)));
            h -= s;
        }
        // Left
        for p in self.panels.iter().filter(|p| p.visible && p.dock == Dock::Left) {
            let s = p.effective_size().min(w);
            out.push((p.name.clone(), Rect::new(x, y, s, h)));
            x += s; w -= s;
        }
        // Right
        for p in self.panels.iter().filter(|p| p.visible && p.dock == Dock::Right) {
            let s = p.effective_size().min(w);
            out.push((p.name.clone(), Rect::new(x + w - s, y, s, h)));
            w -= s;
        }
        // Center
        for p in self.panels.iter().filter(|p| p.visible && p.dock == Dock::Center) {
            out.push((p.name.clone(), Rect::new(x, y, w, h)));
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // --- Rect tests ---

    #[test]
    fn rect_new_fields() {
        let r = Rect::new(1.0, 2.0, 3.0, 4.0);
        assert_eq!(r.x, 1.0); assert_eq!(r.y, 2.0);
        assert_eq!(r.w, 3.0); assert_eq!(r.h, 4.0);
    }

    #[test]
    fn rect_contains_inside() {
        let r = Rect::new(10.0, 10.0, 100.0, 100.0);
        assert!(r.contains(50.0, 50.0));
    }

    #[test]
    fn rect_contains_outside() {
        let r = Rect::new(10.0, 10.0, 100.0, 100.0);
        assert!(!r.contains(5.0, 5.0));
        assert!(!r.contains(200.0, 200.0));
    }

    #[test]
    fn rect_contains_boundary_inclusive_start_exclusive_end() {
        let r = Rect::new(0.0, 0.0, 100.0, 100.0);
        assert!(r.contains(0.0, 0.0));         // at origin — inside
        assert!(!r.contains(100.0, 50.0));     // at right edge — outside
        assert!(!r.contains(50.0, 100.0));     // at bottom edge — outside
    }

    #[test]
    fn rect_area() {
        assert_eq!(Rect::new(0.0, 0.0, 4.0, 5.0).area(), 20.0);
        assert_eq!(Rect::new(0.0, 0.0, 0.0, 5.0).area(), 0.0);
        assert_eq!(Rect::new(0.0, 0.0, -1.0, 5.0).area(), 0.0); // negative clamped
    }

    // --- PanelNode constructor tests ---

    #[test]
    fn panel_node_sidebar_constructor() {
        let p = PanelNode::sidebar("left", 200.0);
        assert_eq!(p.dock, Dock::Left);
        assert_eq!(p.size_px, 200.0);
        assert!(p.visible);
    }

    #[test]
    fn panel_node_toolbar_constructor() {
        let p = PanelNode::toolbar("top", 48.0);
        assert_eq!(p.dock, Dock::Top);
        assert_eq!(p.size_px, 48.0);
    }

    #[test]
    fn panel_node_statusbar_constructor() {
        let p = PanelNode::statusbar("status", 24.0);
        assert_eq!(p.dock, Dock::Bottom);
        assert_eq!(p.size_px, 24.0);
    }

    #[test]
    fn panel_node_right_pane_constructor() {
        let p = PanelNode::right_pane("props", 300.0);
        assert_eq!(p.dock, Dock::Right);
    }

    #[test]
    fn panel_node_center_constructor() {
        let p = PanelNode::center("canvas");
        assert_eq!(p.dock, Dock::Center);
        assert_eq!(p.size_px, 0.0);
    }

    // --- effective_size clamp tests ---

    #[test]
    fn effective_size_respects_min() {
        let mut p = PanelNode::sidebar("s", 50.0); // below min 120
        p.min_size_px = 120.0;
        assert_eq!(p.effective_size(), 120.0);
    }

    #[test]
    fn effective_size_respects_max() {
        let mut p = PanelNode::sidebar("s", 999.0); // above max 480
        p.max_size_px = Some(480.0);
        assert_eq!(p.effective_size(), 480.0);
    }

    #[test]
    fn effective_size_within_bounds() {
        let p = PanelNode::sidebar("s", 250.0);
        assert_eq!(p.effective_size(), 250.0);
    }

    // --- PanelTree tests ---

    #[test]
    fn panel_tree_new_empty() {
        let t = PanelTree::new();
        assert!(t.panels.is_empty());
    }

    #[test]
    fn panel_tree_add_and_by_name() {
        let mut t = PanelTree::new();
        t.add(PanelNode::center("main"));
        assert!(t.by_name("main").is_some());
        assert!(t.by_name("missing").is_none());
    }

    // --- solve tests ---

    #[test]
    fn solve_empty_tree_returns_empty() {
        let t = PanelTree::new();
        assert!(t.solve(800.0, 600.0).is_empty());
    }

    #[test]
    fn solve_toolbar_only() {
        let mut t = PanelTree::new();
        t.add(PanelNode::toolbar("top", 48.0));
        let rects = t.solve(800.0, 600.0);
        assert_eq!(rects.len(), 1);
        let (name, r) = &rects[0];
        assert_eq!(name, "top");
        assert_eq!(*r, Rect::new(0.0, 0.0, 800.0, 48.0));
    }

    #[test]
    fn solve_full_shell_layout() {
        // toolbar 48px top, statusbar 24px bottom, sidebar 248px left, center fills rest
        let mut t = PanelTree::new();
        t.add(PanelNode::toolbar("toolbar", 48.0));
        t.add(PanelNode::statusbar("statusbar", 24.0));
        t.add(PanelNode::sidebar("sidebar", 248.0));
        t.add(PanelNode::center("canvas"));
        let rects = t.solve(800.0, 600.0);
        assert_eq!(rects.len(), 4);

        let get = |name: &str| rects.iter().find(|(n, _)| n == name).map(|(_, r)| *r).unwrap();

        // toolbar: full width at top
        assert_eq!(get("toolbar"), Rect::new(0.0, 0.0, 800.0, 48.0));
        // statusbar: full width at bottom (y = 600 - 24 = 576)
        assert_eq!(get("statusbar"), Rect::new(0.0, 576.0, 800.0, 24.0));
        // sidebar: left, starts at y=48, height = 600-48-24 = 528
        assert_eq!(get("sidebar"), Rect::new(0.0, 48.0, 248.0, 528.0));
        // center: x=248, y=48, w=800-248=552, h=528
        assert_eq!(get("canvas"), Rect::new(248.0, 48.0, 552.0, 528.0));
    }

    #[test]
    fn solve_invisible_panel_excluded() {
        let mut t = PanelTree::new();
        let mut p = PanelNode::sidebar("hidden", 200.0);
        p.set_visible(false);
        t.add(p);
        t.add(PanelNode::center("canvas"));
        let rects = t.solve(800.0, 600.0);
        assert!(rects.iter().all(|(n, _)| n != "hidden"));
        // center gets full viewport
        let (_, r) = rects.iter().find(|(n, _)| n == "canvas").unwrap();
        assert_eq!(*r, Rect::new(0.0, 0.0, 800.0, 600.0));
    }

    #[test]
    fn solve_zero_viewport_no_crash() {
        let mut t = PanelTree::new();
        t.add(PanelNode::toolbar("toolbar", 48.0));
        t.add(PanelNode::center("canvas"));
        let rects = t.solve(0.0, 0.0);
        // all rects should be zero-sized, no panic
        for (_, r) in &rects {
            assert_eq!(r.area(), 0.0);
        }
    }

    #[test]
    fn solve_oversized_panel_clamped_to_viewport() {
        let mut t = PanelTree::new();
        // sidebar wants 999px but viewport is only 300px wide; effective_size = 480 (max), then clamped to 300
        let mut p = PanelNode::sidebar("huge", 999.0);
        p.max_size_px = Some(999.0); // lift max so effective_size = 999, viewport clamps it
        t.add(p);
        t.add(PanelNode::center("canvas"));
        let rects = t.solve(300.0, 400.0);
        let get = |name: &str| rects.iter().find(|(n, _)| n == name).map(|(_, r)| *r).unwrap();
        // sidebar width clamped to 300 (the full viewport width)
        assert_eq!(get("huge").w, 300.0);
        // center gets 0 width
        assert_eq!(get("canvas").w, 0.0);
    }
}
