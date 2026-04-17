use std::ops::{Add, Sub, Mul, Div};

// ---------------------------------------------------------------------------
// Pixels — primary unit for GPU rendering coordinates
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, PartialOrd, Default)]
pub struct Pixels(pub f32);

impl Add for Pixels {
    type Output = Pixels;
    fn add(self, rhs: Pixels) -> Pixels { Pixels(self.0 + rhs.0) }
}

impl Sub for Pixels {
    type Output = Pixels;
    fn sub(self, rhs: Pixels) -> Pixels { Pixels(self.0 - rhs.0) }
}

impl Mul<f32> for Pixels {
    type Output = Pixels;
    fn mul(self, rhs: f32) -> Pixels { Pixels(self.0 * rhs) }
}

impl Div<f32> for Pixels {
    type Output = Pixels;
    fn div(self, rhs: f32) -> Pixels { Pixels(self.0 / rhs) }
}

impl From<f32> for Pixels {
    fn from(v: f32) -> Pixels { Pixels(v) }
}

impl From<u32> for Pixels {
    fn from(v: u32) -> Pixels { Pixels(v as f32) }
}

impl Pixels {
    pub fn new(v: f32) -> Self { Pixels(v) }
    pub fn zero() -> Self { Pixels(0.0) }
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
        Point { x: self.x + rhs.x, y: self.y + rhs.y }
    }
}

impl<T: Sub<Output = T>> Sub for Point<T> {
    type Output = Point<T>;
    fn sub(self, rhs: Point<T>) -> Point<T> {
        Point { x: self.x - rhs.x, y: self.y - rhs.y }
    }
}

impl<T> Point<T> {
    pub fn new(x: T, y: T) -> Self { Point { x, y } }
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

impl<T> Size<T> {
    pub fn new(width: T, height: T) -> Self { Size { width, height } }
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
    pub fn new(origin: Point<T>, size: Size<T>) -> Self { Bounds { origin, size } }

    pub fn contains(&self, pt: &Point<T>) -> bool {
        pt.x >= self.origin.x
            && pt.x <= self.origin.x + self.size.width
            && pt.y >= self.origin.y
            && pt.y <= self.origin.y + self.size.height
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
        Edges { top: value, right: value, bottom: value, left: value }
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
    pub fn new(h: f32, s: f32, l: f32, a: f32) -> Self { Hsla { h, s, l, a } }

    pub fn transparent() -> Self { Hsla { h: 0.0, s: 0.0, l: 0.0, a: 0.0 } }

    pub fn black() -> Self { Hsla { h: 0.0, s: 0.0, l: 0.0, a: 1.0 } }

    pub fn white() -> Self { Hsla { h: 0.0, s: 0.0, l: 1.0, a: 1.0 } }

    pub fn with_alpha(mut self, a: f32) -> Self {
        self.a = a;
        self
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
    fn add(self, rhs: Vec2) -> Vec2 { Vec2 { x: self.x + rhs.x, y: self.y + rhs.y } }
}

impl Sub for Vec2 {
    type Output = Vec2;
    fn sub(self, rhs: Vec2) -> Vec2 { Vec2 { x: self.x - rhs.x, y: self.y - rhs.y } }
}

impl Mul<f32> for Vec2 {
    type Output = Vec2;
    fn mul(self, rhs: f32) -> Vec2 { Vec2 { x: self.x * rhs, y: self.y * rhs } }
}

impl Vec2 {
    pub fn new(x: f32, y: f32) -> Self { Vec2 { x, y } }

    pub fn zero() -> Self { Vec2 { x: 0.0, y: 0.0 } }

    pub fn length(self) -> f32 { (self.x * self.x + self.y * self.y).sqrt() }

    pub fn normalize(self) -> Self {
        let len = self.length();
        if len == 0.0 {
            Vec2::zero()
        } else {
            Vec2 { x: self.x / len, y: self.y / len }
        }
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
    pub fn new(bounds: Bounds<T>) -> Self { ContentMask { bounds } }
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
        AtlasBounds { left, top, right, bottom }
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
        AtlasTile { texture_id, bounds, padding }
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
    pub fn new(x: T, y: T, z: T) -> Self { PathVertex { x, y, z } }
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
    pub fn identity() -> Self { Self::default() }
}

// ---------------------------------------------------------------------------
// ElementId / GlobalElementId — stable element identity
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct ElementId(pub u64);

impl ElementId {
    pub fn new(id: u64) -> Self { ElementId(id) }
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct GlobalElementId(pub Vec<ElementId>);

impl GlobalElementId {
    pub fn new() -> Self { GlobalElementId(Vec::new()) }

    pub fn push(&mut self, id: ElementId) { self.0.push(id); }

    pub fn pop(&mut self) -> Option<ElementId> { self.0.pop() }
}

// ---------------------------------------------------------------------------
// LayoutId — newtype over a taffy node identifier (stored as u64)
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct LayoutId(pub u64);

impl LayoutId {
    pub fn new(id: u64) -> Self { LayoutId(id) }
}

// ---------------------------------------------------------------------------
// FontId
// ---------------------------------------------------------------------------

pub type FontId = u32;
