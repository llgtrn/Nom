//! Pointer event primitives — mouse, pen, touch — OS-independent.
#![deny(unsafe_code)]

use crate::geometry::{Pixels, Point};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum PointerKind { Mouse, Pen, Touch }

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum MouseButton { Left, Right, Middle, Aux1, Aux2 }

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum PointerPhase { Down, Move, Up, Enter, Leave, Cancel }

/// Modifier key bitmask (shift=1, ctrl=2, alt=4, meta=8).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct Modifiers(pub u8);

impl Modifiers {
    pub const SHIFT: u8 = 1;
    pub const CTRL: u8 = 2;
    pub const ALT: u8 = 4;
    pub const META: u8 = 8;

    pub fn new() -> Self { Self(0) }
    pub fn with(mut self, flag: u8) -> Self { self.0 |= flag; self }
    pub fn has(self, flag: u8) -> bool { self.0 & flag != 0 }
    pub fn is_empty(self) -> bool { self.0 == 0 }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PointerEvent {
    pub kind: PointerKind,
    pub phase: PointerPhase,
    pub position: Point<Pixels>,
    pub button: Option<MouseButton>,
    pub pressure: f32,          // 0.0..=1.0, always 1.0 for mouse
    pub tilt_x: f32,            // -1.0..=1.0 radians (approximate)
    pub tilt_y: f32,
    pub modifiers: Modifiers,
    pub click_count: u32,       // 1 for single, 2 for double, 3 for triple
}

impl PointerEvent {
    pub fn mouse_down(position: Point<Pixels>, button: MouseButton) -> Self {
        Self {
            kind: PointerKind::Mouse, phase: PointerPhase::Down, position,
            button: Some(button), pressure: 1.0, tilt_x: 0.0, tilt_y: 0.0,
            modifiers: Modifiers::default(), click_count: 1,
        }
    }

    pub fn mouse_move(position: Point<Pixels>) -> Self {
        Self {
            kind: PointerKind::Mouse, phase: PointerPhase::Move, position,
            button: None, pressure: 1.0, tilt_x: 0.0, tilt_y: 0.0,
            modifiers: Modifiers::default(), click_count: 0,
        }
    }

    pub fn pen_sample(position: Point<Pixels>, pressure: f32) -> Self {
        Self {
            kind: PointerKind::Pen, phase: PointerPhase::Move, position,
            button: None, pressure: pressure.clamp(0.0, 1.0),
            tilt_x: 0.0, tilt_y: 0.0,
            modifiers: Modifiers::default(), click_count: 0,
        }
    }

    pub fn with_modifiers(mut self, modifiers: Modifiers) -> Self { self.modifiers = modifiers; self }
    pub fn with_click_count(mut self, count: u32) -> Self { self.click_count = count; self }
    pub fn with_tilt(mut self, x: f32, y: f32) -> Self {
        self.tilt_x = x.clamp(-1.0, 1.0);
        self.tilt_y = y.clamp(-1.0, 1.0);
        self
    }

    pub fn is_click(&self) -> bool { self.phase == PointerPhase::Up && self.click_count >= 1 }
    pub fn is_double_click(&self) -> bool { self.is_click() && self.click_count == 2 }
    pub fn is_drag(&self) -> bool { self.phase == PointerPhase::Move && self.button.is_some() }
}

/// Click-detection state machine: converts a stream of Down/Up events into
/// `click_count` values (single/double/triple).  Resets after 500ms idle.
pub struct ClickCounter {
    last_down_ms: u64,
    last_position: Point<Pixels>,
    consecutive: u32,
    threshold_ms: u64,
    threshold_px: f32,
}

impl ClickCounter {
    pub fn new(threshold_ms: u64, threshold_px: f32) -> Self {
        Self {
            last_down_ms: 0, last_position: Point { x: Pixels(0.0), y: Pixels(0.0) },
            consecutive: 0, threshold_ms, threshold_px,
        }
    }

    pub fn observe_down(&mut self, position: Point<Pixels>, now_ms: u64) -> u32 {
        let dx = position.x.0 - self.last_position.x.0;
        let dy = position.y.0 - self.last_position.y.0;
        let distance_sq = dx * dx + dy * dy;
        if now_ms.saturating_sub(self.last_down_ms) <= self.threshold_ms
            && distance_sq <= self.threshold_px * self.threshold_px
        {
            self.consecutive += 1;
        } else {
            self.consecutive = 1;
        }
        self.last_down_ms = now_ms;
        self.last_position = position;
        self.consecutive
    }

    pub fn reset(&mut self) {
        self.consecutive = 0;
        self.last_down_ms = 0;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::geometry::{Pixels, Point};

    fn pt(x: f32, y: f32) -> Point<Pixels> {
        Point { x: Pixels(x), y: Pixels(y) }
    }

    // --- Modifiers ---

    #[test]
    fn modifiers_new_is_empty() {
        assert!(Modifiers::new().is_empty());
    }

    #[test]
    fn modifiers_with_sets_bit_and_has_returns_true() {
        let m = Modifiers::new().with(Modifiers::SHIFT);
        assert!(m.has(Modifiers::SHIFT));
    }

    #[test]
    fn modifiers_has_returns_false_for_unset() {
        let m = Modifiers::new().with(Modifiers::SHIFT);
        assert!(!m.has(Modifiers::CTRL));
    }

    #[test]
    fn modifiers_shift_ctrl_bitmask_3_has_both() {
        let m = Modifiers::new().with(Modifiers::SHIFT).with(Modifiers::CTRL);
        assert_eq!(m.0, 3);
        assert!(m.has(Modifiers::SHIFT));
        assert!(m.has(Modifiers::CTRL));
    }

    // --- PointerEvent constructors ---

    #[test]
    fn mouse_down_correct_fields() {
        let ev = PointerEvent::mouse_down(pt(10.0, 20.0), MouseButton::Left);
        assert_eq!(ev.kind, PointerKind::Mouse);
        assert_eq!(ev.phase, PointerPhase::Down);
        assert_eq!(ev.pressure, 1.0);
        assert_eq!(ev.button, Some(MouseButton::Left));
        assert_eq!(ev.click_count, 1);
    }

    #[test]
    fn mouse_move_no_button() {
        let ev = PointerEvent::mouse_move(pt(5.0, 5.0));
        assert_eq!(ev.button, None);
        assert_eq!(ev.phase, PointerPhase::Move);
    }

    #[test]
    fn pen_sample_clamps_pressure_above_one() {
        let ev = PointerEvent::pen_sample(pt(0.0, 0.0), 1.5);
        assert_eq!(ev.pressure, 1.0);
    }

    #[test]
    fn pen_sample_clamps_pressure_below_zero() {
        let ev = PointerEvent::pen_sample(pt(0.0, 0.0), -0.5);
        assert_eq!(ev.pressure, 0.0);
    }

    #[test]
    fn builder_chain_with_modifiers_click_count_tilt() {
        let m = Modifiers::new().with(Modifiers::ALT);
        let ev = PointerEvent::mouse_down(pt(0.0, 0.0), MouseButton::Right)
            .with_modifiers(m)
            .with_click_count(2)
            .with_tilt(0.5, -0.5);
        assert_eq!(ev.modifiers, m);
        assert_eq!(ev.click_count, 2);
        assert_eq!(ev.tilt_x, 0.5);
        assert_eq!(ev.tilt_y, -0.5);
    }

    #[test]
    fn with_tilt_clamps_to_minus_one_plus_one() {
        let ev = PointerEvent::mouse_move(pt(0.0, 0.0)).with_tilt(2.0, -3.0);
        assert_eq!(ev.tilt_x, 1.0);
        assert_eq!(ev.tilt_y, -1.0);
    }

    #[test]
    fn is_click_true_on_up_with_count_gte_1() {
        let ev = PointerEvent::mouse_down(pt(0.0, 0.0), MouseButton::Left)
            .with_click_count(1);
        // phase is Down — not a click yet
        assert!(!ev.is_click());
        // simulate Up
        let mut up = ev;
        up.phase = PointerPhase::Up;
        assert!(up.is_click());
    }

    #[test]
    fn is_double_click_true_only_when_count_2() {
        let mk = |count: u32| {
            let mut ev = PointerEvent::mouse_down(pt(0.0, 0.0), MouseButton::Left)
                .with_click_count(count);
            ev.phase = PointerPhase::Up;
            ev
        };
        assert!(!mk(1).is_double_click());
        assert!(mk(2).is_double_click());
        assert!(!mk(3).is_double_click());
    }

    #[test]
    fn is_drag_requires_move_and_button() {
        let no_button = PointerEvent::mouse_move(pt(0.0, 0.0));
        assert!(!no_button.is_drag());

        let mut with_button = no_button;
        with_button.button = Some(MouseButton::Left);
        assert!(with_button.is_drag());

        let mut down = with_button;
        down.phase = PointerPhase::Down;
        assert!(!down.is_drag());
    }

    // --- ClickCounter ---

    #[test]
    fn click_counter_first_click_returns_1() {
        let mut cc = ClickCounter::new(500, 5.0);
        assert_eq!(cc.observe_down(pt(10.0, 10.0), 1000), 1);
    }

    #[test]
    fn click_counter_two_rapid_same_pos_returns_2() {
        let mut cc = ClickCounter::new(500, 5.0);
        cc.observe_down(pt(10.0, 10.0), 1000);
        assert_eq!(cc.observe_down(pt(10.0, 10.0), 1200), 2);
    }

    #[test]
    fn click_counter_three_rapid_same_pos_returns_3() {
        let mut cc = ClickCounter::new(500, 5.0);
        cc.observe_down(pt(10.0, 10.0), 1000);
        cc.observe_down(pt(10.0, 10.0), 1200);
        assert_eq!(cc.observe_down(pt(10.0, 10.0), 1400), 3);
    }

    #[test]
    fn click_counter_too_slow_resets_to_1() {
        let mut cc = ClickCounter::new(500, 5.0);
        cc.observe_down(pt(10.0, 10.0), 1000);
        assert_eq!(cc.observe_down(pt(10.0, 10.0), 2000), 1);
    }

    #[test]
    fn click_counter_too_far_resets_to_1() {
        let mut cc = ClickCounter::new(500, 5.0);
        cc.observe_down(pt(10.0, 10.0), 1000);
        assert_eq!(cc.observe_down(pt(100.0, 100.0), 1200), 1);
    }

    #[test]
    fn click_counter_reset_clears_consecutive() {
        let mut cc = ClickCounter::new(500, 5.0);
        cc.observe_down(pt(10.0, 10.0), 1000);
        cc.observe_down(pt(10.0, 10.0), 1200);
        cc.reset();
        assert_eq!(cc.observe_down(pt(10.0, 10.0), 1400), 1);
    }
}
