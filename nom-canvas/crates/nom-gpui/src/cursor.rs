//! Text-cursor primitive: position + blink phase + rendering style.
#![deny(unsafe_code)]

use std::time::Instant;
#[cfg(test)]
use std::time::Duration;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CursorShape {
    /// Thin vertical bar (insert mode).
    Bar,
    /// Full-cell block (overwrite / vim normal mode).
    Block,
    /// Underscore at baseline (vim insert in some shells).
    Underline,
    /// Hollow outline (unfocused window).
    HollowBlock,
}

#[derive(Clone, Debug, PartialEq)]
pub struct CursorStyle {
    pub shape: CursorShape,
    pub width_px: f32,   // meaningful for Bar shape; others derive from cell
    pub blink_on_ms: u32,
    pub blink_off_ms: u32,
    pub blink_enabled: bool,
}

impl Default for CursorStyle {
    fn default() -> Self {
        Self {
            shape: CursorShape::Bar,
            width_px: 1.5,
            blink_on_ms: 530,
            blink_off_ms: 530,
            blink_enabled: true,
        }
    }
}

impl CursorStyle {
    pub fn cycle_ms(&self) -> u32 {
        self.blink_on_ms + self.blink_off_ms
    }
    pub fn with_shape(mut self, shape: CursorShape) -> Self {
        self.shape = shape;
        self
    }
    pub fn without_blink(mut self) -> Self {
        self.blink_enabled = false;
        self
    }
}

pub struct Cursor {
    pub row: u32,
    pub column: u32,
    style: CursorStyle,
    started_at: Instant,
    focused: bool,
}

impl Cursor {
    pub fn new(row: u32, column: u32, style: CursorStyle) -> Self {
        Self { row, column, style, started_at: Instant::now(), focused: true }
    }

    pub fn set_position(&mut self, row: u32, column: u32) {
        self.row = row;
        self.column = column;
        // Restart the blink cycle so a newly-moved cursor is immediately visible.
        self.started_at = Instant::now();
    }

    pub fn set_focused(&mut self, focused: bool) {
        self.focused = focused;
        if focused {
            // Restart blink on regaining focus.
            self.started_at = Instant::now();
        }
    }

    pub fn is_focused(&self) -> bool {
        self.focused
    }

    pub fn style(&self) -> &CursorStyle {
        &self.style
    }

    /// Returns true when the cursor should be drawn solid at `now`.  An
    /// unfocused cursor draws as a hollow outline (caller decides); this
    /// method returns `false` for an unfocused+blink-enabled cursor so that
    /// non-blink renders stay visible but blink-capable renders go invisible.
    pub fn is_visible_at(&self, now: Instant) -> bool {
        if !self.style.blink_enabled {
            return self.focused;
        }
        if !self.focused {
            return false;
        }
        let cycle = self.style.cycle_ms().max(1) as u128;
        let elapsed = now.saturating_duration_since(self.started_at).as_millis();
        let phase = (elapsed % cycle) as u32;
        phase < self.style.blink_on_ms
    }

    /// Current phase as a `[0.0..=1.0]` value; 0 = start of on, 0.5 = start of off.
    pub fn phase_at(&self, now: Instant) -> f32 {
        let cycle = self.style.cycle_ms().max(1) as f32;
        let elapsed = now.saturating_duration_since(self.started_at).as_millis() as f32;
        (elapsed % cycle) / cycle
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_style_is_bar_with_blink() {
        let s = CursorStyle::default();
        assert_eq!(s.shape, CursorShape::Bar);
        assert!(s.blink_enabled);
    }

    #[test]
    fn cycle_ms_sums_on_and_off() {
        let s = CursorStyle::default();
        assert_eq!(s.cycle_ms(), s.blink_on_ms + s.blink_off_ms);
        assert_eq!(s.cycle_ms(), 1060);
    }

    #[test]
    fn with_shape_builder_changes_shape() {
        let s = CursorStyle::default().with_shape(CursorShape::Block);
        assert_eq!(s.shape, CursorShape::Block);
    }

    #[test]
    fn without_blink_disables_blink() {
        let s = CursorStyle::default().without_blink();
        assert!(!s.blink_enabled);
    }

    #[test]
    fn new_stores_position_and_focused() {
        let c = Cursor::new(3, 7, CursorStyle::default());
        assert_eq!(c.row, 3);
        assert_eq!(c.column, 7);
        assert!(c.is_focused());
    }

    #[test]
    fn set_position_updates_row_col_and_restarts_blink() {
        let mut c = Cursor::new(0, 0, CursorStyle::default());
        c.set_position(5, 10);
        assert_eq!(c.row, 5);
        assert_eq!(c.column, 10);
        // Immediately after set_position, the cursor should be visible (phase 0).
        let now = Instant::now();
        assert!(c.is_visible_at(now));
    }

    #[test]
    fn set_focused_false_with_blink_enabled_is_invisible() {
        let mut c = Cursor::new(0, 0, CursorStyle::default());
        c.set_focused(false);
        assert!(!c.is_visible_at(Instant::now()));
    }

    #[test]
    fn set_focused_false_with_blink_disabled_is_also_invisible() {
        let mut c = Cursor::new(0, 0, CursorStyle::default().without_blink());
        c.set_focused(false);
        assert!(!c.is_visible_at(Instant::now()));
    }

    #[test]
    fn focused_blink_enabled_at_time_zero_is_visible() {
        let mut c = Cursor::new(0, 0, CursorStyle::default());
        c.set_focused(true);
        let now = Instant::now();
        assert!(c.is_visible_at(now));
    }

    #[test]
    fn focused_blink_enabled_past_blink_on_ms_is_invisible() {
        let mut c = Cursor::new(0, 0, CursorStyle::default());
        c.set_focused(true);
        // started_at is set inside set_focused; use a future instant well past blink_on_ms.
        let after_on = Instant::now() + Duration::from_millis(531);
        assert!(!c.is_visible_at(after_on));
    }

    #[test]
    fn focused_blink_enabled_past_full_cycle_is_visible_again() {
        let mut c = Cursor::new(0, 0, CursorStyle::default());
        c.set_focused(true);
        // After one full cycle (1060ms) + 1ms we are back in the on-phase.
        let after_cycle = Instant::now() + Duration::from_millis(1061);
        assert!(c.is_visible_at(after_cycle));
    }

    #[test]
    fn blink_disabled_focused_always_visible() {
        let mut c = Cursor::new(0, 0, CursorStyle::default().without_blink());
        c.set_focused(true);
        let base = Instant::now();
        assert!(c.is_visible_at(base));
        assert!(c.is_visible_at(base + Duration::from_millis(5000)));
    }

    #[test]
    fn phase_at_time_zero_returns_zero() {
        let mut c = Cursor::new(0, 0, CursorStyle::default());
        c.set_focused(true);
        let now = Instant::now();
        let p = c.phase_at(now);
        assert!(p >= 0.0 && p < 0.1, "phase at time 0 should be near 0, got {p}");
    }

    #[test]
    fn phase_at_half_cycle_returns_approx_half() {
        let mut c = Cursor::new(0, 0, CursorStyle::default());
        c.set_focused(true);
        let half = Instant::now() + Duration::from_millis(530);
        let p = c.phase_at(half);
        assert!((p - 0.5).abs() < 0.05, "phase at half cycle should be ~0.5, got {p}");
    }
}
