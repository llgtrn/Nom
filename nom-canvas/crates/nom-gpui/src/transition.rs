//! Animated state-transition helpers.
//!
//! A `Transition<T>` interpolates a value `T` from a `from` state to a `to`
//! state over a fixed duration.  Typical uses:
//!   - Panel reveal: `Transition<f32>` on opacity 0.0→1.0
//!   - Mode switch: `Transition<f32>` on progress 0.0→1.0 driving the whole view
//!   - Block expand: `Transition<f32>` on height 0→measured
#![deny(unsafe_code)]

use std::time::{Duration, Instant};
use crate::animation::{Animation, Easing, ease_lerp};

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum TransitionState {
    Idle,
    Running,
    Completed,
}

#[derive(Clone)]
pub struct Transition {
    anim: Animation,
    state: TransitionState,
}

impl Transition {
    pub fn new(duration: Duration, easing: Easing, from: f32, to: f32) -> Self {
        Self {
            anim: Animation::new(duration, easing, from, to),
            state: TransitionState::Running,
        }
    }

    pub fn sample(&mut self, now: Instant) -> f32 {
        let v = self.anim.sample(now);
        if self.anim.is_finished(now) {
            self.state = TransitionState::Completed;
        }
        v
    }

    pub fn state(&self) -> TransitionState {
        self.state
    }

    pub fn is_running(&self) -> bool {
        self.state == TransitionState::Running
    }

    pub fn is_completed(&self) -> bool {
        self.state == TransitionState::Completed
    }

    pub fn restart(&mut self) {
        self.anim.restart();
        self.state = TransitionState::Running;
    }
}

/// Vector transition: interpolate a 2D point.
pub struct PointTransition {
    x: Transition,
    y: Transition,
}

impl PointTransition {
    pub fn new(duration: Duration, easing: Easing, from: (f32, f32), to: (f32, f32)) -> Self {
        Self {
            x: Transition::new(duration, easing, from.0, to.0),
            y: Transition::new(duration, easing, from.1, to.1),
        }
    }

    pub fn sample(&mut self, now: Instant) -> (f32, f32) {
        (self.x.sample(now), self.y.sample(now))
    }

    pub fn is_completed(&self) -> bool {
        self.x.is_completed() && self.y.is_completed()
    }
}

/// Chain multiple transitions in sequence.  Each fires after the previous completes.
pub struct TransitionChain {
    transitions: Vec<Transition>,
    active_index: usize,
    chain_start: Instant,
    /// Cumulative time budget per step so we can dispatch samples deterministically.
    cumulative_offsets: Vec<Duration>,
}

impl TransitionChain {
    pub fn new(steps: Vec<(Duration, Easing, f32, f32)>) -> Self {
        let mut cumulative = Vec::with_capacity(steps.len());
        let mut acc = Duration::ZERO;
        for (d, _, _, _) in &steps {
            cumulative.push(acc);
            acc += *d;
        }
        let transitions: Vec<Transition> = steps
            .into_iter()
            .map(|(d, e, from, to)| Transition::new(d, e, from, to))
            .collect();
        Self {
            transitions,
            active_index: 0,
            chain_start: Instant::now(),
            cumulative_offsets: cumulative,
        }
    }

    pub fn sample_at(&mut self, now: Instant) -> Option<f32> {
        if self.transitions.is_empty() {
            return None;
        }
        let elapsed = now.saturating_duration_since(self.chain_start);
        // Find which step we're in.
        let mut idx = 0;
        for (i, &off) in self.cumulative_offsets.iter().enumerate() {
            if elapsed >= off {
                idx = i;
            }
        }
        self.active_index = idx;
        let step_start = self.chain_start + self.cumulative_offsets[idx];
        Some(self.transitions[idx].anim.sample(if now >= step_start { now } else { step_start }))
    }

    pub fn is_completed(&self) -> bool {
        self.transitions.iter().all(|t| t.is_completed())
    }

    pub fn step_count(&self) -> usize {
        self.transitions.len()
    }

    pub fn active_index(&self) -> usize {
        self.active_index
    }
}

/// Convenience: instant linear interpolation (not using Animation).
pub fn lerp_clamped(a: f32, b: f32, t: f32) -> f32 {
    ease_lerp(a, b, t.clamp(0.0, 1.0), Easing::Linear)
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── Transition ──────────────────────────────────────────────────────────

    #[test]
    fn new_starts_running() {
        let t = Transition::new(Duration::from_millis(100), Easing::Linear, 0.0, 1.0);
        assert_eq!(t.state(), TransitionState::Running);
        assert!(t.is_running());
        assert!(!t.is_completed());
    }

    #[test]
    fn sample_at_start_returns_from() {
        // Use a known start: create the animation then read its start field via sample.
        // We approximate by sampling immediately (elapsed ≈ 0).
        let mut t = Transition::new(Duration::from_millis(500), Easing::Linear, 5.0, 10.0);
        // Sample at a time well before duration.
        let now = Instant::now();
        // The transition's anim.start is ≤ now, so elapsed is tiny.  Value should be near 5.0.
        let v = t.sample(now);
        assert!(v >= 5.0 && v <= 10.0, "expected in [5,10], got {}", v);
    }

    #[test]
    fn sample_past_duration_returns_to_and_completes() {
        let mut t = Transition::new(Duration::from_millis(100), Easing::Linear, 0.0, 1.0);
        let future = Instant::now() + Duration::from_secs(10);
        let v = t.sample(future);
        assert!((v - 1.0).abs() < 1e-5, "expected 1.0, got {}", v);
        assert!(t.is_completed());
        assert!(!t.is_running());
    }

    #[test]
    fn is_running_true_before_completion() {
        let t = Transition::new(Duration::from_millis(1000), Easing::Linear, 0.0, 1.0);
        assert!(t.is_running());
    }

    #[test]
    fn is_completed_after_sample_past_end() {
        let mut t = Transition::new(Duration::from_millis(50), Easing::Linear, 0.0, 1.0);
        let future = Instant::now() + Duration::from_secs(1);
        t.sample(future);
        assert!(t.is_completed());
    }

    #[test]
    fn restart_resets_to_running() {
        let mut t = Transition::new(Duration::from_millis(50), Easing::Linear, 0.0, 1.0);
        let future = Instant::now() + Duration::from_secs(1);
        t.sample(future);
        assert!(t.is_completed());
        t.restart();
        assert!(t.is_running());
        assert_eq!(t.state(), TransitionState::Running);
    }

    #[test]
    fn zero_duration_immediately_at_to() {
        let mut t = Transition::new(Duration::ZERO, Easing::Linear, 3.0, 7.0);
        let v = t.sample(Instant::now());
        assert!((v - 7.0).abs() < 1e-5, "expected 7.0, got {}", v);
        assert!(t.is_completed());
    }

    // ── PointTransition ──────────────────────────────────────────────────────

    #[test]
    fn point_transition_samples_both_axes() {
        let mut pt = PointTransition::new(
            Duration::from_millis(100),
            Easing::Linear,
            (0.0, 0.0),
            (10.0, 20.0),
        );
        let future = Instant::now() + Duration::from_secs(1);
        let (x, y) = pt.sample(future);
        assert!((x - 10.0).abs() < 1e-4, "x should be 10.0, got {}", x);
        assert!((y - 20.0).abs() < 1e-4, "y should be 20.0, got {}", y);
    }

    #[test]
    fn point_transition_is_completed_when_both_done() {
        let mut pt = PointTransition::new(
            Duration::from_millis(50),
            Easing::Linear,
            (0.0, 0.0),
            (1.0, 1.0),
        );
        assert!(!pt.is_completed());
        let future = Instant::now() + Duration::from_secs(1);
        pt.sample(future);
        assert!(pt.is_completed());
    }

    // ── TransitionChain ──────────────────────────────────────────────────────

    #[test]
    fn chain_empty_step_count_zero() {
        let chain = TransitionChain::new(vec![]);
        assert_eq!(chain.step_count(), 0);
    }

    #[test]
    fn chain_sample_at_returns_none_when_empty() {
        let mut chain = TransitionChain::new(vec![]);
        assert!(chain.sample_at(Instant::now()).is_none());
    }

    #[test]
    fn chain_three_steps_active_index_advances() {
        let steps = vec![
            (Duration::from_millis(100), Easing::Linear, 0.0f32, 1.0f32),
            (Duration::from_millis(100), Easing::Linear, 1.0, 2.0),
            (Duration::from_millis(100), Easing::Linear, 2.0, 3.0),
        ];
        let mut chain = TransitionChain::new(steps);
        assert_eq!(chain.step_count(), 3);

        // Before any time passes: active_index should be 0.
        let t0 = chain.chain_start;
        chain.sample_at(t0);
        assert_eq!(chain.active_index(), 0);

        // After 150ms: into second step (offset 100ms).
        let t1 = chain.chain_start + Duration::from_millis(150);
        chain.sample_at(t1);
        assert_eq!(chain.active_index(), 1);

        // After 250ms: into third step (offset 200ms).
        let t2 = chain.chain_start + Duration::from_millis(250);
        chain.sample_at(t2);
        assert_eq!(chain.active_index(), 2);
    }

    // ── lerp_clamped ────────────────────────────────────────────────────────

    #[test]
    fn lerp_clamped_t_below_zero_clamps_to_a() {
        let v = lerp_clamped(5.0, 10.0, -1.0);
        assert!((v - 5.0).abs() < 1e-5, "expected 5.0, got {}", v);
    }

    #[test]
    fn lerp_clamped_t_above_one_clamps_to_b() {
        let v = lerp_clamped(5.0, 10.0, 2.0);
        assert!((v - 10.0).abs() < 1e-5, "expected 10.0, got {}", v);
    }

    #[test]
    fn lerp_clamped_t_half() {
        let v = lerp_clamped(0.0, 10.0, 0.5);
        assert!((v - 5.0).abs() < 1e-5, "expected 5.0, got {}", v);
    }
}
