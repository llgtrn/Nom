/// Bezier control point with a position and weight.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BezierPoint {
    /// Horizontal position.
    pub x: f32,
    /// Vertical position.
    pub y: f32,
    /// Influence weight (default 1.0).
    pub weight: f32,
}

impl BezierPoint {
    /// Create a new control point at `(x, y)` with weight 1.0.
    pub fn new(x: f32, y: f32) -> Self {
        Self { x, y, weight: 1.0 }
    }

    /// Override the weight, returning `self` for chaining.
    pub fn with_weight(mut self, w: f32) -> Self {
        self.weight = w;
        self
    }

    /// Linear interpolation between `self` and `other` at parameter `t`.
    ///
    /// The result weight is always 1.0.
    pub fn lerp(&self, other: &BezierPoint, t: f32) -> BezierPoint {
        BezierPoint {
            x: self.x * (1.0 - t) + other.x * t,
            y: self.y * (1.0 - t) + other.y * t,
            weight: 1.0,
        }
    }
}

/// Cubic Bezier curve defined by four control points.
#[derive(Debug, Clone, Copy)]
pub struct BezierCurve {
    /// Start point.
    pub p0: BezierPoint,
    /// First control point.
    pub p1: BezierPoint,
    /// Second control point.
    pub p2: BezierPoint,
    /// End point.
    pub p3: BezierPoint,
}

impl BezierCurve {
    /// Construct a cubic Bezier from four control points.
    pub fn new(p0: BezierPoint, p1: BezierPoint, p2: BezierPoint, p3: BezierPoint) -> Self {
        Self { p0, p1, p2, p3 }
    }

    /// Evaluate the curve at parameter `t` in `[0, 1]` using de Casteljau's algorithm.
    pub fn evaluate(&self, t: f32) -> BezierPoint {
        // Round 1 — 4 points → 3 points
        let q0 = self.p0.lerp(&self.p1, t);
        let q1 = self.p1.lerp(&self.p2, t);
        let q2 = self.p2.lerp(&self.p3, t);
        // Round 2 — 3 points → 2 points
        let r0 = q0.lerp(&q1, t);
        let r1 = q1.lerp(&q2, t);
        // Round 3 — 2 points → 1 point
        r0.lerp(&r1, t)
    }

    /// Estimate arc length by summing chord distances across `steps` intervals.
    ///
    /// At least one step is always used.
    pub fn length_estimate(&self, steps: usize) -> f32 {
        let n = steps.max(1);
        let mut total = 0.0_f32;
        let mut prev = self.evaluate(0.0);
        for i in 1..=n {
            let t = i as f32 / n as f32;
            let curr = self.evaluate(t);
            let dx = curr.x - prev.x;
            let dy = curr.y - prev.y;
            total += (dx * dx + dy * dy).sqrt();
            prev = curr;
        }
        total
    }

    /// Rough linearity check: true when the straight-line distance from p0 to p3
    /// is less than 1e-3 (i.e. start and end are effectively the same point).
    pub fn is_linear(&self) -> bool {
        let dx = (self.p0.x - self.p3.x).abs();
        let dy = (self.p0.y - self.p3.y).abs();
        dx + dy < 1e-3
    }
}

/// A Bezier curve with time-based animation state.
#[derive(Debug, Clone)]
pub struct AnimatedBezier {
    /// The underlying curve.
    pub curve: BezierCurve,
    /// Current animation progress in `[0, 1]`.
    pub current_t: f32,
    /// Units of `t` advanced per unit of time.
    pub speed: f32,
}

impl AnimatedBezier {
    /// Create a new animated bezier starting at `t = 0`.
    pub fn new(curve: BezierCurve, speed: f32) -> Self {
        Self { curve, current_t: 0.0, speed }
    }

    /// Advance animation by `dt` time units; clamps `current_t` to 1.0.
    pub fn advance(&mut self, dt: f32) {
        self.current_t = (self.current_t + self.speed * dt).min(1.0);
    }

    /// Return the point on the curve at the current animation time.
    pub fn current_position(&self) -> BezierPoint {
        self.curve.evaluate(self.current_t)
    }

    /// Returns `true` when the animation has reached the end of the curve.
    pub fn is_complete(&self) -> bool {
        self.current_t >= 1.0
    }
}

#[cfg(test)]
mod bezier_tests {
    use super::*;

    #[test]
    fn lerp_midpoint() {
        let a = BezierPoint::new(0.0, 0.0);
        let b = BezierPoint::new(4.0, 8.0);
        let mid = a.lerp(&b, 0.5);
        assert!((mid.x - 2.0).abs() < 1e-5, "mid.x should be 2.0, got {}", mid.x);
        assert!((mid.y - 4.0).abs() < 1e-5, "mid.y should be 4.0, got {}", mid.y);
        assert!((mid.weight - 1.0).abs() < 1e-5, "lerp weight should be 1.0");
    }

    #[test]
    fn with_weight() {
        let p = BezierPoint::new(1.0, 2.0).with_weight(0.5);
        assert!((p.weight - 0.5).abs() < 1e-5, "weight should be 0.5, got {}", p.weight);
        assert!((p.x - 1.0).abs() < 1e-5);
        assert!((p.y - 2.0).abs() < 1e-5);
    }

    #[test]
    fn evaluate_at_0_is_p0() {
        let curve = BezierCurve::new(
            BezierPoint::new(0.0, 0.0),
            BezierPoint::new(1.0, 2.0),
            BezierPoint::new(3.0, 4.0),
            BezierPoint::new(6.0, 0.0),
        );
        let pt = curve.evaluate(0.0);
        assert!((pt.x - 0.0).abs() < 1e-5, "evaluate(0) x should equal p0.x");
        assert!((pt.y - 0.0).abs() < 1e-5, "evaluate(0) y should equal p0.y");
    }

    #[test]
    fn evaluate_at_1_is_p3() {
        let curve = BezierCurve::new(
            BezierPoint::new(0.0, 0.0),
            BezierPoint::new(1.0, 2.0),
            BezierPoint::new(3.0, 4.0),
            BezierPoint::new(6.0, 1.0),
        );
        let pt = curve.evaluate(1.0);
        assert!((pt.x - 6.0).abs() < 1e-4, "evaluate(1) x should equal p3.x, got {}", pt.x);
        assert!((pt.y - 1.0).abs() < 1e-4, "evaluate(1) y should equal p3.y, got {}", pt.y);
    }

    #[test]
    fn evaluate_midpoint() {
        // Symmetric S-curve: midpoint should be at (3, 2) by symmetry.
        let curve = BezierCurve::new(
            BezierPoint::new(0.0, 0.0),
            BezierPoint::new(0.0, 4.0),
            BezierPoint::new(6.0, 0.0),
            BezierPoint::new(6.0, 4.0),
        );
        let mid = curve.evaluate(0.5);
        assert!((mid.x - 3.0).abs() < 1e-4, "midpoint x should be 3.0, got {}", mid.x);
        assert!((mid.y - 2.0).abs() < 1e-4, "midpoint y should be 2.0, got {}", mid.y);
    }

    #[test]
    fn length_estimate_positive() {
        let curve = BezierCurve::new(
            BezierPoint::new(0.0, 0.0),
            BezierPoint::new(1.0, 3.0),
            BezierPoint::new(4.0, 3.0),
            BezierPoint::new(5.0, 0.0),
        );
        let len = curve.length_estimate(20);
        assert!(len > 0.0, "length estimate must be positive, got {}", len);
        // A straight line from (0,0) to (5,0) has length 5; the curved path should be longer.
        assert!(len > 5.0, "curved length should exceed straight-line distance");
    }

    #[test]
    fn advance_increments_t() {
        let curve = BezierCurve::new(
            BezierPoint::new(0.0, 0.0),
            BezierPoint::new(1.0, 1.0),
            BezierPoint::new(2.0, 1.0),
            BezierPoint::new(3.0, 0.0),
        );
        let mut anim = AnimatedBezier::new(curve, 0.5);
        assert!((anim.current_t - 0.0).abs() < 1e-5);
        anim.advance(0.4); // t = 0.2
        assert!((anim.current_t - 0.2).abs() < 1e-5, "expected t=0.2 after advance(0.4) at speed 0.5, got {}", anim.current_t);
    }

    #[test]
    fn advance_clamps_at_1() {
        let curve = BezierCurve::new(
            BezierPoint::new(0.0, 0.0),
            BezierPoint::new(1.0, 1.0),
            BezierPoint::new(2.0, 1.0),
            BezierPoint::new(3.0, 0.0),
        );
        let mut anim = AnimatedBezier::new(curve, 2.0);
        anim.advance(1.0); // would be 2.0 without clamp
        assert!((anim.current_t - 1.0).abs() < 1e-5, "current_t should be clamped to 1.0, got {}", anim.current_t);
    }

    #[test]
    fn is_complete() {
        let curve = BezierCurve::new(
            BezierPoint::new(0.0, 0.0),
            BezierPoint::new(1.0, 1.0),
            BezierPoint::new(2.0, 1.0),
            BezierPoint::new(3.0, 0.0),
        );
        let mut anim = AnimatedBezier::new(curve, 1.0);
        assert!(!anim.is_complete(), "should not be complete at t=0");
        anim.advance(1.0);
        assert!(anim.is_complete(), "should be complete after advancing to t=1");
    }
}
