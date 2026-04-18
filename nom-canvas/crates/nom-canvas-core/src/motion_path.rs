//! Motion path primitives: easing functions, keyframes, path interpolation, and animation.

/// Easing curve kinds for keyframe transitions.
#[derive(Debug, Clone, PartialEq)]
pub enum EasingKind {
    /// Constant velocity — output equals input.
    Linear,
    /// Accelerates from rest.
    EaseIn,
    /// Decelerates to rest.
    EaseOut,
    /// Accelerates then decelerates (symmetric S-curve).
    EaseInOut,
}

impl EasingKind {
    /// Map a normalised time `t` in `[0, 1]` through this easing curve.
    pub fn apply(&self, t: f32) -> f32 {
        match self {
            EasingKind::Linear => t,
            EasingKind::EaseIn => t * t,
            EasingKind::EaseOut => t * (2.0 - t),
            EasingKind::EaseInOut => {
                if t < 0.5 {
                    2.0 * t * t
                } else {
                    -1.0 + (4.0 - 2.0 * t) * t
                }
            }
        }
    }

    /// Returns `true` for easing kinds that are symmetric around `t = 0.5`.
    pub fn is_symmetric(&self) -> bool {
        matches!(self, EasingKind::Linear | EasingKind::EaseInOut)
    }
}

/// A single keyframe on a motion path.
#[derive(Debug, Clone, PartialEq)]
pub struct MotionKeyframe {
    /// Normalised time position in `[0.0, 1.0]`.
    pub time: f32,
    /// Canvas X coordinate at this keyframe.
    pub x: f32,
    /// Canvas Y coordinate at this keyframe.
    pub y: f32,
    /// Easing applied when leaving this keyframe toward the next one.
    pub easing: EasingKind,
}

impl MotionKeyframe {
    /// Returns `true` when `time` lies in the valid range `[0.0, 1.0]`.
    pub fn is_valid(&self) -> bool {
        (0.0..=1.0).contains(&self.time)
    }

    /// Returns the `(x, y)` position recorded at this keyframe.
    pub fn position(&self) -> (f32, f32) {
        (self.x, self.y)
    }
}

/// An ordered sequence of keyframes describing a motion path.
#[derive(Debug, Clone, Default)]
pub struct MotionPath {
    /// Raw keyframes in insertion order; sort via [`MotionPath::sorted_keyframes`].
    pub keyframes: Vec<MotionKeyframe>,
}

impl MotionPath {
    /// Creates an empty `MotionPath`.
    pub fn new() -> Self {
        Self { keyframes: Vec::new() }
    }

    /// Appends a keyframe.
    pub fn add_keyframe(&mut self, kf: MotionKeyframe) {
        self.keyframes.push(kf);
    }

    /// Returns references to all keyframes sorted by `time` (ascending).
    pub fn sorted_keyframes(&self) -> Vec<&MotionKeyframe> {
        let mut refs: Vec<&MotionKeyframe> = self.keyframes.iter().collect();
        refs.sort_by(|a, b| a.time.partial_cmp(&b.time).unwrap_or(std::cmp::Ordering::Equal));
        refs
    }

    /// Span of normalised time covered by the path.
    ///
    /// Returns `last.time − first.time` when there are at least two keyframes, or `0.0` otherwise.
    pub fn duration(&self) -> f32 {
        let sorted = self.sorted_keyframes();
        if sorted.len() < 2 {
            return 0.0;
        }
        sorted.last().unwrap().time - sorted.first().unwrap().time
    }
}

/// Stateless interpolator that evaluates a [`MotionPath`] at an arbitrary `t`.
pub struct PathInterpolator;

impl PathInterpolator {
    /// Sample the path position at normalised time `t` in `[0, 1]`.
    ///
    /// * Empty path → `(0.0, 0.0)`
    /// * `t` ≤ first keyframe → first keyframe position
    /// * `t` ≥ last keyframe → last keyframe position
    /// * Otherwise → linear lerp between the surrounding pair, with the
    ///   **left** keyframe's easing applied to the local `t`.
    pub fn interpolate(path: &MotionPath, t: f32) -> (f32, f32) {
        let sorted = path.sorted_keyframes();
        if sorted.is_empty() {
            return (0.0, 0.0);
        }
        let first = sorted.first().unwrap();
        let last = sorted.last().unwrap();

        if t <= first.time {
            return first.position();
        }
        if t >= last.time {
            return last.position();
        }

        // Find the bracketing pair.
        let right_idx = sorted
            .iter()
            .position(|kf| kf.time > t)
            .unwrap_or(sorted.len() - 1);
        let left = sorted[right_idx - 1];
        let right = sorted[right_idx];

        let span = right.time - left.time;
        let local_t = if span > 0.0 { (t - left.time) / span } else { 0.0 };
        let eased_t = left.easing.apply(local_t);

        let x = left.x + (right.x - left.x) * eased_t;
        let y = left.y + (right.y - left.y) * eased_t;
        (x, y)
    }
}

/// Stateful animator that advances a playhead along a [`MotionPath`].
pub struct MotionAnimator {
    /// The path being animated.
    pub path: MotionPath,
    /// Current normalised time, clamped to `[0.0, 1.0]`.
    pub current_t: f32,
}

impl MotionAnimator {
    /// Creates a new animator starting at `t = 0`.
    pub fn new(path: MotionPath) -> Self {
        Self { path, current_t: 0.0 }
    }

    /// Advances `current_t` by `dt`, clamping the result to `[0.0, 1.0]`.
    pub fn advance(&mut self, dt: f32) {
        self.current_t = (self.current_t + dt).clamp(0.0, 1.0);
    }

    /// Returns the interpolated canvas position at the current playhead.
    pub fn current_position(&self) -> (f32, f32) {
        PathInterpolator::interpolate(&self.path, self.current_t)
    }

    /// Returns `true` when the playhead has reached the end of the path.
    pub fn is_done(&self) -> bool {
        self.current_t >= 1.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── EasingKind ──────────────────────────────────────────────────────────

    #[test]
    fn easing_linear_apply() {
        assert!((EasingKind::Linear.apply(0.5) - 0.5).abs() < 1e-6);
        assert!((EasingKind::Linear.apply(0.0) - 0.0).abs() < 1e-6);
        assert!((EasingKind::Linear.apply(1.0) - 1.0).abs() < 1e-6);
    }

    #[test]
    fn easing_ease_in_apply() {
        // EaseIn: t*t — at t=0.5 expect 0.25
        let result = EasingKind::EaseIn.apply(0.5);
        assert!((result - 0.25).abs() < 1e-6, "expected 0.25, got {result}");
        assert!((EasingKind::EaseIn.apply(1.0) - 1.0).abs() < 1e-6);
    }

    #[test]
    fn easing_is_symmetric() {
        assert!(EasingKind::Linear.is_symmetric());
        assert!(EasingKind::EaseInOut.is_symmetric());
        assert!(!EasingKind::EaseIn.is_symmetric());
        assert!(!EasingKind::EaseOut.is_symmetric());
    }

    // ── MotionKeyframe ───────────────────────────────────────────────────────

    #[test]
    fn keyframe_is_valid() {
        let valid = MotionKeyframe { time: 0.5, x: 10.0, y: 20.0, easing: EasingKind::Linear };
        assert!(valid.is_valid());

        let below = MotionKeyframe { time: -0.1, x: 0.0, y: 0.0, easing: EasingKind::Linear };
        assert!(!below.is_valid());

        let above = MotionKeyframe { time: 1.1, x: 0.0, y: 0.0, easing: EasingKind::Linear };
        assert!(!above.is_valid());
    }

    // ── MotionPath ───────────────────────────────────────────────────────────

    #[test]
    fn motion_path_duration_less_than_two_keyframes() {
        let mut path = MotionPath::new();
        assert_eq!(path.duration(), 0.0, "empty path duration must be 0");

        path.add_keyframe(MotionKeyframe { time: 0.3, x: 0.0, y: 0.0, easing: EasingKind::Linear });
        assert_eq!(path.duration(), 0.0, "single-keyframe duration must be 0");
    }

    #[test]
    fn motion_path_sorted_keyframes_order() {
        let mut path = MotionPath::new();
        path.add_keyframe(MotionKeyframe { time: 0.8, x: 80.0, y: 0.0, easing: EasingKind::Linear });
        path.add_keyframe(MotionKeyframe { time: 0.2, x: 20.0, y: 0.0, easing: EasingKind::Linear });
        path.add_keyframe(MotionKeyframe { time: 0.5, x: 50.0, y: 0.0, easing: EasingKind::Linear });

        let sorted = path.sorted_keyframes();
        assert_eq!(sorted.len(), 3);
        assert!((sorted[0].time - 0.2).abs() < 1e-6);
        assert!((sorted[1].time - 0.5).abs() < 1e-6);
        assert!((sorted[2].time - 0.8).abs() < 1e-6);
    }

    // ── PathInterpolator ─────────────────────────────────────────────────────

    #[test]
    fn interpolator_endpoints() {
        let mut path = MotionPath::new();
        path.add_keyframe(MotionKeyframe { time: 0.0, x: 0.0, y: 0.0, easing: EasingKind::Linear });
        path.add_keyframe(MotionKeyframe { time: 1.0, x: 100.0, y: 200.0, easing: EasingKind::Linear });

        let start = PathInterpolator::interpolate(&path, 0.0);
        assert!((start.0 - 0.0).abs() < 1e-4 && (start.1 - 0.0).abs() < 1e-4,
            "at t=0 expected (0,0), got {start:?}");

        let end = PathInterpolator::interpolate(&path, 1.0);
        assert!((end.0 - 100.0).abs() < 1e-4 && (end.1 - 200.0).abs() < 1e-4,
            "at t=1 expected (100,200), got {end:?}");
    }

    #[test]
    fn interpolator_empty_path() {
        let path = MotionPath::new();
        assert_eq!(PathInterpolator::interpolate(&path, 0.5), (0.0, 0.0));
    }

    // ── MotionAnimator ───────────────────────────────────────────────────────

    #[test]
    fn animator_advance_clamping() {
        let path = MotionPath::new();
        let mut anim = MotionAnimator::new(path);

        anim.advance(0.6);
        assert!((anim.current_t - 0.6).abs() < 1e-6);

        // Advancing beyond 1.0 must clamp.
        anim.advance(0.6);
        assert!((anim.current_t - 1.0).abs() < 1e-6, "current_t must clamp to 1.0");

        // Negative advance must not go below 0.0 after reset.
        let path2 = MotionPath::new();
        let mut anim2 = MotionAnimator::new(path2);
        anim2.advance(-0.5);
        assert!((anim2.current_t - 0.0).abs() < 1e-6, "current_t must clamp to 0.0");
    }

    #[test]
    fn animator_is_done() {
        let path = MotionPath::new();
        let mut anim = MotionAnimator::new(path);
        assert!(!anim.is_done(), "not done at t=0");

        anim.advance(1.0);
        assert!(anim.is_done(), "done after advancing to t=1");
    }
}
