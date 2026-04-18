use std::ops::{Add, Div, Mul, Neg, Sub};

// ---------------------------------------------------------------------------
// Pixels — primary unit for GPU rendering coordinates
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, PartialOrd, Default)]
pub struct Pixels(pub f32);

impl Add for Pixels {
    type Output = Pixels;
    fn add(self, rhs: Pixels) -> Pixels {
        Pixels(self.0 + rhs.0)
    }
}

impl Sub for Pixels {
    type Output = Pixels;
    fn sub(self, rhs: Pixels) -> Pixels {
        Pixels(self.0 - rhs.0)
    }
}

impl Mul<f32> for Pixels {
    type Output = Pixels;
    fn mul(self, rhs: f32) -> Pixels {
        Pixels(self.0 * rhs)
    }
}

impl Div<f32> for Pixels {
    type Output = Pixels;
    fn div(self, rhs: f32) -> Pixels {
        Pixels(self.0 / rhs)
    }
}

impl Neg for Pixels {
    type Output = Pixels;
    fn neg(self) -> Pixels {
        Pixels(-self.0)
    }
}

impl From<f32> for Pixels {
    fn from(v: f32) -> Pixels {
        Pixels(v)
    }
}

impl From<u32> for Pixels {
    fn from(v: u32) -> Pixels {
        Pixels(v as f32)
    }
}

impl Pixels {
    pub fn new(v: f32) -> Self {
        Pixels(v)
    }
    pub fn zero() -> Self {
        Pixels(0.0)
    }
    pub fn floor(self) -> Self {
        Pixels(self.0.floor())
    }
    pub fn ceil(self) -> Self {
        Pixels(self.0.ceil())
    }
    pub fn abs(self) -> Self {
        Pixels(self.0.abs())
    }
}

// ---------------------------------------------------------------------------
// Point<T> — 2D coordinate
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Point<T> {
    pub x: T,
    pub y: T,
}

impl<T: Add<Output = T>> Add for Point<T> {
    type Output = Point<T>;
    fn add(self, rhs: Point<T>) -> Point<T> {
        Point {
            x: self.x + rhs.x,
            y: self.y + rhs.y,
        }
    }
}

impl<T: Sub<Output = T>> Sub for Point<T> {
    type Output = Point<T>;
    fn sub(self, rhs: Point<T>) -> Point<T> {
        Point {
            x: self.x - rhs.x,
            y: self.y - rhs.y,
        }
    }
}

impl<T: Default> Point<T> {
    pub fn zero() -> Self {
        Point {
            x: T::default(),
            y: T::default(),
        }
    }
}

impl<T> Point<T> {
    pub fn new(x: T, y: T) -> Self {
        Point { x, y }
    }
}

impl<T: Neg<Output = T>> Neg for Point<T> {
    type Output = Point<T>;
    fn neg(self) -> Point<T> {
        Point {
            x: -self.x,
            y: -self.y,
        }
    }
}

impl Mul<f32> for Point<Pixels> {
    type Output = Point<Pixels>;
    fn mul(self, rhs: f32) -> Point<Pixels> {
        Point {
            x: self.x * rhs,
            y: self.y * rhs,
        }
    }
}

pub type PixelPoint = Point<Pixels>;

// ---------------------------------------------------------------------------
// Size<T> — 2D dimensions
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Size<T> {
    pub width: T,
    pub height: T,
}

impl<T: Default> Size<T> {
    pub fn zero() -> Self {
        Size {
            width: T::default(),
            height: T::default(),
        }
    }
}

impl<T> Size<T> {
    pub fn new(width: T, height: T) -> Self {
        Size { width, height }
    }
}

impl Size<Pixels> {
    pub fn area(&self) -> f32 {
        self.width.0 * self.height.0
    }

    pub fn contains(&self, other: &Size<Pixels>) -> bool {
        other.width.0 <= self.width.0 && other.height.0 <= self.height.0
    }

    pub fn aspect_ratio(&self) -> f32 {
        if self.height.0 == 0.0 {
            return 0.0;
        }
        self.width.0 / self.height.0
    }
}

// ---------------------------------------------------------------------------
// Bounds<T> — axis-aligned bounding box
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Bounds<T> {
    pub origin: Point<T>,
    pub size: Size<T>,
}

impl<T> Bounds<T>
where
    T: Copy + Add<Output = T> + PartialOrd,
{
    pub fn new(origin: Point<T>, size: Size<T>) -> Self {
        Bounds { origin, size }
    }

    pub fn contains(&self, pt: &Point<T>) -> bool {
        pt.x >= self.origin.x
            && pt.x <= self.origin.x + self.size.width
            && pt.y >= self.origin.y
            && pt.y <= self.origin.y + self.size.height
    }

    pub fn intersects(&self, other: &Bounds<T>) -> bool {
        let self_right = self.origin.x + self.size.width;
        let self_bottom = self.origin.y + self.size.height;
        let other_right = other.origin.x + other.size.width;
        let other_bottom = other.origin.y + other.size.height;
        self.origin.x < other_right
            && self_right > other.origin.x
            && self.origin.y < other_bottom
            && self_bottom > other.origin.y
    }
}

impl Bounds<Pixels> {
    /// Returns the center point of this bounds.
    pub fn center(&self) -> Point<Pixels> {
        Point {
            x: Pixels(self.origin.x.0 + self.size.width.0 / 2.0),
            y: Pixels(self.origin.y.0 + self.size.height.0 / 2.0),
        }
    }

    /// Returns a new Bounds expanded outward by `amount` on all sides.
    pub fn expand(&self, amount: Pixels) -> Bounds<Pixels> {
        Bounds {
            origin: Point {
                x: Pixels(self.origin.x.0 - amount.0),
                y: Pixels(self.origin.y.0 - amount.0),
            },
            size: Size {
                width: Pixels(self.size.width.0 + amount.0 * 2.0),
                height: Pixels(self.size.height.0 + amount.0 * 2.0),
            },
        }
    }

    /// Returns the area (width * height) of this bounds.
    pub fn area(&self) -> f32 {
        self.size.area()
    }
}

impl Point<Pixels> {
    /// Euclidean distance to another point.
    pub fn distance(&self, other: Point<Pixels>) -> f32 {
        let dx = self.x.0 - other.x.0;
        let dy = self.y.0 - other.y.0;
        (dx * dx + dy * dy).sqrt()
    }
}

pub type PixelBounds = Bounds<Pixels>;

// ---------------------------------------------------------------------------
// Edges<T> — per-edge values (top / right / bottom / left)
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Edges<T> {
    pub top: T,
    pub right: T,
    pub bottom: T,
    pub left: T,
}

impl<T: Copy> Edges<T> {
    pub fn all(value: T) -> Self {
        Edges {
            top: value,
            right: value,
            bottom: value,
            left: value,
        }
    }
}

// ---------------------------------------------------------------------------
// Corners<T> — per-corner values
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Corners<T> {
    pub top_left: T,
    pub top_right: T,
    pub bottom_right: T,
    pub bottom_left: T,
}

impl<T: Copy> Corners<T> {
    pub fn all(value: T) -> Self {
        Corners {
            top_left: value,
            top_right: value,
            bottom_right: value,
            bottom_left: value,
        }
    }
}

// ---------------------------------------------------------------------------
// Hsla — h: 0-360, s/l/a: 0-1
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Hsla {
    pub h: f32,
    pub s: f32,
    pub l: f32,
    pub a: f32,
}

impl Hsla {
    pub fn new(h: f32, s: f32, l: f32, a: f32) -> Self {
        Hsla { h, s, l, a }
    }

    pub fn transparent() -> Self {
        Hsla {
            h: 0.0,
            s: 0.0,
            l: 0.0,
            a: 0.0,
        }
    }

    pub fn black() -> Self {
        Hsla {
            h: 0.0,
            s: 0.0,
            l: 0.0,
            a: 1.0,
        }
    }

    pub fn white() -> Self {
        Hsla {
            h: 0.0,
            s: 0.0,
            l: 1.0,
            a: 1.0,
        }
    }

    pub fn with_alpha(mut self, a: f32) -> Self {
        self.a = a;
        self
    }

    /// Convert to (r, g, b, a) in [0, 1].
    pub fn to_rgba(self) -> (f32, f32, f32, f32) {
        let h = self.h / 360.0;
        let s = self.s;
        let l = self.l;
        let q = if l < 0.5 {
            l * (1.0 + s)
        } else {
            l + s - l * s
        };
        let p = 2.0 * l - q;
        let hue_to_rgb = |mut t: f32| -> f32 {
            if t < 0.0 {
                t += 1.0;
            }
            if t > 1.0 {
                t -= 1.0;
            }
            if t < 1.0 / 6.0 {
                return p + (q - p) * 6.0 * t;
            }
            if t < 1.0 / 2.0 {
                return q;
            }
            if t < 2.0 / 3.0 {
                return p + (q - p) * (2.0 / 3.0 - t) * 6.0;
            }
            p
        };
        if s == 0.0 {
            (l, l, l, self.a)
        } else {
            (
                hue_to_rgb(h + 1.0 / 3.0),
                hue_to_rgb(h),
                hue_to_rgb(h - 1.0 / 3.0),
                self.a,
            )
        }
    }
}

// ---------------------------------------------------------------------------
// Vec2 — floating-point 2D vector
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Vec2 {
    pub x: f32,
    pub y: f32,
}

impl Add for Vec2 {
    type Output = Vec2;
    fn add(self, rhs: Vec2) -> Vec2 {
        Vec2 {
            x: self.x + rhs.x,
            y: self.y + rhs.y,
        }
    }
}

impl Sub for Vec2 {
    type Output = Vec2;
    fn sub(self, rhs: Vec2) -> Vec2 {
        Vec2 {
            x: self.x - rhs.x,
            y: self.y - rhs.y,
        }
    }
}

impl Mul<f32> for Vec2 {
    type Output = Vec2;
    fn mul(self, rhs: f32) -> Vec2 {
        Vec2 {
            x: self.x * rhs,
            y: self.y * rhs,
        }
    }
}

impl Vec2 {
    pub fn new(x: f32, y: f32) -> Self {
        Vec2 { x, y }
    }

    pub fn zero() -> Self {
        Vec2 { x: 0.0, y: 0.0 }
    }

    pub fn length(self) -> f32 {
        (self.x * self.x + self.y * self.y).sqrt()
    }

    pub fn normalize(self) -> Self {
        let len = self.length();
        if len == 0.0 {
            Vec2::zero()
        } else {
            Vec2 {
                x: self.x / len,
                y: self.y / len,
            }
        }
    }

    pub fn dot(self, rhs: Vec2) -> f32 {
        self.x * rhs.x + self.y * rhs.y
    }
}

// ---------------------------------------------------------------------------
// ContentMask<T> — GPU clipping region per primitive
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct ContentMask<T> {
    pub bounds: Bounds<T>,
}

impl<T: Copy + Default> ContentMask<T> {
    pub fn new(bounds: Bounds<T>) -> Self {
        ContentMask { bounds }
    }
}

// ---------------------------------------------------------------------------
// AtlasBounds — pixel location in glyph atlas
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct AtlasBounds {
    pub left: u32,
    pub top: u32,
    pub right: u32,
    pub bottom: u32,
}

impl AtlasBounds {
    pub fn new(left: u32, top: u32, right: u32, bottom: u32) -> Self {
        AtlasBounds {
            left,
            top,
            right,
            bottom,
        }
    }
}

// ---------------------------------------------------------------------------
// AtlasTile — reference to a packed glyph tile
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct AtlasTile {
    pub texture_id: u32,
    pub bounds: AtlasBounds,
    pub padding: f32,
}

impl AtlasTile {
    pub fn new(texture_id: u32, bounds: AtlasBounds, padding: f32) -> Self {
        AtlasTile {
            texture_id,
            bounds,
            padding,
        }
    }
}

// ---------------------------------------------------------------------------
// PathVertex<T> — vertex for Path primitive
// ---------------------------------------------------------------------------

#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct PathVertex<T> {
    pub x: T,
    pub y: T,
    pub z: T,
}

impl<T> PathVertex<T> {
    pub fn new(x: T, y: T, z: T) -> Self {
        PathVertex { x, y, z }
    }
}

// ---------------------------------------------------------------------------
// TransformationMatrix — 4x4 f32 matrix (default = identity)
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TransformationMatrix(pub [[f32; 4]; 4]);

impl Default for TransformationMatrix {
    fn default() -> Self {
        TransformationMatrix([
            [1.0, 0.0, 0.0, 0.0],
            [0.0, 1.0, 0.0, 0.0],
            [0.0, 0.0, 1.0, 0.0],
            [0.0, 0.0, 0.0, 1.0],
        ])
    }
}

impl TransformationMatrix {
    pub fn identity() -> Self {
        Self::default()
    }
}

// ---------------------------------------------------------------------------
// ElementId / GlobalElementId — stable element identity
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct ElementId(pub u64);

impl ElementId {
    pub fn new(id: u64) -> Self {
        ElementId(id)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct GlobalElementId(pub Vec<ElementId>);

impl GlobalElementId {
    pub fn new() -> Self {
        GlobalElementId(Vec::new())
    }

    pub fn push(&mut self, id: ElementId) {
        self.0.push(id);
    }

    pub fn pop(&mut self) -> Option<ElementId> {
        self.0.pop()
    }
}

// ---------------------------------------------------------------------------
// LayoutId — newtype over a taffy node identifier (stored as u64)
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct LayoutId(pub u64);

impl LayoutId {
    pub fn new(id: u64) -> Self {
        LayoutId(id)
    }
}

// ---------------------------------------------------------------------------
// FontId
// ---------------------------------------------------------------------------

pub type FontId = u32;

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn color_types_roundtrip() {
        // Hsla constructors and field access
        let black = Hsla::black();
        assert_eq!(black.h, 0.0);
        assert_eq!(black.s, 0.0);
        assert_eq!(black.l, 0.0);
        assert_eq!(black.a, 1.0);

        let white = Hsla::white();
        assert_eq!(white.l, 1.0);

        let transparent = Hsla::transparent();
        assert_eq!(transparent.a, 0.0);

        // with_alpha preserves other fields
        let semi = Hsla::new(180.0, 0.5, 0.5, 1.0).with_alpha(0.5);
        assert!((semi.a - 0.5).abs() < 1e-6);
        assert_eq!(semi.h, 180.0);

        // AtlasTile roundtrip
        let bounds = AtlasBounds::new(10, 20, 30, 40);
        assert_eq!(bounds.left, 10);
        assert_eq!(bounds.top, 20);
        assert_eq!(bounds.right, 30);
        assert_eq!(bounds.bottom, 40);

        let tile = AtlasTile::new(7, bounds, 1.5);
        assert_eq!(tile.texture_id, 7);
        assert_eq!(tile.padding, 1.5);
        assert_eq!(tile.bounds, bounds);
    }

    #[test]
    fn size_bounds_constructors() {
        // Point arithmetic
        let a = Point::new(Pixels(3.0), Pixels(4.0));
        let b = Point::new(Pixels(1.0), Pixels(2.0));
        let sum = a + b;
        assert_eq!(sum.x, Pixels(4.0));
        assert_eq!(sum.y, Pixels(6.0));
        let diff = a - b;
        assert_eq!(diff.x, Pixels(2.0));
        assert_eq!(diff.y, Pixels(2.0));

        // Size constructor
        let s = Size::new(Pixels(100.0), Pixels(200.0));
        assert_eq!(s.width, Pixels(100.0));
        assert_eq!(s.height, Pixels(200.0));

        // Bounds contains
        let origin = Point::new(Pixels(0.0), Pixels(0.0));
        let b = Bounds::new(origin, s);
        assert!(b.contains(&Point::new(Pixels(50.0), Pixels(100.0))));
        assert!(!b.contains(&Point::new(Pixels(150.0), Pixels(100.0))));

        // Edges::all and Corners::all
        let e = Edges::all(Pixels(5.0));
        assert_eq!(e.top, Pixels(5.0));
        assert_eq!(e.right, Pixels(5.0));
        assert_eq!(e.bottom, Pixels(5.0));
        assert_eq!(e.left, Pixels(5.0));

        let c = Corners::all(Pixels(8.0));
        assert_eq!(c.top_left, Pixels(8.0));
        assert_eq!(c.bottom_right, Pixels(8.0));

        // Pixels arithmetic
        let p = Pixels(10.0) + Pixels(5.0);
        assert_eq!(p, Pixels(15.0));
        let p = Pixels(10.0) - Pixels(3.0);
        assert_eq!(p, Pixels(7.0));
        let p = Pixels(4.0) * 2.5;
        assert_eq!(p, Pixels(10.0));
        let p = Pixels(9.0) / 3.0;
        assert_eq!(p, Pixels(3.0));

        // TransformationMatrix identity
        let m = TransformationMatrix::identity();
        assert_eq!(m.0[0][0], 1.0);
        assert_eq!(m.0[1][1], 1.0);
        assert_eq!(m.0[2][2], 1.0);
        assert_eq!(m.0[3][3], 1.0);
        assert_eq!(m.0[0][1], 0.0);

        // Vec2 operations
        let v = Vec2::new(3.0, 4.0);
        assert!((v.length() - 5.0).abs() < 1e-6);
        let n = v.normalize();
        assert!((n.x - 0.6).abs() < 1e-6);
        assert!((n.y - 0.8).abs() < 1e-6);

        // GlobalElementId push/pop
        let mut gid = GlobalElementId::new();
        gid.push(ElementId::new(1));
        gid.push(ElementId::new(2));
        assert_eq!(gid.pop(), Some(ElementId::new(2)));
        assert_eq!(gid.0.len(), 1);
    }

    // ---- Pixels extended ----

    #[test]
    fn pixels_add() {
        assert_eq!(Pixels(2.0) + Pixels(3.0), Pixels(5.0));
    }

    #[test]
    fn pixels_sub() {
        assert_eq!(Pixels(5.0) - Pixels(2.0), Pixels(3.0));
    }

    #[test]
    fn pixels_mul_f32() {
        assert_eq!(Pixels(4.0) * 2.0, Pixels(8.0));
    }

    #[test]
    fn pixels_zero() {
        assert_eq!(Pixels::zero(), Pixels(0.0));
    }

    #[test]
    fn pixels_floor() {
        assert_eq!(Pixels(3.7).floor(), Pixels(3.0));
    }

    #[test]
    fn pixels_ceil() {
        assert_eq!(Pixels(3.2).ceil(), Pixels(4.0));
    }

    #[test]
    fn pixels_abs() {
        assert_eq!(Pixels(-3.0).abs(), Pixels(3.0));
    }

    #[test]
    fn pixels_ord() {
        assert!(Pixels(1.0) < Pixels(2.0));
        assert!(!(Pixels(2.0) < Pixels(1.0)));
    }

    // ---- Point<Pixels> ----

    #[test]
    fn point_add() {
        let a = Point::new(Pixels(1.0), Pixels(2.0));
        let b = Point::new(Pixels(3.0), Pixels(4.0));
        let c = a + b;
        assert_eq!(c.x, Pixels(4.0));
        assert_eq!(c.y, Pixels(6.0));
    }

    #[test]
    fn point_scale() {
        let p = Point::new(Pixels(3.0), Pixels(5.0)) * 2.0;
        assert_eq!(p.x, Pixels(6.0));
        assert_eq!(p.y, Pixels(10.0));
    }

    #[test]
    fn point_zero() {
        let p: Point<Pixels> = Point::zero();
        assert_eq!(p.x, Pixels(0.0));
        assert_eq!(p.y, Pixels(0.0));
    }

    #[test]
    fn point_negate() {
        let p = -Point::new(Pixels(1.0), Pixels(2.0));
        assert_eq!(p.x, Pixels(-1.0));
        assert_eq!(p.y, Pixels(-2.0));
    }

    // ---- Size<Pixels> ----

    #[test]
    fn size_area() {
        let s = Size::new(Pixels(3.0), Pixels(4.0));
        assert!((s.area() - 12.0).abs() < 1e-6);
    }

    #[test]
    fn size_zero() {
        let s: Size<Pixels> = Size::zero();
        assert_eq!(s.width, Pixels(0.0));
        assert_eq!(s.height, Pixels(0.0));
    }

    #[test]
    fn size_contains_smaller() {
        let big = Size::new(Pixels(10.0), Pixels(10.0));
        let small = Size::new(Pixels(5.0), Pixels(5.0));
        assert!(big.contains(&small));
        assert!(!small.contains(&big));
    }

    // ---- Bounds<Pixels> ----

    #[test]
    fn bounds_contains_point() {
        let b = Bounds::new(
            Point::new(Pixels(0.0), Pixels(0.0)),
            Size::new(Pixels(10.0), Pixels(10.0)),
        );
        assert!(b.contains(&Point::new(Pixels(5.0), Pixels(5.0))));
    }

    #[test]
    fn bounds_contains_edge() {
        let b = Bounds::new(
            Point::new(Pixels(0.0), Pixels(0.0)),
            Size::new(Pixels(10.0), Pixels(10.0)),
        );
        assert!(b.contains(&Point::new(Pixels(0.0), Pixels(0.0))));
        assert!(b.contains(&Point::new(Pixels(10.0), Pixels(10.0))));
    }

    #[test]
    fn bounds_does_not_contain_outside() {
        let b = Bounds::new(
            Point::new(Pixels(0.0), Pixels(0.0)),
            Size::new(Pixels(10.0), Pixels(10.0)),
        );
        assert!(!b.contains(&Point::new(Pixels(11.0), Pixels(5.0))));
    }

    #[test]
    fn bounds_intersects() {
        let a = Bounds::new(
            Point::new(Pixels(0.0), Pixels(0.0)),
            Size::new(Pixels(10.0), Pixels(10.0)),
        );
        let b = Bounds::new(
            Point::new(Pixels(5.0), Pixels(5.0)),
            Size::new(Pixels(10.0), Pixels(10.0)),
        );
        assert!(a.intersects(&b));
    }

    #[test]
    fn bounds_no_intersect() {
        let a = Bounds::new(
            Point::new(Pixels(0.0), Pixels(0.0)),
            Size::new(Pixels(5.0), Pixels(5.0)),
        );
        let b = Bounds::new(
            Point::new(Pixels(10.0), Pixels(10.0)),
            Size::new(Pixels(5.0), Pixels(5.0)),
        );
        assert!(!a.intersects(&b));
    }

    // ---- Hsla ----

    #[test]
    fn hsla_fields() {
        let c = Hsla::new(120.0, 0.5, 0.4, 0.8);
        assert_eq!(c.h, 120.0);
        assert_eq!(c.s, 0.5);
        assert_eq!(c.l, 0.4);
        assert_eq!(c.a, 0.8);
    }

    #[test]
    fn hsla_to_rgba_gray() {
        // s==0 means achromatic: r==g==b==l
        let (r, g, b, a) = Hsla::new(0.0, 0.0, 0.5, 1.0).to_rgba();
        assert!((r - 0.5).abs() < 1e-5, "r={r}");
        assert!((g - 0.5).abs() < 1e-5, "g={g}");
        assert!((b - 0.5).abs() < 1e-5, "b={b}");
        assert!((a - 1.0).abs() < 1e-5, "a={a}");
    }

    #[test]
    fn hsla_alpha_range() {
        let c = Hsla::new(200.0, 0.3, 0.6, 0.75);
        assert!(c.a >= 0.0 && c.a <= 1.0);
    }

    // ---- Vec2 ----

    #[test]
    fn vec2_dot() {
        let a = Vec2::new(1.0, 0.0);
        let b = Vec2::new(0.0, 1.0);
        assert!((a.dot(b)).abs() < 1e-6);
        assert!((Vec2::new(2.0, 3.0).dot(Vec2::new(4.0, 5.0)) - 23.0).abs() < 1e-6);
    }

    #[test]
    fn vec2_length() {
        assert!((Vec2::new(3.0, 4.0).length() - 5.0).abs() < 1e-6);
    }

    // ---- New tests: bounds_area, bounds_center, bounds_expand, point_distance, size_aspect_ratio ----

    #[test]
    fn bounds_area() {
        let b = Bounds::new(
            Point::new(Pixels(0.0), Pixels(0.0)),
            Size::new(Pixels(4.0), Pixels(5.0)),
        );
        assert!((b.area() - 20.0).abs() < 1e-6);
    }

    #[test]
    fn bounds_center() {
        let b = Bounds::new(
            Point::new(Pixels(0.0), Pixels(0.0)),
            Size::new(Pixels(10.0), Pixels(10.0)),
        );
        let c = b.center();
        assert_eq!(c.x, Pixels(5.0));
        assert_eq!(c.y, Pixels(5.0));
    }

    #[test]
    fn bounds_expand() {
        let b = Bounds::new(
            Point::new(Pixels(10.0), Pixels(10.0)),
            Size::new(Pixels(20.0), Pixels(20.0)),
        );
        let expanded = b.expand(Pixels(2.0));
        assert_eq!(expanded.origin.x, Pixels(8.0));
        assert_eq!(expanded.origin.y, Pixels(8.0));
        assert_eq!(expanded.size.width, Pixels(24.0));
        assert_eq!(expanded.size.height, Pixels(24.0));
    }

    #[test]
    fn point_distance() {
        let a = Point::new(Pixels(0.0), Pixels(0.0));
        let b = Point::new(Pixels(3.0), Pixels(4.0));
        assert!((a.distance(b) - 5.0).abs() < 1e-6);
    }

    #[test]
    fn size_aspect_ratio() {
        let s = Size::new(Pixels(16.0), Pixels(9.0));
        let ratio = s.aspect_ratio();
        assert!((ratio - 16.0 / 9.0).abs() < 1e-5, "ratio={ratio}");
    }

    #[test]
    fn bounds_contains_point_on_edge() {
        let origin = Point::new(Pixels(10.0), Pixels(20.0));
        let size = Size::new(Pixels(100.0), Pixels(50.0));
        let bounds = Bounds::new(origin, size);

        // Points exactly on each edge must be considered inside (inclusive).
        assert!(
            bounds.contains(&Point::new(Pixels(10.0), Pixels(30.0))),
            "left edge"
        );
        assert!(
            bounds.contains(&Point::new(Pixels(110.0), Pixels(30.0))),
            "right edge"
        );
        assert!(
            bounds.contains(&Point::new(Pixels(60.0), Pixels(20.0))),
            "top edge"
        );
        assert!(
            bounds.contains(&Point::new(Pixels(60.0), Pixels(70.0))),
            "bottom edge"
        );

        // Corners must also be inside.
        assert!(
            bounds.contains(&Point::new(Pixels(10.0), Pixels(20.0))),
            "top-left corner"
        );
        assert!(
            bounds.contains(&Point::new(Pixels(110.0), Pixels(70.0))),
            "bottom-right corner"
        );

        // One pixel outside each edge must be excluded.
        assert!(
            !bounds.contains(&Point::new(Pixels(9.9), Pixels(30.0))),
            "just outside left"
        );
        assert!(
            !bounds.contains(&Point::new(Pixels(110.1), Pixels(30.0))),
            "just outside right"
        );
        assert!(
            !bounds.contains(&Point::new(Pixels(60.0), Pixels(19.9))),
            "just outside top"
        );
        assert!(
            !bounds.contains(&Point::new(Pixels(60.0), Pixels(70.1))),
            "just outside bottom"
        );
    }

    // ---- LinearRgba-like clamp behaviour via Hsla (alpha=0 and alpha=1) ----

    #[test]
    fn hsla_alpha_zero_is_fully_transparent() {
        let c = Hsla::new(60.0, 0.5, 0.5, 0.0);
        assert_eq!(c.a, 0.0, "alpha=0 must be fully transparent");
        let (_, _, _, a) = c.to_rgba();
        assert!((a - 0.0).abs() < 1e-6, "to_rgba alpha must be 0.0");
    }

    #[test]
    fn hsla_alpha_one_is_fully_opaque() {
        let c = Hsla::new(60.0, 0.5, 0.5, 1.0);
        assert_eq!(c.a, 1.0, "alpha=1 must be fully opaque");
        let (_, _, _, a) = c.to_rgba();
        assert!((a - 1.0).abs() < 1e-6, "to_rgba alpha must be 1.0");
    }

    #[test]
    fn hsla_with_alpha_clamps_does_not_panic_for_valid_range() {
        // with_alpha simply stores the value — caller ensures [0,1].
        // Test boundary values.
        let c = Hsla::new(0.0, 0.0, 0.0, 1.0).with_alpha(0.0);
        assert_eq!(c.a, 0.0);
        let c2 = Hsla::new(0.0, 0.0, 0.0, 0.0).with_alpha(1.0);
        assert_eq!(c2.a, 1.0);
    }

    #[test]
    fn hsla_to_rgba_red_hue() {
        // Pure red: h=0, s=1, l=0.5 → (1.0, 0.0, 0.0, 1.0)
        let (r, g, b, a) = Hsla::new(0.0, 1.0, 0.5, 1.0).to_rgba();
        assert!((r - 1.0).abs() < 1e-5, "red channel should be 1.0, got {r}");
        assert!(g < 0.01, "green channel should be ~0.0, got {g}");
        assert!(b < 0.01, "blue channel should be ~0.0, got {b}");
        assert!((a - 1.0).abs() < 1e-5);
    }

    // ---- Pixels arithmetic: large values stay representable ----

    #[test]
    fn pixels_large_values_do_not_overflow_f32() {
        // f32 can represent values up to ~3.4e38; use GPU-realistic large coords.
        let big = Pixels(100_000.0);
        let result = big + big;
        assert_eq!(result, Pixels(200_000.0));
    }

    #[test]
    fn pixels_negative_values_work() {
        let p = Pixels(-50.0);
        assert_eq!(p.abs(), Pixels(50.0));
        assert_eq!(-p, Pixels(50.0));
    }

    #[test]
    fn pixels_div_produces_correct_result() {
        assert_eq!(Pixels(10.0) / 4.0, Pixels(2.5));
    }

    #[test]
    fn pixels_from_u32_conversion() {
        let p = Pixels::from(42u32);
        assert_eq!(p, Pixels(42.0));
    }

    #[test]
    fn pixels_from_f32_conversion() {
        let p = Pixels::from(3.14f32);
        assert!((p.0 - 3.14).abs() < 1e-5);
    }

    // ---- Bounds::contains edge cases ----

    #[test]
    fn bounds_contains_zero_size() {
        // A zero-size bounds at origin only contains its own origin point.
        let b = Bounds::new(
            Point::new(Pixels(5.0), Pixels(5.0)),
            Size::new(Pixels(0.0), Pixels(0.0)),
        );
        assert!(b.contains(&Point::new(Pixels(5.0), Pixels(5.0))));
        assert!(!b.contains(&Point::new(Pixels(5.1), Pixels(5.0))));
    }

    #[test]
    fn bounds_contains_negative_origin() {
        let b = Bounds::new(
            Point::new(Pixels(-10.0), Pixels(-10.0)),
            Size::new(Pixels(20.0), Pixels(20.0)),
        );
        assert!(b.contains(&Point::new(Pixels(0.0), Pixels(0.0))));
        assert!(b.contains(&Point::new(Pixels(-10.0), Pixels(-10.0))));
        assert!(!b.contains(&Point::new(Pixels(11.0), Pixels(0.0))));
    }

    // ---- Vec2 additional ops ----

    #[test]
    fn vec2_sub() {
        let a = Vec2::new(5.0, 3.0);
        let b = Vec2::new(2.0, 1.0);
        let c = a - b;
        assert!((c.x - 3.0).abs() < 1e-6);
        assert!((c.y - 2.0).abs() < 1e-6);
    }

    #[test]
    fn vec2_mul_scalar() {
        let v = Vec2::new(3.0, 4.0) * 2.0;
        assert!((v.x - 6.0).abs() < 1e-6);
        assert!((v.y - 8.0).abs() < 1e-6);
    }

    #[test]
    fn vec2_normalize_zero_returns_zero() {
        let v = Vec2::zero().normalize();
        assert_eq!(v.x, 0.0);
        assert_eq!(v.y, 0.0);
    }

    // ---- Size aspect_ratio edge cases ----

    #[test]
    fn size_aspect_ratio_zero_height_returns_zero() {
        let s = Size::new(Pixels(100.0), Pixels(0.0));
        assert_eq!(s.aspect_ratio(), 0.0);
    }

    #[test]
    fn size_aspect_ratio_square_is_one() {
        let s = Size::new(Pixels(50.0), Pixels(50.0));
        assert!((s.aspect_ratio() - 1.0).abs() < 1e-6);
    }

    // ---- ContentMask ----

    #[test]
    fn content_mask_new_stores_bounds() {
        let b = Bounds::new(
            Point::new(Pixels(0.0), Pixels(0.0)),
            Size::new(Pixels(100.0), Pixels(100.0)),
        );
        let mask = ContentMask::new(b);
        assert_eq!(mask.bounds, b);
    }

    // ---- PathVertex ----

    #[test]
    fn path_vertex_new_stores_components() {
        let v = PathVertex::new(Pixels(1.0), Pixels(2.0), Pixels(3.0));
        assert_eq!(v.x, Pixels(1.0));
        assert_eq!(v.y, Pixels(2.0));
        assert_eq!(v.z, Pixels(3.0));
    }

    // ---- GlobalElementId ----

    #[test]
    fn global_element_id_empty_pop_returns_none() {
        let mut gid = GlobalElementId::new();
        assert_eq!(gid.pop(), None);
    }

    #[test]
    fn global_element_id_multiple_push_pop() {
        let mut gid = GlobalElementId::new();
        for i in 0..5 {
            gid.push(ElementId::new(i));
        }
        assert_eq!(gid.0.len(), 5);
        // Pop returns last pushed first (stack semantics)
        assert_eq!(gid.pop(), Some(ElementId::new(4)));
        assert_eq!(gid.0.len(), 4);
    }

    // ---- Pixels: saturating-like ops using f32 ----

    #[test]
    fn pixels_add_negative_gives_correct_result() {
        // Adding a negative pixel value is valid and well-defined.
        let result = Pixels(10.0) + Pixels(-3.0);
        assert_eq!(result, Pixels(7.0));
    }

    #[test]
    fn pixels_sub_to_negative() {
        let result = Pixels(5.0) - Pixels(8.0);
        assert_eq!(result, Pixels(-3.0));
    }

    #[test]
    fn pixels_mul_by_zero() {
        let result = Pixels(42.0) * 0.0;
        assert_eq!(result, Pixels(0.0));
    }

    #[test]
    fn pixels_mul_by_negative() {
        let result = Pixels(5.0) * -1.0;
        assert_eq!(result, Pixels(-5.0));
    }

    #[test]
    fn pixels_floor_already_integer() {
        assert_eq!(Pixels(4.0).floor(), Pixels(4.0));
    }

    #[test]
    fn pixels_ceil_already_integer() {
        assert_eq!(Pixels(7.0).ceil(), Pixels(7.0));
    }

    #[test]
    fn pixels_neg_double_negation() {
        let p = Pixels(3.0);
        assert_eq!(-(-p), p);
    }

    #[test]
    fn pixels_zero_abs_is_zero() {
        assert_eq!(Pixels(0.0).abs(), Pixels(0.0));
    }

    // ---- Bounds: more edge-case geometry ----

    #[test]
    fn bounds_intersects_touching_edges_excluded() {
        // Two adjacent (touching) rects — they share an edge but strictly
        // the intersection check is exclusive on the touching sides.
        let a = Bounds::new(
            Point::new(Pixels(0.0), Pixels(0.0)),
            Size::new(Pixels(10.0), Pixels(10.0)),
        );
        let b = Bounds::new(
            Point::new(Pixels(10.0), Pixels(0.0)),
            Size::new(Pixels(10.0), Pixels(10.0)),
        );
        // Right edge of a == left edge of b → not strictly overlapping.
        assert!(!a.intersects(&b));
    }

    #[test]
    fn bounds_expand_zero_amount_unchanged() {
        let b = Bounds::new(
            Point::new(Pixels(5.0), Pixels(5.0)),
            Size::new(Pixels(10.0), Pixels(10.0)),
        );
        let expanded = b.expand(Pixels(0.0));
        assert_eq!(expanded, b);
    }

    #[test]
    fn bounds_expand_large_amount() {
        let b = Bounds::new(
            Point::new(Pixels(0.0), Pixels(0.0)),
            Size::new(Pixels(10.0), Pixels(10.0)),
        );
        let e = b.expand(Pixels(100.0));
        assert_eq!(e.origin.x, Pixels(-100.0));
        assert_eq!(e.size.width, Pixels(210.0));
    }

    #[test]
    fn bounds_center_offset_origin() {
        let b = Bounds::new(
            Point::new(Pixels(10.0), Pixels(20.0)),
            Size::new(Pixels(40.0), Pixels(20.0)),
        );
        let c = b.center();
        assert!((c.x.0 - 30.0).abs() < 1e-5, "cx={}", c.x.0);
        assert!((c.y.0 - 30.0).abs() < 1e-5, "cy={}", c.y.0);
    }

    #[test]
    fn size_contains_equal_is_true() {
        let s = Size::new(Pixels(10.0), Pixels(10.0));
        assert!(s.contains(&s));
    }

    // ---- Vec2 additional coverage ----

    #[test]
    fn vec2_zero_length_is_zero() {
        assert!((Vec2::zero().length()).abs() < 1e-6);
    }

    #[test]
    fn vec2_add_zero_unchanged() {
        let v = Vec2::new(3.0, 4.0);
        let result = v + Vec2::zero();
        assert!((result.x - v.x).abs() < 1e-6);
        assert!((result.y - v.y).abs() < 1e-6);
    }

    #[test]
    fn vec2_dot_with_self_equals_length_squared() {
        let v = Vec2::new(3.0, 4.0);
        let dot = v.dot(v);
        let len_sq = v.length() * v.length();
        assert!((dot - len_sq).abs() < 1e-5);
    }

    // ---- Corners ----

    #[test]
    fn corners_all_sets_all_fields() {
        let c = Corners::all(Pixels(12.0));
        assert_eq!(c.top_left, Pixels(12.0));
        assert_eq!(c.top_right, Pixels(12.0));
        assert_eq!(c.bottom_right, Pixels(12.0));
        assert_eq!(c.bottom_left, Pixels(12.0));
    }

    #[test]
    fn corners_default_is_zero() {
        let c = Corners::<Pixels>::default();
        assert_eq!(c.top_left, Pixels(0.0));
    }

    // ---- Edges ----

    #[test]
    fn edges_all_sets_all_fields() {
        let e = Edges::all(Pixels(3.0));
        assert_eq!(e.top, Pixels(3.0));
        assert_eq!(e.right, Pixels(3.0));
        assert_eq!(e.bottom, Pixels(3.0));
        assert_eq!(e.left, Pixels(3.0));
    }

    #[test]
    fn edges_default_is_zero() {
        let e = Edges::<Pixels>::default();
        assert_eq!(e.top, Pixels(0.0));
        assert_eq!(e.left, Pixels(0.0));
    }

    // ---- AtlasBounds / AtlasTile ----

    #[test]
    fn atlas_bounds_default_is_zero() {
        let b = AtlasBounds::default();
        assert_eq!(b.left, 0);
        assert_eq!(b.top, 0);
        assert_eq!(b.right, 0);
        assert_eq!(b.bottom, 0);
    }

    #[test]
    fn atlas_tile_default_padding_is_zero() {
        let t = AtlasTile::default();
        assert!((t.padding - 0.0).abs() < 1e-6);
    }

    // ---- LayoutId ----

    #[test]
    fn layout_id_new_stores_value() {
        let id = LayoutId::new(42);
        assert_eq!(id.0, 42);
    }

    #[test]
    fn layout_id_equality() {
        assert_eq!(LayoutId::new(7), LayoutId::new(7));
        assert_ne!(LayoutId::new(7), LayoutId::new(8));
    }

    // ---- ElementId ----

    #[test]
    fn element_id_new_stores_value() {
        let id = ElementId::new(100);
        assert_eq!(id.0, 100);
    }
}
