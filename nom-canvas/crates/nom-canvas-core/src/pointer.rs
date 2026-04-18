/// Pointer (mouse / touch) capture state machine for canvas interactions.

/// The current state of the pointer on the canvas.
#[derive(Debug, Clone, PartialEq)]
pub enum PointerState {
    /// No button pressed, no drag in progress.
    Idle,
    /// A button was pressed on an element but not yet moved enough to be a drag.
    Pressed {
        /// The element that was pressed.
        element_id: String,
        /// Canvas-space position where the press occurred.
        start: (f32, f32),
    },
    /// The pointer is being dragged after a press.
    Dragging {
        /// The element being dragged.
        element_id: String,
        /// Canvas-space position where the drag started.
        start: (f32, f32),
        /// Current canvas-space position of the pointer.
        current: (f32, f32),
    },
}

impl PointerState {
    /// Creates the initial idle state.
    pub fn new() -> Self {
        Self::Idle
    }

    /// Transition: press an element.
    ///
    /// From `Idle` → `Pressed`.
    /// From `Pressed` or `Dragging` (press on a different element while one is
    /// already pressed) → `Pressed` with the new element (replaces existing press).
    pub fn on_press(self, element_id: impl Into<String>, pos: (f32, f32)) -> Self {
        Self::Pressed {
            element_id: element_id.into(),
            start: pos,
        }
    }

    /// Transition: move the pointer.
    ///
    /// `Idle` → `Idle` (no movement recorded without a press).
    /// `Pressed` → `Dragging` (first move after press begins a drag).
    /// `Dragging` → `Dragging` (current position updated).
    pub fn on_move(self, pos: (f32, f32)) -> Self {
        match self {
            Self::Idle => Self::Idle,
            Self::Pressed { element_id, start } => Self::Dragging {
                element_id,
                start,
                current: pos,
            },
            Self::Dragging {
                element_id,
                start,
                current: _,
            } => Self::Dragging {
                element_id,
                start,
                current: pos,
            },
        }
    }

    /// Transition: release the pointer.
    ///
    /// Any state → `Idle`.
    pub fn on_release(self) -> Self {
        Self::Idle
    }

    /// Returns the drag delta `(dx, dy)` if the pointer is currently dragging,
    /// otherwise `None`.
    pub fn drag_delta(&self) -> Option<(f32, f32)> {
        if let Self::Dragging { start, current, .. } = self {
            Some((current.0 - start.0, current.1 - start.1))
        } else {
            None
        }
    }

    /// Returns `true` if the pointer is in the `Idle` state.
    pub fn is_idle(&self) -> bool {
        matches!(self, Self::Idle)
    }

    /// Returns `true` if the pointer is in the `Pressed` state.
    pub fn is_pressed(&self) -> bool {
        matches!(self, Self::Pressed { .. })
    }

    /// Returns `true` if the pointer is in the `Dragging` state.
    pub fn is_dragging(&self) -> bool {
        matches!(self, Self::Dragging { .. })
    }
}

impl Default for PointerState {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── basic transitions ─────────────────────────────────────────────────────

    #[test]
    fn initial_state_is_idle() {
        let s = PointerState::new();
        assert!(s.is_idle());
        assert!(!s.is_pressed());
        assert!(!s.is_dragging());
    }

    #[test]
    fn idle_to_pressed_on_press() {
        let s = PointerState::Idle.on_press("elem1", (10.0, 20.0));
        assert!(s.is_pressed());
        assert!(!s.is_idle());
        assert!(!s.is_dragging());
    }

    #[test]
    fn pressed_to_dragging_on_move() {
        let s = PointerState::Idle
            .on_press("elem1", (0.0, 0.0))
            .on_move((5.0, 5.0));
        assert!(s.is_dragging());
    }

    #[test]
    fn dragging_to_idle_on_release() {
        let s = PointerState::Idle
            .on_press("a", (0.0, 0.0))
            .on_move((10.0, 10.0))
            .on_release();
        assert!(s.is_idle());
    }

    #[test]
    fn pressed_to_idle_on_release() {
        let s = PointerState::Idle.on_press("elem", (5.0, 5.0)).on_release();
        assert!(s.is_idle());
    }

    // ── drag delta ────────────────────────────────────────────────────────────

    #[test]
    fn drag_delta_while_dragging() {
        let s = PointerState::Idle
            .on_press("elem", (10.0, 20.0))
            .on_move((13.0, 17.0));
        let delta = s.drag_delta().unwrap();
        assert!((delta.0 - 3.0).abs() < 1e-6, "dx={}", delta.0);
        assert!((delta.1 - (-3.0)).abs() < 1e-6, "dy={}", delta.1);
    }

    #[test]
    fn drag_delta_returns_none_when_idle() {
        let s = PointerState::Idle;
        assert!(s.drag_delta().is_none());
    }

    #[test]
    fn drag_delta_returns_none_when_pressed() {
        let s = PointerState::Idle.on_press("a", (0.0, 0.0));
        assert!(s.drag_delta().is_none());
    }

    #[test]
    fn drag_delta_updates_on_successive_moves() {
        let s = PointerState::Idle
            .on_press("el", (0.0, 0.0))
            .on_move((5.0, 5.0)) // first move: delta (5, 5)
            .on_move((15.0, 3.0)); // second move: delta from start still measured
        let delta = s.drag_delta().unwrap();
        assert!((delta.0 - 15.0).abs() < 1e-6, "dx={}", delta.0);
        assert!((delta.1 - 3.0).abs() < 1e-6, "dy={}", delta.1);
    }

    // ── idle ignores move ─────────────────────────────────────────────────────

    #[test]
    fn idle_on_move_stays_idle() {
        let s = PointerState::Idle.on_move((100.0, 200.0));
        assert!(
            s.is_idle(),
            "idle state must not change on move without press"
        );
    }

    // ── press on different element while already pressed ─────────────────────

    #[test]
    fn press_on_different_element_while_pressed_replaces_press() {
        let s = PointerState::Idle
            .on_press("elem_a", (0.0, 0.0))
            .on_press("elem_b", (50.0, 50.0));
        // State must be Pressed on elem_b.
        assert!(s.is_pressed());
        if let PointerState::Pressed { element_id, start } = &s {
            assert_eq!(element_id, "elem_b");
            assert!((start.0 - 50.0).abs() < 1e-6);
        } else {
            panic!("expected Pressed state");
        }
    }

    #[test]
    fn press_on_different_element_while_dragging_replaces_with_new_press() {
        let s = PointerState::Idle
            .on_press("elem_a", (0.0, 0.0))
            .on_move((10.0, 10.0)) // now Dragging elem_a
            .on_press("elem_b", (20.0, 30.0)); // new press should cancel drag
        assert!(s.is_pressed());
        if let PointerState::Pressed { element_id, start } = &s {
            assert_eq!(element_id, "elem_b");
            assert!((start.0 - 20.0).abs() < 1e-6);
            assert!((start.1 - 30.0).abs() < 1e-6);
        } else {
            panic!("expected Pressed state for elem_b");
        }
    }

    // ── full lifecycle ────────────────────────────────────────────────────────

    #[test]
    fn full_lifecycle_idle_pressed_dragging_idle() {
        let s0 = PointerState::Idle;
        assert!(s0.is_idle());

        let s1 = s0.on_press("node1", (100.0, 100.0));
        assert!(s1.is_pressed());

        let s2 = s1.on_move((110.0, 120.0));
        assert!(s2.is_dragging());
        let d = s2.drag_delta().unwrap();
        assert!((d.0 - 10.0).abs() < 1e-6);
        assert!((d.1 - 20.0).abs() < 1e-6);

        let s3 = s2.on_release();
        assert!(s3.is_idle());
    }

    #[test]
    fn cannot_go_idle_to_dragging_directly() {
        // A move from Idle must not produce a Dragging state.
        let s = PointerState::Idle.on_move((5.0, 5.0));
        assert!(
            !s.is_dragging(),
            "Idle→move must not produce Dragging state"
        );
        assert!(s.is_idle());
    }
}
