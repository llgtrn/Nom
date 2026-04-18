use crate::types::*;
use std::time::Duration;

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

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MotionPreference {
    Full,
    Reduced,
}

impl Animation {
    pub fn new(duration: Duration, easing: impl Fn(f32) -> f32 + Send + Sync + 'static) -> Self {
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

    pub fn delta_with_motion(&self, elapsed: Duration, preference: MotionPreference) -> f32 {
        match preference {
            MotionPreference::Full => self.delta(elapsed),
            MotionPreference::Reduced => 1.0,
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
        1.0 - (-zeta * omega * t).exp()
            * ((omega_d * t).cos() + (zeta * omega / omega_d) * (omega_d * t).sin())
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
// AnimationGroup — sequential / parallel animation steps
// ---------------------------------------------------------------------------

/// A single step in an `AnimationGroup`.
pub struct AnimationStep {
    pub duration: f32,
    pub easing: Box<dyn Fn(f32) -> f32 + Send + Sync>,
}

/// Chains multiple animation steps; `sample` maps a global time to the
/// current step's local eased value.
pub struct AnimationGroup {
    steps: Vec<AnimationStep>,
}

impl AnimationGroup {
    pub fn new() -> Self {
        Self { steps: Vec::new() }
    }

    /// Append a step with the given duration and easing function.
    pub fn then(
        mut self,
        duration: f32,
        easing: impl Fn(f32) -> f32 + Send + Sync + 'static,
    ) -> Self {
        self.steps.push(AnimationStep {
            duration,
            easing: Box::new(easing),
        });
        self
    }

    /// Total duration of all steps combined.
    pub fn total_duration(&self) -> f32 {
        self.steps.iter().map(|s| s.duration).sum()
    }

    /// Map `global_t` ∈ [0, total_duration] to the eased value of the
    /// current step.  Returns the final step's value when `global_t` exceeds
    /// the total duration.
    pub fn sample(&self, global_t: f32) -> f32 {
        if self.steps.is_empty() {
            return 0.0;
        }
        let mut remaining = global_t.max(0.0);
        for step in &self.steps {
            if remaining <= step.duration {
                let local_t = if step.duration > 0.0 {
                    (remaining / step.duration).clamp(0.0, 1.0)
                } else {
                    1.0
                };
                return (step.easing)(local_t);
            }
            remaining -= step.duration;
        }
        // Past the end — return the final eased value at t=1.
        let last = self.steps.last().unwrap();
        (last.easing)(1.0)
    }
}

impl Default for AnimationGroup {
    fn default() -> Self {
        Self::new()
    }
}

// ---------------------------------------------------------------------------
// AnimationOrder — deterministic ordered animation values
// ---------------------------------------------------------------------------

/// Stores (id, value) pairs and returns them sorted by id ascending,
/// guaranteeing a deterministic application order regardless of insertion order.
pub struct AnimationOrder {
    anims: Vec<(u32, f32)>,
}

impl AnimationOrder {
    pub fn new() -> Self {
        Self { anims: Vec::new() }
    }

    /// Insert or replace the value for the given id.
    pub fn set(&mut self, id: u32, value: f32) {
        if let Some(entry) = self.anims.iter_mut().find(|(eid, _)| *eid == id) {
            entry.1 = value;
        } else {
            self.anims.push((id, value));
        }
    }

    /// Return all (id, value) pairs sorted by id ascending.
    pub fn get_sorted(&self) -> Vec<(u32, f32)> {
        let mut out = self.anims.clone();
        out.sort_by_key(|(id, _)| *id);
        out
    }
}

impl Default for AnimationOrder {
    fn default() -> Self {
        Self::new()
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

    // -----------------------------------------------------------------------
    // AnimationGroup tests
    // -----------------------------------------------------------------------

    #[test]
    fn animation_group_single_step() {
        let group = AnimationGroup::new().then(1.0, easing::linear());
        // At t=0.5 within a single 1-second linear step → 0.5.
        let result = group.sample(0.5);
        assert!((result - 0.5).abs() < 1e-6);
    }

    #[test]
    fn animation_group_two_steps_transition() {
        // Step 1: linear 0→1 over 1 s; step 2: ease_in 0→1 over 1 s.
        let group = AnimationGroup::new()
            .then(1.0, easing::linear())
            .then(1.0, easing::ease_in());
        // At global_t=0.5 we are in step 1, local_t=0.5 → linear(0.5)=0.5.
        assert!((group.sample(0.5) - 0.5).abs() < 1e-6);
        // At global_t=1.5 we are in step 2, local_t=0.5 → ease_in(0.5)=0.25.
        assert!((group.sample(1.5) - 0.25).abs() < 1e-6);
        // Past total duration → final step at t=1 → ease_in(1.0)=1.0.
        assert!((group.sample(3.0) - 1.0).abs() < 1e-6);
    }

    #[test]
    fn animation_group_total_duration() {
        let group = AnimationGroup::new()
            .then(0.3, easing::linear())
            .then(0.7, easing::ease_out());
        assert!((group.total_duration() - 1.0).abs() < 1e-6);
    }

    // -----------------------------------------------------------------------
    // AnimationOrder tests
    // -----------------------------------------------------------------------

    #[test]
    fn animation_order_sorts_by_id() {
        let mut order = AnimationOrder::new();
        order.set(3, 0.9);
        order.set(1, 0.1);
        order.set(2, 0.5);
        let sorted = order.get_sorted();
        assert_eq!(sorted, vec![(1, 0.1), (2, 0.5), (3, 0.9)]);
    }

    #[test]
    fn animation_order_set_overrides() {
        let mut order = AnimationOrder::new();
        order.set(1, 0.1);
        order.set(1, 0.8);
        let sorted = order.get_sorted();
        assert_eq!(sorted.len(), 1);
        assert!((sorted[0].1 - 0.8).abs() < 1e-6);
    }

    #[test]
    fn animation_linear_easing_midpoint() {
        // Linear easing at t=0.5 must return exactly 0.5.
        let anim = Animation::new(Duration::from_millis(500), easing::linear());
        let result = anim.evaluate(0.5);
        assert!(
            (result - 0.5).abs() < 1e-6,
            "linear(0.5) must be 0.5, got {result}"
        );
    }

    #[test]
    fn animation_step_completion_at_full_duration() {
        // AnimationGroup with a single linear step: at global_t == step duration
        // the sample value must be 1.0 (completed).
        let group = AnimationGroup::new().then(2.0, easing::linear());
        let result = group.sample(2.0);
        assert!(
            (result - 1.0).abs() < 1e-6,
            "step at full duration must return 1.0, got {result}"
        );
        // Past the end also returns 1.0.
        let past = group.sample(5.0);
        assert!(
            (past - 1.0).abs() < 1e-6,
            "step past full duration must return 1.0, got {past}"
        );
    }

    // -----------------------------------------------------------------------
    // Easing boundary tests
    // -----------------------------------------------------------------------

    #[test]
    fn easing_linear_at_zero() {
        let f = easing::linear();
        assert!((f(0.0) - 0.0).abs() < 1e-6, "linear(0.0) must be 0.0");
    }

    #[test]
    fn easing_linear_at_one() {
        let f = easing::linear();
        assert!((f(1.0) - 1.0).abs() < 1e-6, "linear(1.0) must be 1.0");
    }

    #[test]
    fn easing_ease_in_less_than_linear_at_midpoint() {
        let f = easing::ease_in();
        // t*t at 0.5 = 0.25 < 0.5
        assert!(f(0.5) < 0.5, "ease_in(0.5) must be < 0.5");
    }

    #[test]
    fn easing_ease_out_greater_than_linear_at_midpoint() {
        let f = easing::ease_out();
        // t*(2-t) at 0.5 = 0.75 > 0.5
        assert!(f(0.5) > 0.5, "ease_out(0.5) must be > 0.5");
    }

    #[test]
    fn easing_ease_in_at_boundaries() {
        let f = easing::ease_in();
        assert!((f(0.0) - 0.0).abs() < 1e-6);
        assert!((f(1.0) - 1.0).abs() < 1e-6);
    }

    #[test]
    fn easing_ease_out_at_boundaries() {
        let f = easing::ease_out();
        assert!((f(0.0) - 0.0).abs() < 1e-6);
        assert!((f(1.0) - 1.0).abs() < 1e-6);
    }

    #[test]
    fn spring_settle_converges_near_one() {
        // Advance spring 100 small steps (simulated via spring_value at t=1.0)
        // At t=1.0 the spring should be very close to its target of 1.0.
        let value = easing::spring_value(400.0, 28.0, 1.0);
        assert!(
            (value - 1.0).abs() < 0.05,
            "spring at t=1.0 should be within 0.05 of 1.0, got {value}"
        );
    }

    #[test]
    fn spring_starts_at_zero() {
        let value = easing::spring_value(400.0, 28.0, 0.0);
        assert!(
            (value - 0.0).abs() < 1e-6,
            "spring at t=0 must be 0.0, got {value}"
        );
    }

    #[test]
    fn animation_delta_increases_with_elapsed() {
        let anim = Animation::new(Duration::from_millis(1000), easing::linear());
        let d1 = anim.delta(Duration::from_millis(100));
        let d2 = anim.delta(Duration::from_millis(500));
        assert!(d2 > d1, "delta should increase with elapsed time");
    }

    #[test]
    fn animation_looping_delta_wraps() {
        let anim = Animation::new(Duration::from_millis(1000), easing::linear()).looping();
        // At 1.5× duration the fractional part should be ~0.5
        let d = anim.delta(Duration::from_millis(1500));
        assert!(
            (d - 0.5).abs() < 0.01,
            "looping delta at 1.5× should be ~0.5, got {d}"
        );
    }

    #[test]
    fn reduced_motion_delta_jumps_to_complete() {
        let anim = Animation::new(Duration::from_millis(300), easing::ease_out());
        assert_eq!(
            anim.delta_with_motion(Duration::from_millis(1), MotionPreference::Reduced),
            1.0
        );
    }

    #[test]
    fn full_motion_delta_preserves_elapsed_timing() {
        let anim = Animation::new(Duration::from_millis(300), easing::ease_out());
        let delta = anim.delta_with_motion(Duration::from_millis(150), MotionPreference::Full);
        assert!((delta - 0.5).abs() < 1e-6);
    }

    // -----------------------------------------------------------------------
    // Spring physics convergence
    // -----------------------------------------------------------------------

    #[test]
    fn spring_value_at_zero_is_zero() {
        assert!((easing::spring_value(200.0, 20.0, 0.0)).abs() < 1e-6);
    }

    #[test]
    fn spring_value_converges_near_one_at_large_t() {
        // At t=2.0 the spring should be very close to settled.
        let v = easing::spring_value(400.0, 28.0, 2.0);
        assert!((v - 1.0).abs() < 0.01, "spring at t=2.0: {v}");
    }

    #[test]
    fn spring_critically_damped_returns_one() {
        // zeta >= 1 → overdamped path, should return a value close to 1 at t=1.
        // omega = sqrt(stiffness); zeta = damping / (2*omega).
        // Choose damping so zeta >= 1: damping = 2*omega = 2*sqrt(100) = 20.
        let v = easing::spring_value(100.0, 20.0, 1.0);
        // Overdamped does not oscillate — value must be in [0, 1].
        assert!(v >= 0.0 && v <= 1.0, "overdamped spring value out of range: {v}");
    }

    #[test]
    fn spring_fn_closure_matches_spring_value() {
        let f = easing::spring(400.0, 28.0);
        let expected = easing::spring_value(400.0, 28.0, 0.5);
        assert!((f(0.5) - expected).abs() < 1e-6);
    }

    // -----------------------------------------------------------------------
    // Cubic-bezier timing (ease_in_out correctness)
    // -----------------------------------------------------------------------

    #[test]
    fn ease_in_out_symmetric_at_midpoint() {
        let f = easing::ease_in_out();
        // ease_in_out must equal 0.5 at t=0.5 (symmetric).
        assert!((f(0.5) - 0.5).abs() < 1e-6, "ease_in_out(0.5) must be 0.5");
    }

    #[test]
    fn ease_in_out_at_zero_and_one() {
        let f = easing::ease_in_out();
        assert!((f(0.0) - 0.0).abs() < 1e-6);
        assert!((f(1.0) - 1.0).abs() < 1e-6);
    }

    #[test]
    fn ease_out_quint_at_boundaries() {
        let f = easing::ease_out_quint();
        assert!((f(0.0) - 0.0).abs() < 1e-5, "ease_out_quint(0)={}", f(0.0));
        assert!((f(1.0) - 1.0).abs() < 1e-5, "ease_out_quint(1)={}", f(1.0));
    }

    #[test]
    fn ease_out_quint_greater_than_ease_out_at_midpoint() {
        // ease_out_quint accelerates more steeply than ease_out at early progress.
        let quint = easing::ease_out_quint()(0.5);
        let quad = easing::ease_out()(0.5);
        assert!(quint > quad, "ease_out_quint({quint}) should exceed ease_out({quad}) at t=0.5");
    }

    // -----------------------------------------------------------------------
    // Reduced-motion flag behaviour
    // -----------------------------------------------------------------------

    #[test]
    fn reduced_motion_preference_eq() {
        assert_eq!(MotionPreference::Reduced, MotionPreference::Reduced);
        assert_eq!(MotionPreference::Full, MotionPreference::Full);
        assert_ne!(MotionPreference::Full, MotionPreference::Reduced);
    }

    #[test]
    fn reduced_motion_always_returns_one_regardless_of_elapsed() {
        let anim = Animation::new(Duration::from_secs(5), easing::linear());
        // Even at t=0, reduced motion jumps to 1.0.
        assert_eq!(anim.delta_with_motion(Duration::ZERO, MotionPreference::Reduced), 1.0);
        assert_eq!(anim.delta_with_motion(Duration::from_millis(1), MotionPreference::Reduced), 1.0);
        assert_eq!(anim.delta_with_motion(Duration::from_secs(10), MotionPreference::Reduced), 1.0);
    }

    // -----------------------------------------------------------------------
    // AnimationGroup: more coverage
    // -----------------------------------------------------------------------

    #[test]
    fn animation_group_empty_sample_returns_zero() {
        let group = AnimationGroup::new();
        assert!((group.sample(0.5) - 0.0).abs() < 1e-6);
    }

    #[test]
    fn animation_group_sample_at_zero() {
        let group = AnimationGroup::new().then(1.0, easing::linear());
        assert!((group.sample(0.0) - 0.0).abs() < 1e-6);
    }

    #[test]
    fn animation_group_three_steps_total_duration() {
        let group = AnimationGroup::new()
            .then(0.1, easing::linear())
            .then(0.2, easing::ease_in())
            .then(0.7, easing::ease_out());
        assert!((group.total_duration() - 1.0).abs() < 1e-5);
    }

    #[test]
    fn animation_group_zero_duration_step_returns_final_value() {
        let group = AnimationGroup::new().then(0.0, easing::linear());
        // Zero-duration step: local_t=1.0 → linear(1.0)=1.0.
        assert!((group.sample(0.0) - 1.0).abs() < 1e-6);
    }

    // -----------------------------------------------------------------------
    // Interpolation helpers
    // -----------------------------------------------------------------------

    #[test]
    fn lerp_negative_to_positive() {
        let v = lerp(-1.0, 1.0, 0.5);
        assert!((v - 0.0).abs() < 1e-6);
    }

    #[test]
    fn lerp_t_greater_than_one_extrapolates() {
        // lerp does NOT clamp — t=2.0 extrapolates beyond `to`.
        let v = lerp(0.0, 10.0, 2.0);
        assert!((v - 20.0).abs() < 1e-5);
    }

    #[test]
    fn lerp_hsla_at_zero_returns_from() {
        let from = Hsla::new(30.0, 0.2, 0.4, 0.6);
        let to = Hsla::new(90.0, 0.8, 0.9, 1.0);
        let result = lerp_hsla(from, to, 0.0);
        assert!((result.h - from.h).abs() < 1e-5);
        assert!((result.s - from.s).abs() < 1e-5);
    }

    #[test]
    fn lerp_hsla_at_one_returns_to() {
        let from = Hsla::new(30.0, 0.2, 0.4, 0.6);
        let to = Hsla::new(90.0, 0.8, 0.9, 1.0);
        let result = lerp_hsla(from, to, 1.0);
        assert!((result.h - to.h).abs() < 1e-5);
        assert!((result.a - to.a).abs() < 1e-5);
    }
}
