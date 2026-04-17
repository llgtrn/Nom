//! Geometric primitives: Point, Size, Bounds, pixel units.
//!
//! Three pixel units separate logical layout from physical output:
//! - `Pixels` — logical (before DPI scale).
//! - `ScaledPixels` — logical × dpi_scale (for layout math at target DPI).
//! - `DevicePixels` — physical pixels on the output surface.

use std::ops::{Add, AddAssign, Div, Mul, Neg, Sub, SubAssign};

/// Logical pixel unit (DPI-independent).
#[derive(Clone, Copy, Debug, Default, PartialEq, PartialOrd)]
#[repr(transparent)]
pub struct Pixels(pub f32);

impl Pixels {
    pub const ZERO: Self = Self(0.0);

    pub fn scale(self, factor: f32) -> ScaledPixels {
        ScaledPixels(self.0 * factor)
    }

    pub fn max(self, other: Self) -> Self {
        Self(self.0.max(other.0))
    }

    pub fn min(self, other: Self) -> Self {
        Self(self.0.min(other.0))
    }
}

/// DPI-scaled logical pixel (between Pixels and DevicePixels).
#[derive(Clone, Copy, Debug, Default, PartialEq, PartialOrd)]
#[repr(transparent)]
pub struct ScaledPixels(pub f32);

impl ScaledPixels {
    pub fn to_device(self) -> DevicePixels {
        DevicePixels(self.0.round() as i32)
    }
}

/// Physical output pixel (integer, aligned to device grid).
#[derive(Clone, Copy, Debug, Default, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[repr(transparent)]
pub struct DevicePixels(pub i32);

impl DevicePixels {
    pub const ZERO: Self = Self(0);
}

macro_rules! impl_pixel_arith {
    ($t:ty, $inner:ty) => {
        impl Add for $t {
            type Output = Self;
            fn add(self, rhs: Self) -> Self {
                Self(self.0 + rhs.0)
            }
        }
        impl Sub for $t {
            type Output = Self;
            fn sub(self, rhs: Self) -> Self {
                Self(self.0 - rhs.0)
            }
        }
        impl Mul<$inner> for $t {
            type Output = Self;
            fn mul(self, rhs: $inner) -> Self {
                Self(self.0 * rhs)
            }
        }
        impl Div<$inner> for $t {
            type Output = Self;
            fn div(self, rhs: $inner) -> Self {
                Self(self.0 / rhs)
            }
        }
        impl Neg for $t {
            type Output = Self;
            fn neg(self) -> Self {
                Self(-self.0)
            }
        }
        impl AddAssign for $t {
            fn add_assign(&mut self, rhs: Self) {
                self.0 += rhs.0;
            }
        }
        impl SubAssign for $t {
            fn sub_assign(&mut self, rhs: Self) {
                self.0 -= rhs.0;
            }
        }
    };
}

impl_pixel_arith!(Pixels, f32);
impl_pixel_arith!(ScaledPixels, f32);
impl_pixel_arith!(DevicePixels, i32);

/// 2D point, generic over pixel unit or any numeric type with arithmetic.
#[derive(Clone, Copy, Debug, Default, Eq, Hash, PartialEq)]
#[repr(C)]
pub struct Point<T> {
    pub x: T,
    pub y: T,
}

impl<T> Point<T> {
    pub const fn new(x: T, y: T) -> Self {
        Self { x, y }
    }
}

impl<T: Copy + Add<Output = T>> Add for Point<T> {
    type Output = Self;
    fn add(self, rhs: Self) -> Self {
        Self {
            x: self.x + rhs.x,
            y: self.y + rhs.y,
        }
    }
}

impl<T: Copy + Sub<Output = T>> Sub for Point<T> {
    type Output = Self;
    fn sub(self, rhs: Self) -> Self {
        Self {
            x: self.x - rhs.x,
            y: self.y - rhs.y,
        }
    }
}

/// 2D size, generic over pixel unit.
#[derive(Clone, Copy, Debug, Default, Eq, Hash, PartialEq)]
#[repr(C)]
pub struct Size<T> {
    pub width: T,
    pub height: T,
}

impl<T> Size<T> {
    pub const fn new(width: T, height: T) -> Self {
        Self { width, height }
    }
}

/// Axis-aligned bounding box (origin is top-left corner).
#[derive(Clone, Copy, Debug, Default, Eq, Hash, PartialEq)]
#[repr(C)]
pub struct Bounds<T> {
    pub origin: Point<T>,
    pub size: Size<T>,
}

impl<T: Copy + Add<Output = T> + PartialOrd> Bounds<T> {
    pub const fn new(origin: Point<T>, size: Size<T>) -> Self {
        Self { origin, size }
    }

    pub fn right(&self) -> T {
        self.origin.x + self.size.width
    }

    pub fn bottom(&self) -> T {
        self.origin.y + self.size.height
    }

    pub fn contains(&self, p: Point<T>) -> bool {
        p.x >= self.origin.x && p.x < self.right() && p.y >= self.origin.y && p.y < self.bottom()
    }
}

impl<T: Copy + Add<Output = T> + PartialOrd> Bounds<T> {
    pub fn intersects(&self, other: &Self) -> bool {
        self.origin.x < other.right()
            && other.origin.x < self.right()
            && self.origin.y < other.bottom()
            && other.origin.y < self.bottom()
    }
}

impl<T: Copy + Add<Output = T> + Sub<Output = T> + Ord> Bounds<T> {
    /// Enclosing bounds of `self ∪ other`. Requires integer-valued T (DevicePixels, i32)
    /// because enclosing f32 rectangles without Ord needs partial-min/max dances.
    pub fn union(&self, other: &Self) -> Self {
        let ox = self.origin.x.min(other.origin.x);
        let oy = self.origin.y.min(other.origin.y);
        let rx = self.right().max(other.right());
        let by = self.bottom().max(other.bottom());
        Self {
            origin: Point { x: ox, y: oy },
            size: Size {
                width: rx - ox,
                height: by - oy,
            },
        }
    }
}

/// Padding or spacing on the four edges of a rectangle.
#[derive(Clone, Copy, Debug, Default, Eq, Hash, PartialEq)]
#[repr(C)]
pub struct Edges<T> {
    pub top: T,
    pub right: T,
    pub bottom: T,
    pub left: T,
}

impl<T: Copy> Edges<T> {
    pub const fn new(top: T, right: T, bottom: T, left: T) -> Self {
        Self {
            top,
            right,
            bottom,
            left,
        }
    }

    pub const fn all(v: T) -> Self {
        Self {
            top: v,
            right: v,
            bottom: v,
            left: v,
        }
    }
}

/// Per-corner radii for rounded rectangles.
#[derive(Clone, Copy, Debug, Default, Eq, Hash, PartialEq)]
#[repr(C)]
pub struct Corners<T> {
    pub top_left: T,
    pub top_right: T,
    pub bottom_right: T,
    pub bottom_left: T,
}

impl<T: Copy> Corners<T> {
    pub const fn all(v: T) -> Self {
        Self {
            top_left: v,
            top_right: v,
            bottom_right: v,
            bottom_left: v,
        }
    }
}

/// 2D affine transform (rotate × scale + translate). Used for sprite transforms.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TransformationMatrix {
    pub rotation_scale: [[f32; 2]; 2],
    pub translation: [f32; 2],
}

impl TransformationMatrix {
    pub const IDENTITY: Self = Self {
        rotation_scale: [[1.0, 0.0], [0.0, 1.0]],
        translation: [0.0, 0.0],
    };

    pub fn translate(tx: f32, ty: f32) -> Self {
        Self {
            rotation_scale: [[1.0, 0.0], [0.0, 1.0]],
            translation: [tx, ty],
        }
    }

    pub fn scale(sx: f32, sy: f32) -> Self {
        Self {
            rotation_scale: [[sx, 0.0], [0.0, sy]],
            translation: [0.0, 0.0],
        }
    }

    pub fn rotate(radians: f32) -> Self {
        let (s, c) = radians.sin_cos();
        Self {
            rotation_scale: [[c, -s], [s, c]],
            translation: [0.0, 0.0],
        }
    }

    /// Compose two transforms: `self` applied after `rhs`.
    pub fn compose(&self, rhs: &Self) -> Self {
        let a = self.rotation_scale;
        let b = rhs.rotation_scale;
        let rs = [
            [
                a[0][0] * b[0][0] + a[0][1] * b[1][0],
                a[0][0] * b[0][1] + a[0][1] * b[1][1],
            ],
            [
                a[1][0] * b[0][0] + a[1][1] * b[1][0],
                a[1][0] * b[0][1] + a[1][1] * b[1][1],
            ],
        ];
        let t = [
            a[0][0] * rhs.translation[0] + a[0][1] * rhs.translation[1] + self.translation[0],
            a[1][0] * rhs.translation[0] + a[1][1] * rhs.translation[1] + self.translation[1],
        ];
        Self {
            rotation_scale: rs,
            translation: t,
        }
    }

    pub fn apply(&self, p: [f32; 2]) -> [f32; 2] {
        let rs = self.rotation_scale;
        [
            rs[0][0] * p[0] + rs[0][1] * p[1] + self.translation[0],
            rs[1][0] * p[0] + rs[1][1] * p[1] + self.translation[1],
        ]
    }
}

impl Default for TransformationMatrix {
    fn default() -> Self {
        Self::IDENTITY
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pixel_scaling_roundtrip() {
        let p = Pixels(10.0);
        assert_eq!(p.scale(2.0).to_device(), DevicePixels(20));
    }

    /// Compile-time assertion that `From<f32>` is NOT implemented for `Pixels`,
    /// `ScaledPixels`, or `DevicePixels`. If someone adds `impl From<f32> for Pixels`
    /// in the future, this test will fail to compile — catching the regression early.
    ///
    /// `assert_not_impl_all!(T: Trait)` passes if `T` does NOT implement `Trait`.
    #[test]
    fn pixels_does_not_implement_from_f32() {
        use static_assertions::assert_not_impl_all;
        assert_not_impl_all!(Pixels: From<f32>);
        assert_not_impl_all!(ScaledPixels: From<f32>);
        // DevicePixels is integer-based; no From<i32> either.
        assert_not_impl_all!(DevicePixels: From<i32>);
    }

    #[test]
    fn pixels_requires_explicit_construction() {
        // Verify that Pixels must be constructed explicitly — .0 field is the only
        // extraction path after removing the implicit From<f32> / From<Pixels> impls.
        let p = Pixels(42.0);
        assert_eq!(p.0, 42.0_f32);
        // ScaledPixels and DevicePixels likewise have no From<f32>/From<i32>:
        let sp = ScaledPixels(5.0);
        assert_eq!(sp.0, 5.0_f32);
        let dp = DevicePixels(3);
        assert_eq!(dp.0, 3_i32);
    }

    #[test]
    fn point_arithmetic() {
        let a = Point::new(1i32, 2);
        let b = Point::new(3i32, 4);
        assert_eq!(a + b, Point::new(4, 6));
        assert_eq!(b - a, Point::new(2, 2));
    }

    #[test]
    fn bounds_contains_and_intersects() {
        let a = Bounds::new(Point::new(0i32, 0), Size::new(10, 10));
        let b = Bounds::new(Point::new(5i32, 5), Size::new(10, 10));
        assert!(a.contains(Point::new(3, 3)));
        assert!(!a.contains(Point::new(11, 3)));
        assert!(a.intersects(&b));
    }

    #[test]
    fn bounds_union_encloses_both() {
        let a = Bounds::new(Point::new(0i32, 0), Size::new(5, 5));
        let b = Bounds::new(Point::new(10i32, 10), Size::new(5, 5));
        let u = a.union(&b);
        assert_eq!(u.origin, Point::new(0, 0));
        assert_eq!(u.size, Size::new(15, 15));
    }

    #[test]
    fn transform_identity_is_noop() {
        let t = TransformationMatrix::IDENTITY;
        assert_eq!(t.apply([1.0, 2.0]), [1.0, 2.0]);
    }

    #[test]
    fn transform_compose_translate_then_scale() {
        let scale = TransformationMatrix::scale(2.0, 2.0);
        let translate = TransformationMatrix::translate(3.0, 4.0);
        // scale ∘ translate: translate first, then scale
        let composed = scale.compose(&translate);
        assert_eq!(composed.apply([1.0, 1.0]), [8.0, 10.0]);
    }
}
