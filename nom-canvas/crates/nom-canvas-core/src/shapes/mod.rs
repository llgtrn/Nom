//! Canvas shape primitives — the 8-variant enum that every [`Element`] carries.
//!
//! [`Element`]: crate::element::Element

use nom_gpui::{Bounds, Pixels, Point};

/// Axis-aligned rectangle: filled, stroked, or both.
#[derive(Clone, Debug)]
pub struct Rectangle {}

/// Ellipse inscribed in the element bounds.
#[derive(Clone, Debug)]
pub struct Ellipse {}

/// Diamond (rhombus) inscribed in the element bounds.
#[derive(Clone, Debug)]
pub struct Diamond {}

/// Straight line segment between two endpoints.
#[derive(Clone, Debug)]
pub struct Line {
    pub endpoints: [Point<Pixels>; 2],
}

/// Arrow with optional waypoints and elbow-routing flag.
#[derive(Clone, Debug)]
pub struct Arrow {
    /// Ordered control points including start and end.
    pub waypoints: Vec<Point<Pixels>>,
    /// When `true` the renderer applies elbow (right-angle) routing.
    pub elbowed: bool,
}

/// Text label with a logical font size.
#[derive(Clone, Debug)]
pub struct Text {
    pub content: String,
    pub font_size: Pixels,
}

/// Freehand stroke captured as a sequence of pressure-sensitive points.
#[derive(Clone, Debug)]
pub struct FreeDraw {
    /// Screen-space control points in drawing order.
    pub points: Vec<Point<Pixels>>,
    /// Stylus pressure per point, same length as `points` (0.0 = none, 1.0 = full).
    pub pressures: Vec<f32>,
}

/// Raster image with an optional crop rectangle.
#[derive(Clone, Debug)]
pub struct Image {
    /// Content-addressed file identifier (maps to an asset store entry).
    pub file_id: String,
    /// If set, only this sub-region of the source image is rendered.
    pub crop: Option<Bounds<Pixels>>,
}

/// All possible shapes an [`Element`] can render.
///
/// [`Element`]: crate::element::Element
#[derive(Clone, Debug)]
pub enum Shape {
    Rectangle(Rectangle),
    Ellipse(Ellipse),
    Diamond(Diamond),
    Line(Line),
    Arrow(Arrow),
    Text(Text),
    FreeDraw(FreeDraw),
    Image(Image),
}
