//! 2-D transform stack: composable scale/translate transforms, inverse computation,
//! clamped transform results, and a composer facade.

/// A 2-D affine transform limited to uniform scale, translation, and rotation.
///
/// `apply_to_point` applies scale then translate only (rotation stub kept for future use).
#[derive(Debug, Clone, PartialEq)]
pub struct Transform2D {
    /// X translation.
    pub tx: f32,
    /// Y translation.
    pub ty: f32,
    /// X scale factor.
    pub sx: f32,
    /// Y scale factor.
    pub sy: f32,
    /// Rotation in radians (carried but not applied by `apply_to_point`).
    pub rotation_rad: f32,
}

impl Transform2D {
    /// Returns the identity transform (no scale, no translation, no rotation).
    pub fn identity() -> Transform2D {
        Transform2D {
            tx: 0.0,
            ty: 0.0,
            sx: 1.0,
            sy: 1.0,
            rotation_rad: 0.0,
        }
    }

    /// Applies scale then translate: `(x*sx + tx, y*sy + ty)`.
    /// Rotation is intentionally ignored (stub).
    pub fn apply_to_point(&self, x: f32, y: f32) -> (f32, f32) {
        (x * self.sx + self.tx, y * self.sy + self.ty)
    }

    /// Returns `true` when this transform is the identity.
    pub fn is_identity(&self) -> bool {
        self.tx == 0.0
            && self.ty == 0.0
            && self.sx == 1.0
            && self.sy == 1.0
            && self.rotation_rad == 0.0
    }
}

/// A stack of `Transform2D` values representing the current transform hierarchy.
#[derive(Debug, Clone, Default)]
pub struct TransformStack {
    stack: Vec<Transform2D>,
}

impl TransformStack {
    /// Creates an empty stack.
    pub fn new() -> Self {
        TransformStack { stack: Vec::new() }
    }

    /// Pushes a transform onto the stack.
    pub fn push(&mut self, t: Transform2D) {
        self.stack.push(t);
    }

    /// Pops the top transform from the stack, returning it if present.
    pub fn pop(&mut self) -> Option<Transform2D> {
        self.stack.pop()
    }

    /// Returns the top transform, or the identity if the stack is empty.
    pub fn current(&self) -> Transform2D {
        self.stack.last().cloned().unwrap_or_else(Transform2D::identity)
    }

    /// Returns the number of transforms currently on the stack.
    pub fn depth(&self) -> usize {
        self.stack.len()
    }
}

/// The inverse of a `Transform2D`: negates translation (scaled) and inverts scale/rotation.
#[derive(Debug, Clone)]
pub struct InverseTransform {
    /// The stored inverse as a `Transform2D` for reuse of `apply_to_point`.
    pub transform: Transform2D,
}

impl InverseTransform {
    /// Computes the inverse of `t`:
    /// `tx = -t.tx / t.sx`, `ty = -t.ty / t.sy`,
    /// `sx = 1/t.sx`, `sy = 1/t.sy`, `rotation_rad = -t.rotation_rad`.
    pub fn from_transform(t: &Transform2D) -> InverseTransform {
        InverseTransform {
            transform: Transform2D {
                tx: -t.tx / t.sx,
                ty: -t.ty / t.sy,
                sx: 1.0 / t.sx,
                sy: 1.0 / t.sy,
                rotation_rad: -t.rotation_rad,
            },
        }
    }

    /// Applies the inverse transform to a point by delegating to `transform.apply_to_point`.
    pub fn apply_to_point(&self, x: f32, y: f32) -> (f32, f32) {
        self.transform.apply_to_point(x, y)
    }
}

/// The result of transforming a point, including whether it was clamped to bounds.
#[derive(Debug, Clone, PartialEq)]
pub struct TransformResult {
    /// Transformed (and possibly clamped) X coordinate.
    pub x: f32,
    /// Transformed (and possibly clamped) Y coordinate.
    pub y: f32,
    /// `true` if any clamping occurred.
    pub clamped: bool,
}

impl TransformResult {
    /// Constructs a result from a point, optionally clamping to `(min_x, min_y, max_x, max_y)`.
    pub fn from_point(x: f32, y: f32, clamp_bounds: Option<(f32, f32, f32, f32)>) -> TransformResult {
        match clamp_bounds {
            None => TransformResult { x, y, clamped: false },
            Some((min_x, min_y, max_x, max_y)) => {
                let cx = x.clamp(min_x, max_x);
                let cy = y.clamp(min_y, max_y);
                TransformResult {
                    x: cx,
                    y: cy,
                    clamped: cx != x || cy != y,
                }
            }
        }
    }
}

/// Facade combining a `TransformStack` with convenience methods for canvas transform management.
#[derive(Debug, Clone, Default)]
pub struct TransformComposer {
    stack: TransformStack,
}

impl TransformComposer {
    /// Creates a new composer with an empty stack.
    pub fn new() -> Self {
        TransformComposer { stack: TransformStack::new() }
    }

    /// Pushes a transform onto the internal stack.
    pub fn push_transform(&mut self, t: Transform2D) {
        self.stack.push(t);
    }

    /// Pops the top transform from the internal stack.
    pub fn pop_transform(&mut self) {
        self.stack.pop();
    }

    /// Applies the current top-of-stack transform to the given point.
    pub fn transform_point(&self, x: f32, y: f32) -> (f32, f32) {
        self.stack.current().apply_to_point(x, y)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn transform_apply_to_point_scale_and_translate() {
        let t = Transform2D { tx: 10.0, ty: 20.0, sx: 2.0, sy: 3.0, rotation_rad: 0.0 };
        let (rx, ry) = t.apply_to_point(5.0, 4.0);
        assert_eq!(rx, 5.0 * 2.0 + 10.0); // 20.0
        assert_eq!(ry, 4.0 * 3.0 + 20.0); // 32.0
    }

    #[test]
    fn transform_is_identity_true_for_identity() {
        let t = Transform2D::identity();
        assert!(t.is_identity());
    }

    #[test]
    fn transform_identity_constructor_values() {
        let t = Transform2D::identity();
        assert_eq!(t.tx, 0.0);
        assert_eq!(t.ty, 0.0);
        assert_eq!(t.sx, 1.0);
        assert_eq!(t.sy, 1.0);
        assert_eq!(t.rotation_rad, 0.0);
    }

    #[test]
    fn stack_push_and_current_returns_top() {
        let mut s = TransformStack::new();
        let t1 = Transform2D { tx: 1.0, ty: 2.0, sx: 1.0, sy: 1.0, rotation_rad: 0.0 };
        let t2 = Transform2D { tx: 5.0, ty: 6.0, sx: 2.0, sy: 2.0, rotation_rad: 0.0 };
        s.push(t1);
        s.push(t2.clone());
        assert_eq!(s.current(), t2);
        assert_eq!(s.depth(), 2);
    }

    #[test]
    fn stack_pop_returns_pushed_transform() {
        let mut s = TransformStack::new();
        let t = Transform2D { tx: 3.0, ty: 4.0, sx: 1.5, sy: 1.5, rotation_rad: 0.0 };
        s.push(t.clone());
        let popped = s.pop();
        assert_eq!(popped, Some(t));
        assert_eq!(s.depth(), 0);
        // Empty stack returns identity
        assert!(s.current().is_identity());
    }

    #[test]
    fn inverse_from_transform_sx_half_gives_sx_inv_two() {
        let t = Transform2D { tx: 0.0, ty: 0.0, sx: 0.5, sy: 0.5, rotation_rad: 0.0 };
        let inv = InverseTransform::from_transform(&t);
        assert!((inv.transform.sx - 2.0).abs() < 1e-6);
        assert!((inv.transform.sy - 2.0).abs() < 1e-6);
    }

    #[test]
    fn inverse_apply_to_point_round_trips() {
        let t = Transform2D { tx: 4.0, ty: 8.0, sx: 2.0, sy: 4.0, rotation_rad: 0.0 };
        let inv = InverseTransform::from_transform(&t);
        // Forward: (1,1) → (1*2+4, 1*4+8) = (6, 12)
        let (fx, fy) = t.apply_to_point(1.0, 1.0);
        // Inverse of (6,12): x*0.5 + (-2) = 1.0, y*0.25 + (-2) = 1.0
        let (ox, oy) = inv.apply_to_point(fx, fy);
        assert!((ox - 1.0).abs() < 1e-5, "expected ~1.0 got {ox}");
        assert!((oy - 1.0).abs() < 1e-5, "expected ~1.0 got {oy}");
    }

    #[test]
    fn result_from_point_no_clamp() {
        let r = TransformResult::from_point(3.0, 7.0, None);
        assert_eq!(r.x, 3.0);
        assert_eq!(r.y, 7.0);
        assert!(!r.clamped);
    }

    #[test]
    fn result_from_point_clamped() {
        // x=200 exceeds max_x=100 → clamped
        let r = TransformResult::from_point(200.0, 50.0, Some((0.0, 0.0, 100.0, 100.0)));
        assert_eq!(r.x, 100.0);
        assert_eq!(r.y, 50.0);
        assert!(r.clamped);
    }
}
