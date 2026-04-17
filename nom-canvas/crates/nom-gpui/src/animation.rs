//! Timestamp-based animation with easing.
#![deny(unsafe_code)]

use std::time::{Duration, Instant};

/// Easing curve for [0.0..=1.0] progress values.
#[derive(Clone, Copy, Debug)]
pub enum Easing {
    Linear,
    EaseInQuad,
    EaseOutQuad,
    EaseInOutQuad,
    CubicBezier { x1: f32, y1: f32, x2: f32, y2: f32 },
}

impl Easing {
    /// Mode-switch easing preset (blueprint section 5).
    pub const MODE_SWITCH: Easing = Easing::CubicBezier {
        x1: 0.27,
        y1: 0.2,
        x2: 0.25,
        y2: 1.51,
    };

    pub fn apply(self, t: f32) -> f32 {
        let t = t.clamp(0.0, 1.0);
        match self {
            Easing::Linear => t,
            Easing::EaseInQuad => t * t,
            Easing::EaseOutQuad => 1.0 - (1.0 - t) * (1.0 - t),
            Easing::EaseInOutQuad => {
                if t < 0.5 {
                    2.0 * t * t
                } else {
                    1.0 - 2.0 * (1.0 - t) * (1.0 - t)
                }
            }
            Easing::CubicBezier { x1, y1, x2, y2 } => cubic_bezier_eval(x1, y1, x2, y2, t),
        }
    }
}

fn cubic_bezier_eval(x1: f32, y1: f32, x2: f32, y2: f32, t: f32) -> f32 {
    fn sample_x(x1: f32, x2: f32, u: f32) -> f32 {
        let inv = 1.0 - u;
        3.0 * inv * inv * u * x1 + 3.0 * inv * u * u * x2 + u * u * u
    }

    fn sample_y(y1: f32, y2: f32, u: f32) -> f32 {
        let inv = 1.0 - u;
        3.0 * inv * inv * u * y1 + 3.0 * inv * u * u * y2 + u * u * u
    }

    fn dx_du(x1: f32, x2: f32, u: f32) -> f32 {
        let inv = 1.0 - u;
        3.0 * inv * inv * x1 + 6.0 * inv * u * (x2 - x1) + 3.0 * u * u * (1.0 - x2)
    }

    let mut u = t;
    for _ in 0..8 {
        let x_err = sample_x(x1, x2, u) - t;
        let deriv = dx_du(x1, x2, u);
        if deriv.abs() < 1e-6 {
            break;
        }
        u -= x_err / deriv;
        u = u.clamp(0.0, 1.0);
    }

    let x_err = (sample_x(x1, x2, u) - t).abs();
    if x_err > 1e-5 {
        let mut lo = 0.0_f32;
        let mut hi = 1.0_f32;
        for _ in 0..20 {
            let mid = (lo + hi) * 0.5;
            let x_mid = sample_x(x1, x2, mid);
            if x_mid < t {
                lo = mid;
            } else {
                hi = mid;
            }
        }
        u = (lo + hi) * 0.5;
    }

    sample_y(y1, y2, u)
}

#[derive(Clone)]
pub struct Animation {
    start: Instant,
    duration: Duration,
    easing: Easing,
    from: f32,
    to: f32,
}

impl Animation {
    pub fn new(duration: Duration, easing: Easing, from: f32, to: f32) -> Self {
        Self { start: Instant::now(), duration, easing, from, to }
    }

    pub fn now(&self) -> f32 {
        self.sample(Instant::now())
    }

    pub fn sample(&self, now: Instant) -> f32 {
        let elapsed = now.saturating_duration_since(self.start);
        let t = (elapsed.as_secs_f32() / self.duration.as_secs_f32()).clamp(0.0, 1.0);
        let eased = self.easing.apply(t);
        self.from + (self.to - self.from) * eased
    }

    pub fn progress(&self, now: Instant) -> f32 {
        let elapsed = now.saturating_duration_since(self.start);
        (elapsed.as_secs_f32() / self.duration.as_secs_f32()).clamp(0.0, 1.0)
    }

    pub fn is_finished(&self, now: Instant) -> bool {
        now.saturating_duration_since(self.start) >= self.duration
    }

    pub fn restart(&mut self) {
        self.start = Instant::now();
    }
}

pub fn lerp(from: f32, to: f32, t: f32) -> f32 {
    from + (to - from) * t
}

pub fn ease_lerp(from: f32, to: f32, t: f32, easing: Easing) -> f32 {
    lerp(from, to, easing.apply(t))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn linear_apply_zero() { assert_eq!(Easing::Linear.apply(0.0), 0.0); }

    #[test]
    fn linear_apply_half() { assert_eq!(Easing::Linear.apply(0.5), 0.5); }

    #[test]
    fn linear_apply_one() { assert_eq!(Easing::Linear.apply(1.0), 1.0); }

    #[test]
    fn ease_in_quad_half() {
        assert!((Easing::EaseInQuad.apply(0.5) - 0.25).abs() < 1e-6);
    }

    #[test]
    fn ease_out_quad_half() {
        assert!((Easing::EaseOutQuad.apply(0.5) - 0.75).abs() < 1e-6);
    }

    #[test]
    fn cubic_bezier_zero() {
        let e = Easing::CubicBezier { x1: 0.25, y1: 0.1, x2: 0.25, y2: 1.0 };
        assert!(e.apply(0.0).abs() < 1e-5);
    }

    #[test]
    fn cubic_bezier_one() {
        let e = Easing::CubicBezier { x1: 0.25, y1: 0.1, x2: 0.25, y2: 1.0 };
        assert!((e.apply(1.0) - 1.0).abs() < 1e-4);
    }

    #[test]
    fn cubic_bezier_half_in_range() {
        let e = Easing::CubicBezier { x1: 0.25, y1: 0.1, x2: 0.25, y2: 1.0 };
        let v = e.apply(0.5);
        assert!(v > 0.3 && v < 0.85, "expected 0.3 < {} < 0.85", v);
    }

    #[test]
    fn mode_switch_zero() {
        assert!(Easing::MODE_SWITCH.apply(0.0).abs() < 1e-5);
    }

    #[test]
    fn mode_switch_one_near_one() {
        assert!((Easing::MODE_SWITCH.apply(1.0) - 1.0).abs() < 1e-4);
    }

    #[test]
    fn animation_sample_at_start_equals_from() {
        let anim = Animation::new(Duration::from_millis(100), Easing::Linear, 0.0, 100.0);
        let v = anim.sample(anim.start);
        assert!(v.abs() < 1e-4, "expected ~0, got {}", v);
    }

    #[test]
    fn animation_sample_at_end_equals_to() {
        let anim = Animation::new(Duration::from_millis(100), Easing::Linear, 0.0, 100.0);
        let end = anim.start + Duration::from_millis(100);
        let v = anim.sample(end);
        assert!((v - 100.0).abs() < 1e-4, "expected ~100, got {}", v);
    }

    #[test]
    fn animation_sample_past_end_clamps_to_to() {
        let anim = Animation::new(Duration::from_millis(100), Easing::Linear, 0.0, 100.0);
        let past = anim.start + Duration::from_millis(500);
        let v = anim.sample(past);
        assert!((v - 100.0).abs() < 1e-4, "expected 100 clamped, got {}", v);
    }

    #[test]
    fn animation_is_finished_false_at_start() {
        let anim = Animation::new(Duration::from_millis(100), Easing::Linear, 0.0, 1.0);
        assert!(!anim.is_finished(anim.start));
    }

    #[test]
    fn animation_is_finished_true_after_duration() {
        let anim = Animation::new(Duration::from_millis(100), Easing::Linear, 0.0, 1.0);
        let done = anim.start + Duration::from_millis(100);
        assert!(anim.is_finished(done));
    }

    #[test]
    fn lerp_basic() {
        assert!((lerp(0.0, 10.0, 0.3) - 3.0).abs() < 1e-6);
    }

    #[test]
    fn ease_lerp_linear() {
        assert!((ease_lerp(0.0, 10.0, 0.5, Easing::Linear) - 5.0).abs() < 1e-6);
    }
}
