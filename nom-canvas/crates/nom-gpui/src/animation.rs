use std::time::Duration;
use crate::types::*;

// ---------------------------------------------------------------------------
// Animation
// ---------------------------------------------------------------------------

/// Animation with closure-based easing.
/// Pattern: Zed (APP/zed-main/crates/gpui/src/elements/animation.rs)
/// Zed uses `Rc<dyn Fn(f32)->f32>` — we use `Box` for `Send + Sync` across threads.
pub struct Animation {
    pub duration: Duration,
    /// `true` = play once and stop; `false` = loop forever.
    pub oneshot: bool,
    pub easing: Box<dyn Fn(f32) -> f32 + Send + Sync>,
}

impl Animation {
    pub fn new(
        duration: Duration,
        easing: impl Fn(f32) -> f32 + Send + Sync + 'static,
    ) -> Self {
        Self {
            duration,
            oneshot: true,
            easing: Box::new(easing),
        }
    }

    /// Convert this animation to a looping animation.
    pub fn looping(mut self) -> Self {
        self.oneshot = false;
        self
    }

    /// Evaluate the easing function at `delta` ∈ [0, 1].
    pub fn evaluate(&self, delta: f32) -> f32 {
        (self.easing)(delta.clamp(0.0, 1.0))
    }

    /// Compute interpolation delta from elapsed time.
    ///
    /// - One-shot: clamped to [0, 1].
    /// - Looping: fractional part gives the repeating 0→1 ramp.
    pub fn delta(&self, elapsed: Duration) -> f32 {
        let t = elapsed.as_secs_f32() / self.duration.as_secs_f32();
        if self.oneshot {
            t.clamp(0.0, 1.0)
        } else {
            t.fract()
        }
    }

    /// Returns `true` when a one-shot animation has finished playing.
    pub fn is_complete(&self, elapsed: Duration) -> bool {
        self.oneshot && elapsed >= self.duration
    }
}

// ---------------------------------------------------------------------------
// Standard easing functions — closures, not an enum
// ---------------------------------------------------------------------------

pub mod easing {
    pub fn linear() -> impl Fn(f32) -> f32 + Send + Sync {
        |t| t
    }

    pub fn ease_in() -> impl Fn(f32) -> f32 + Send + Sync {
        |t| t * t
    }

    pub fn ease_out() -> impl Fn(f32) -> f32 + Send + Sync {
        |t| t * (2.0 - t)
    }

    pub fn ease_in_out() -> impl Fn(f32) -> f32 + Send + Sync {
        |t| {
            if t < 0.5 {
                2.0 * t * t
            } else {
                -1.0 + (4.0 - 2.0 * t) * t
            }
        }
    }

    pub fn ease_out_quint() -> impl Fn(f32) -> f32 + Send + Sync {
        |t| {
            let t1 = t - 1.0;
            1.0 + t1 * t1 * t1 * t1 * t1
        }
    }

    /// Spring animation — correct underdamped spring oscillator.
    /// Defaults: stiffness = 400, damping = 28 (AFFiNE motion token).
    /// y(t) = 1 - e^(-zeta*omega*t) * (cos(omega_d*t) + (zeta*omega/omega_d)*sin(omega_d*t))
    /// where omega_d = omega * sqrt(1 - zeta^2)
    pub fn spring(stiffness: f32, damping: f32) -> impl Fn(f32) -> f32 + Send + Sync {
        move |t| spring_value(stiffness, damping, t)
    }

    /// Evaluate the underdamped spring formula at time `t` ∈ [0, 1].
    pub fn spring_value(stiffness: f32, damping: f32, t: f32) -> f32 {
        let omega = stiffness.sqrt();
        let zeta = damping / (2.0 * stiffness.sqrt());
        if zeta >= 1.0 {
            return 1.0 - (-omega * t).exp() * (1.0 + omega * t);
        }
        let omega_d = omega * (1.0 - zeta * zeta).sqrt();
        1.0 - (-zeta * omega * t).exp() * (
            (omega_d * t).cos() + (zeta * omega / omega_d) * (omega_d * t).sin()
        )
    }

    /// NomCanvas connect animation: spring(400, 28).
    pub fn nom_connect() -> impl Fn(f32) -> f32 + Send + Sync {
        spring(400.0, 28.0)
    }

    /// NomCanvas hover animation: ease-out (120 ms).
    pub fn nom_hover() -> impl Fn(f32) -> f32 + Send + Sync {
        ease_out()
    }

    /// NomCanvas panel-resize animation: ease-in-out (200 ms).
    pub fn nom_panel_resize() -> impl Fn(f32) -> f32 + Send + Sync {
        ease_in_out()
    }
}

// ---------------------------------------------------------------------------
// Interpolation helpers
// ---------------------------------------------------------------------------

/// Linearly interpolate between two `f32` values.
pub fn lerp(from: f32, to: f32, t: f32) -> f32 {
    from + (to - from) * t
}

/// Linearly interpolate between two `Hsla` colors.
pub fn lerp_hsla(from: Hsla, to: Hsla, t: f32) -> Hsla {
    Hsla {
        h: lerp(from.h, to.h, t),
        s: lerp(from.s, to.s, t),
        l: lerp(from.l, to.l, t),
        a: lerp(from.a, to.a, t),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn linear_easing_midpoint() {
        let anim = Animation::new(Duration::from_millis(200), easing::linear());
        let result = anim.evaluate(0.5);
        assert!((result - 0.5).abs() < 1e-6);
    }

    #[test]
    fn ease_in_easing_midpoint() {
        let anim = Animation::new(Duration::from_millis(200), easing::ease_in());
        // t * t at t=0.5 → 0.25
        let result = anim.evaluate(0.5);
        assert!((result - 0.25).abs() < 1e-6);
    }

    #[test]
    fn delta_at_half_duration() {
        let dur = Duration::from_millis(400);
        let anim = Animation::new(dur, easing::linear());
        let half = Duration::from_millis(200);
        let delta = anim.delta(half);
        assert!((delta - 0.5).abs() < 1e-6);
    }

    #[test]
    fn is_complete_after_full_duration() {
        let dur = Duration::from_millis(300);
        let anim = Animation::new(dur, easing::linear());
        assert!(!anim.is_complete(Duration::from_millis(200)));
        assert!(anim.is_complete(Duration::from_millis(300)));
        assert!(anim.is_complete(Duration::from_millis(500)));
    }

    #[test]
    fn looping_anim_not_complete() {
        let dur = Duration::from_millis(300);
        let anim = Animation::new(dur, easing::linear()).looping();
        assert!(!anim.is_complete(Duration::from_millis(1000)));
    }

    #[test]
    fn lerp_midpoint() {
        assert!((lerp(0.0, 10.0, 0.5) - 5.0).abs() < 1e-6);
    }

    #[test]
    fn lerp_endpoints() {
        assert!((lerp(3.0, 7.0, 0.0) - 3.0).abs() < 1e-6);
        assert!((lerp(3.0, 7.0, 1.0) - 7.0).abs() < 1e-6);
    }

    #[test]
    fn lerp_hsla_midpoint() {
        let from = Hsla::new(0.0, 0.0, 0.0, 0.0);
        let to = Hsla::new(360.0, 1.0, 1.0, 1.0);
        let mid = lerp_hsla(from, to, 0.5);
        assert!((mid.h - 180.0).abs() < 1e-5);
        assert!((mid.s - 0.5).abs() < 1e-5);
        assert!((mid.l - 0.5).abs() < 1e-5);
        assert!((mid.a - 0.5).abs() < 1e-5);
    }

    #[test]
    fn evaluate_clamps_out_of_range() {
        let anim = Animation::new(Duration::from_millis(100), easing::linear());
        assert!((anim.evaluate(-0.5) - 0.0).abs() < 1e-6);
        assert!((anim.evaluate(1.5) - 1.0).abs() < 1e-6);
    }
}
