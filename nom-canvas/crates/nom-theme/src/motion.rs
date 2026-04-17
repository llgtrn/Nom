//! Motion design tokens — standard durations + easing presets.
//!
//! These are pure values (f32 milliseconds + 4-float cubic-bezier controls).
//! The renderer (nom-gpui::animation) converts them to Easing::CubicBezier
//! and Duration as needed — this module stays dep-free.
#![deny(unsafe_code)]

/// Standard motion durations (milliseconds).  Pick based on spatial extent
/// of the change: short for micro-interactions, long for full-screen.
pub const DURATION_SHORT_MS: u32 = 150;
pub const DURATION_MEDIUM_MS: u32 = 250;
pub const DURATION_LONG_MS: u32 = 400;
pub const DURATION_EXTRA_LONG_MS: u32 = 700;
/// Specifically for mode-switch full-screen transitions.
pub const DURATION_MODE_SWITCH_MS: u32 = 350;

/// Control points for a cubic Bezier (0,0)→(x1,y1)→(x2,y2)→(1,1).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BezierControl {
    pub x1: f32,
    pub y1: f32,
    pub x2: f32,
    pub y2: f32,
}

impl BezierControl {
    pub const fn new(x1: f32, y1: f32, x2: f32, y2: f32) -> Self { Self { x1, y1, x2, y2 } }

    /// Material-style standard curve.
    pub const STANDARD: Self = Self::new(0.4, 0.0, 0.2, 1.0);
    /// Entrance (deceleration).
    pub const DECELERATE: Self = Self::new(0.0, 0.0, 0.2, 1.0);
    /// Exit (acceleration).
    pub const ACCELERATE: Self = Self::new(0.4, 0.0, 1.0, 1.0);
    /// Emphasised — overshoot + settle.  Matches blueprint §5 mode-switch preset.
    pub const EMPHASISED: Self = Self::new(0.27, 0.2, 0.25, 1.51);
    /// Linear (no easing).
    pub const LINEAR: Self = Self::new(0.0, 0.0, 1.0, 1.0);
}

/// Motion level enum for pick-by-role helpers.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MotionLevel { Short, Medium, Long, ExtraLong, ModeSwitch }

impl MotionLevel {
    pub fn duration_ms(self) -> u32 {
        match self {
            Self::Short => DURATION_SHORT_MS,
            Self::Medium => DURATION_MEDIUM_MS,
            Self::Long => DURATION_LONG_MS,
            Self::ExtraLong => DURATION_EXTRA_LONG_MS,
            Self::ModeSwitch => DURATION_MODE_SWITCH_MS,
        }
    }
    pub fn default_bezier(self) -> BezierControl {
        match self {
            Self::Short => BezierControl::DECELERATE,
            Self::Medium => BezierControl::STANDARD,
            Self::Long => BezierControl::STANDARD,
            Self::ExtraLong => BezierControl::STANDARD,
            Self::ModeSwitch => BezierControl::EMPHASISED,
        }
    }
}

/// All durations in ascending order — invariant asserted in tests.
pub const ALL_DURATIONS_ASC: &[u32] = &[
    DURATION_SHORT_MS,
    DURATION_MEDIUM_MS,
    DURATION_MODE_SWITCH_MS,
    DURATION_LONG_MS,
    DURATION_EXTRA_LONG_MS,
];

/// Reduce-motion override: when the user requests reduced motion, all
/// durations collapse toward the Short value.  The UI should call this
/// before configuring any Animation.
pub fn apply_reduced_motion(duration_ms: u32, reduced: bool) -> u32 {
    if reduced { duration_ms.min(DURATION_SHORT_MS) } else { duration_ms }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn durations_short_lt_medium_lt_long_lt_extra_long() {
        assert!(DURATION_SHORT_MS < DURATION_MEDIUM_MS);
        assert!(DURATION_MEDIUM_MS < DURATION_LONG_MS);
        assert!(DURATION_LONG_MS < DURATION_EXTRA_LONG_MS);
    }

    #[test]
    fn duration_mode_switch_between_medium_and_long() {
        assert!(DURATION_MODE_SWITCH_MS > DURATION_MEDIUM_MS);
        assert!(DURATION_MODE_SWITCH_MS < DURATION_LONG_MS);
    }

    #[test]
    fn bezier_standard_values() {
        let s = BezierControl::STANDARD;
        assert_eq!(s.x1, 0.4);
        assert_eq!(s.y1, 0.0);
        assert_eq!(s.x2, 0.2);
        assert_eq!(s.y2, 1.0);
    }

    #[test]
    fn bezier_emphasised_y2_is_1_51() {
        assert_eq!(BezierControl::EMPHASISED.y2, 1.51);
    }

    #[test]
    fn motion_level_short_duration() {
        assert_eq!(MotionLevel::Short.duration_ms(), DURATION_SHORT_MS);
    }

    #[test]
    fn motion_level_mode_switch_bezier_is_emphasised() {
        assert_eq!(MotionLevel::ModeSwitch.default_bezier(), BezierControl::EMPHASISED);
    }

    #[test]
    fn motion_level_medium_bezier_is_standard() {
        assert_eq!(MotionLevel::Medium.default_bezier(), BezierControl::STANDARD);
    }

    #[test]
    fn all_durations_asc_strictly_ascending() {
        for w in ALL_DURATIONS_ASC.windows(2) {
            assert!(w[0] < w[1], "{} should be < {}", w[0], w[1]);
        }
    }

    #[test]
    fn reduced_motion_clamps_400_to_150() {
        assert_eq!(apply_reduced_motion(400, true), 150);
    }

    #[test]
    fn reduced_motion_false_returns_unchanged() {
        assert_eq!(apply_reduced_motion(400, false), 400);
    }

    #[test]
    fn reduced_motion_already_short_unchanged() {
        assert_eq!(apply_reduced_motion(100, true), 100);
    }

    #[test]
    fn bezier_linear_values() {
        let l = BezierControl::LINEAR;
        assert_eq!(l.x1, 0.0);
        assert_eq!(l.y1, 0.0);
        assert_eq!(l.x2, 1.0);
        assert_eq!(l.y2, 1.0);
    }
}
