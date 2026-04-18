/// Which side of the canvas a panel is anchored to.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PanelSide {
    Left,
    Right,
    Bottom,
    Center,
}

impl PanelSide {
    pub fn side_name(&self) -> &str {
        match self {
            PanelSide::Left => "left",
            PanelSide::Right => "right",
            PanelSide::Bottom => "bottom",
            PanelSide::Center => "center",
        }
    }

    /// Returns true for Left and Right (sidebar) panels.
    pub fn is_sidebar(&self) -> bool {
        matches!(self, PanelSide::Left | PanelSide::Right)
    }
}

// ---------------------------------------------------------------------------
// PanelState
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub struct PanelState {
    pub side: PanelSide,
    pub width_px: u32,
    pub is_visible: bool,
    pub is_pinned: bool,
}

impl PanelState {
    pub fn new(side: PanelSide, width_px: u32) -> Self {
        Self {
            side,
            width_px,
            is_visible: true,
            is_pinned: false,
        }
    }

    pub fn toggle_visibility(&mut self) {
        self.is_visible = !self.is_visible;
    }

    pub fn pin(&mut self) {
        self.is_pinned = true;
    }

    /// Resize clamped to [80, 800].
    pub fn resize(&mut self, new_width: u32) {
        self.width_px = new_width.clamp(80, 800);
    }
}

// ---------------------------------------------------------------------------
// ResizeHandle
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub struct ResizeHandle {
    pub panel_side: PanelSide,
    pub drag_start_px: Option<u32>,
}

impl ResizeHandle {
    pub fn new(panel_side: PanelSide) -> Self {
        Self {
            panel_side,
            drag_start_px: None,
        }
    }

    pub fn start_drag(&mut self, x: u32) {
        self.drag_start_px = Some(x);
    }

    /// Returns the drag start position and clears it.
    pub fn end_drag(&mut self) -> Option<u32> {
        self.drag_start_px.take()
    }

    pub fn is_dragging(&self) -> bool {
        self.drag_start_px.is_some()
    }

    /// Returns `current_x - drag_start_px` as i32, or None if not dragging.
    pub fn delta(&self, current_x: u32) -> Option<i32> {
        self.drag_start_px
            .map(|start| current_x as i32 - start as i32)
    }
}

// ---------------------------------------------------------------------------
// LayoutSnapshot
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub struct LayoutSnapshot {
    pub panels: Vec<PanelState>,
}

impl LayoutSnapshot {
    pub fn new() -> Self {
        Self { panels: Vec::new() }
    }

    pub fn capture(panels: &[PanelState]) -> Self {
        Self {
            panels: panels.to_vec(),
        }
    }

    pub fn panel_count(&self) -> usize {
        self.panels.len()
    }

    pub fn visible_count(&self) -> usize {
        self.panels.iter().filter(|p| p.is_visible).count()
    }

    pub fn restore_widths(&self) -> Vec<u32> {
        self.panels.iter().map(|p| p.width_px).collect()
    }
}

impl Default for LayoutSnapshot {
    fn default() -> Self {
        Self::new()
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod layout_state_tests {
    use super::*;

    #[test]
    fn panel_side_is_sidebar() {
        assert!(PanelSide::Left.is_sidebar());
        assert!(PanelSide::Right.is_sidebar());
        assert!(!PanelSide::Bottom.is_sidebar());
        assert!(!PanelSide::Center.is_sidebar());
    }

    #[test]
    fn panel_state_toggle_visibility() {
        let mut panel = PanelState::new(PanelSide::Left, 240);
        assert!(panel.is_visible);
        panel.toggle_visibility();
        assert!(!panel.is_visible);
        panel.toggle_visibility();
        assert!(panel.is_visible);
    }

    #[test]
    fn panel_state_resize_clamps() {
        let mut panel = PanelState::new(PanelSide::Left, 240);
        panel.resize(50);
        assert_eq!(panel.width_px, 80, "below minimum should clamp to 80");
        panel.resize(1000);
        assert_eq!(panel.width_px, 800, "above maximum should clamp to 800");
        panel.resize(300);
        assert_eq!(panel.width_px, 300, "in-range value should be kept");
    }

    #[test]
    fn resize_handle_start_drag_and_is_dragging() {
        let mut handle = ResizeHandle::new(PanelSide::Left);
        assert!(!handle.is_dragging());
        handle.start_drag(120);
        assert!(handle.is_dragging());
        assert_eq!(handle.drag_start_px, Some(120));
    }

    #[test]
    fn resize_handle_end_drag_clears_state() {
        let mut handle = ResizeHandle::new(PanelSide::Right);
        handle.start_drag(200);
        let returned = handle.end_drag();
        assert_eq!(returned, Some(200));
        assert!(!handle.is_dragging());
        assert_eq!(handle.drag_start_px, None);
    }

    #[test]
    fn resize_handle_delta_calculation() {
        let mut handle = ResizeHandle::new(PanelSide::Left);
        // Not dragging — delta returns None.
        assert_eq!(handle.delta(300), None);
        handle.start_drag(100);
        assert_eq!(handle.delta(150), Some(50));
        assert_eq!(handle.delta(80), Some(-20));
    }

    #[test]
    fn layout_snapshot_visible_count() {
        let mut panels = vec![
            PanelState::new(PanelSide::Left, 240),
            PanelState::new(PanelSide::Right, 300),
            PanelState::new(PanelSide::Bottom, 200),
        ];
        panels[1].toggle_visibility(); // hide right panel
        let snap = LayoutSnapshot::capture(&panels);
        assert_eq!(snap.visible_count(), 2);
    }

    #[test]
    fn layout_snapshot_restore_widths() {
        let panels = vec![
            PanelState::new(PanelSide::Left, 240),
            PanelState::new(PanelSide::Right, 320),
            PanelState::new(PanelSide::Bottom, 180),
        ];
        let snap = LayoutSnapshot::capture(&panels);
        assert_eq!(snap.restore_widths(), vec![240, 320, 180]);
    }

    #[test]
    fn panel_state_pin_sets_flag() {
        let mut panel = PanelState::new(PanelSide::Left, 240);
        assert!(!panel.is_pinned);
        panel.pin();
        assert!(panel.is_pinned);
    }
}
