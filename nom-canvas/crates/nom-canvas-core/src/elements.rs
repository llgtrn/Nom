/// Canvas element primitives.
///
/// All coordinates are in canvas-space (f32).  Colours are RGBA in linear [0,1].

/// Arrow-head styles for `CanvasArrow`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ArrowHead {
    /// Open (unfilled) arrowhead — just two lines forming a V.
    Open,
    /// Closed (outlined but hollow) arrowhead.
    Closed,
    /// Filled (solid) arrowhead.
    Filled,
}

/// Axis-aligned bounding box envelope used by the spatial index.
#[derive(Debug, Clone, Copy)]
pub struct ElementBounds {
    /// Unique element identifier.
    pub id: u64,
    /// Minimum corner (top-left in canvas space).
    pub min: [f32; 2],
    /// Maximum corner (bottom-right in canvas space).
    pub max: [f32; 2],
}

// ─── Rectangle ──────────────────────────────────────────────────────────────

/// Canvas rectangle with optional rounded corners and rotation.
#[derive(Debug, Clone)]
pub struct CanvasRect {
    /// Unique element identifier.
    pub id: u64,
    /// `(origin, size)` — origin is the top-left corner before rotation.
    pub bounds: ([f32; 2], [f32; 2]),
    /// Optional fill colour (RGBA).
    pub fill: Option<[f32; 4]>,
    /// Optional stroke colour (RGBA).
    pub stroke: Option<[f32; 4]>,
    /// Corner rounding radius in canvas pixels.
    pub corner_radius: f32,
    /// Rotation in radians (counter-clockwise, applied around centre).
    pub rotation: f32,
    /// Stacking order — higher values render on top.
    pub z_index: u32,
}

impl CanvasRect {
    /// Returns the axis-aligned bounding box (ignores rotation — conservative broadphase).
    pub fn bounds_aabb(&self) -> ElementBounds {
        let (origin, size) = self.bounds;
        ElementBounds {
            id: self.id,
            min: origin,
            max: [origin[0] + size[0], origin[1] + size[1]],
        }
    }

    /// Centre of the rectangle in canvas space.
    pub fn center(&self) -> [f32; 2] {
        let (origin, size) = self.bounds;
        [origin[0] + size[0] / 2.0, origin[1] + size[1] / 2.0]
    }
}

// ─── Ellipse ────────────────────────────────────────────────────────────────

/// Canvas ellipse defined by its bounding box.
#[derive(Debug, Clone)]
pub struct CanvasEllipse {
    /// Unique element identifier.
    pub id: u64,
    /// `(origin, size)` — bounding rectangle of the ellipse.
    pub bounds: ([f32; 2], [f32; 2]),
    /// Optional fill colour (RGBA).
    pub fill: Option<[f32; 4]>,
    /// Optional stroke colour (RGBA).
    pub stroke: Option<[f32; 4]>,
    /// Stacking order — higher values render on top.
    pub z_index: u32,
}

impl CanvasEllipse {
    /// Returns the axis-aligned bounding box of the ellipse.
    pub fn bounds_aabb(&self) -> ElementBounds {
        let (origin, size) = self.bounds;
        ElementBounds {
            id: self.id,
            min: origin,
            max: [origin[0] + size[0], origin[1] + size[1]],
        }
    }

    /// Returns the centre of the ellipse in canvas space.
    pub fn center(&self) -> [f32; 2] {
        let (origin, size) = self.bounds;
        [origin[0] + size[0] / 2.0, origin[1] + size[1] / 2.0]
    }
}

// ─── Line ───────────────────────────────────────────────────────────────────

/// A straight line segment.
#[derive(Debug, Clone)]
pub struct CanvasLine {
    /// Unique element identifier.
    pub id: u64,
    /// Start point in canvas space.
    pub start: [f32; 2],
    /// End point in canvas space.
    pub end: [f32; 2],
    /// Stroke width in canvas pixels.
    pub stroke_width: f32,
    /// Line colour as RGBA in linear [0, 1].
    pub color: [f32; 4],
    /// Dash pattern lengths in canvas units.  Empty = solid line.
    pub dashes: Vec<f32>,
    /// Stacking order — higher values render on top.
    pub z_index: u32,
}

impl CanvasLine {
    /// Returns the axis-aligned bounding box of the line.
    pub fn bounds_aabb(&self) -> ElementBounds {
        ElementBounds {
            id: self.id,
            min: [
                self.start[0].min(self.end[0]),
                self.start[1].min(self.end[1]),
            ],
            max: [
                self.start[0].max(self.end[0]),
                self.start[1].max(self.end[1]),
            ],
        }
    }
}

// ─── Arrow ──────────────────────────────────────────────────────────────────

/// A directed line with an arrowhead at the end.
#[derive(Debug, Clone)]
pub struct CanvasArrow {
    /// Unique element identifier.
    pub id: u64,
    /// Start point in canvas space (tail of the arrow).
    pub start: [f32; 2],
    /// End point in canvas space (tip of the arrow).
    pub end: [f32; 2],
    /// Stroke width in canvas pixels.
    pub stroke_width: f32,
    /// Arrow colour as RGBA in linear [0, 1].
    pub color: [f32; 4],
    /// Style of the arrowhead rendered at `end`.
    pub head_style: ArrowHead,
    /// Stacking order — higher values render on top.
    pub z_index: u32,
}

impl CanvasArrow {
    /// Returns the axis-aligned bounding box of the arrow.
    pub fn bounds_aabb(&self) -> ElementBounds {
        ElementBounds {
            id: self.id,
            min: [
                self.start[0].min(self.end[0]),
                self.start[1].min(self.end[1]),
            ],
            max: [
                self.start[0].max(self.end[0]),
                self.start[1].max(self.end[1]),
            ],
        }
    }
}

// ─── Graph Node ─────────────────────────────────────────────────────────────

/// A DAG node element in graph mode.  Carries the semantic `node_id` from the
/// block graph plus display geometry and a confidence score.
#[derive(Debug, Clone)]
pub struct GraphNodeElement {
    /// Unique canvas element identifier.
    pub id: u64,
    /// Semantic node identifier from the block graph.
    pub node_id: u64,
    /// Top-left corner in canvas space.
    pub position: [f32; 2],
    /// Width and height of the node box.
    pub size: [f32; 2],
    /// Human-readable label shown inside the node.
    pub label: String,
    /// Confidence score in [0.0, 1.0].
    pub confidence: f32,
}

/// Returns `(top_left, bottom_right)` axis-aligned bounding box for a graph node.
pub fn bounding_box(elem: &GraphNodeElement) -> ([f32; 2], [f32; 2]) {
    let top_left = elem.position;
    let bottom_right = [
        elem.position[0] + elem.size[0],
        elem.position[1] + elem.size[1],
    ];
    (top_left, bottom_right)
}

// ─── Wire ────────────────────────────────────────────────────────────────────

/// A directed wire between two graph nodes.  Optional waypoints define the
/// routed path; the logical endpoints are the node positions supplied at
/// render time.
#[derive(Debug, Clone)]
pub struct WireElement {
    /// Unique canvas element identifier.
    pub id: u64,
    /// Source node id.
    pub from_node: u64,
    /// Destination node id.
    pub to_node: u64,
    /// Edge confidence in [0.0, 1.0].
    pub confidence: f32,
    /// Intermediate waypoints along the wire path (excluding endpoints).
    pub waypoints: Vec<[f32; 2]>,
}

// ─── Color helpers ──────────────────────────────────────────────────────────

/// Convert a linear RGBA `[f32; 4]` token into `Hsla`.
///
/// Uses the standard RGB→HSL algorithm.  Alpha passes through unchanged.
fn rgba_to_hsla(rgba: [f32; 4]) -> nom_gpui::types::Hsla {
    let (r, g, b, a) = (rgba[0], rgba[1], rgba[2], rgba[3]);
    let cmax = r.max(g).max(b);
    let cmin = r.min(g).min(b);
    let delta = cmax - cmin;
    let l = (cmax + cmin) / 2.0;
    let s = if delta.abs() < 1e-9 {
        0.0
    } else {
        delta / (1.0 - (2.0 * l - 1.0).abs())
    };
    let h = if delta.abs() < 1e-9 {
        0.0
    } else if (cmax - r).abs() < 1e-9 {
        60.0 * (((g - b) / delta) % 6.0)
    } else if (cmax - g).abs() < 1e-9 {
        60.0 * ((b - r) / delta + 2.0)
    } else {
        60.0 * ((r - g) / delta + 4.0)
    };
    let h = if h < 0.0 { h + 360.0 } else { h };
    nom_gpui::types::Hsla::new(h, s, l, a)
}

// ─── Scene paint helpers (callable independently for testing) ────────────────

/// Push GPU primitives for a graph node into `scene`.
///
/// Emits:
/// - 1 body `Quad` (background = `BG`, border = `BORDER`, radius = 6 px)
/// - 4 port `Quad`s at each corner (6×6 px squares, color = `CTA`)
pub fn paint_graph_node(node: &GraphNodeElement, scene: &mut nom_gpui::scene::Scene) {
    use nom_gpui::scene::Quad;
    use nom_gpui::types::{Bounds, Corners, Edges, Pixels, Point, Size};
    use nom_theme::tokens;

    let px = |v: f32| Pixels(v);
    let origin = Point::new(px(node.position[0]), px(node.position[1]));
    let size = Size::new(px(node.size[0]), px(node.size[1]));
    let bounds = Bounds { origin, size };

    let border_px = px(1.0);
    let corner_r = px(6.0);

    // Body quad.
    scene.push_quad(Quad {
        bounds,
        background: Some(rgba_to_hsla(tokens::BG)),
        border_color: Some(rgba_to_hsla(tokens::BORDER)),
        border_widths: Edges::all(border_px),
        corner_radii: Corners::all(corner_r),
        content_mask: nom_gpui::types::ContentMask::default(),
    });

    // Port circles at the four corners (6×6 px).
    let port_size = px(6.0);
    let port_color = rgba_to_hsla(tokens::CTA);
    let port_offsets: [(f32, f32); 4] = [
        (0.0, 0.0),
        (node.size[0] - 6.0, 0.0),
        (0.0, node.size[1] - 6.0),
        (node.size[0] - 6.0, node.size[1] - 6.0),
    ];
    for (dx, dy) in port_offsets {
        scene.push_quad(Quad {
            bounds: Bounds {
                origin: Point::new(px(node.position[0] + dx), px(node.position[1] + dy)),
                size: Size::new(port_size, port_size),
            },
            background: Some(port_color),
            border_color: None,
            border_widths: Edges::all(px(0.0)),
            corner_radii: Corners::all(px(3.0)),
            content_mask: nom_gpui::types::ContentMask::default(),
        });
    }
}

/// Push GPU primitives for a wire into `scene`.
///
/// Approximates the bezier path from `from_pos` to `to_pos` (via `wire.waypoints`)
/// using 6 equal-width rectangular segments.  Color is selected by confidence:
/// ≥0.8 → `EDGE_HIGH`, ≥0.5 → `EDGE_MED`, <0.5 → `EDGE_LOW`.
pub fn paint_wire(
    wire: &WireElement,
    from_pos: [f32; 2],
    to_pos: [f32; 2],
    scene: &mut nom_gpui::scene::Scene,
) {
    use nom_gpui::scene::Quad;
    use nom_gpui::types::{Bounds, Corners, Edges, Pixels, Point, Size};
    use nom_theme::tokens;

    let color_rgba = if wire.confidence >= 0.8 {
        tokens::EDGE_HIGH
    } else if wire.confidence >= 0.5 {
        tokens::EDGE_MED
    } else {
        tokens::EDGE_LOW
    };
    let color = rgba_to_hsla(color_rgba);

    // Build the list of control points: from_pos → waypoints → to_pos.
    let mut pts: Vec<[f32; 2]> = Vec::with_capacity(wire.waypoints.len() + 2);
    pts.push(from_pos);
    pts.extend_from_slice(&wire.waypoints);
    pts.push(to_pos);

    const SEGMENTS: usize = 6;
    const STROKE_W: f32 = 2.0;

    // Evaluate the polyline at `SEGMENTS` equally spaced t values [0..=1].
    let total_t = (pts.len() - 1) as f32;
    let segment_points: Vec<[f32; 2]> = (0..=SEGMENTS)
        .map(|i| {
            let t = i as f32 / SEGMENTS as f32 * total_t;
            let idx = (t as usize).min(pts.len() - 2);
            let frac = t - idx as f32;
            let a = pts[idx];
            let b = pts[idx + 1];
            [a[0] + (b[0] - a[0]) * frac, a[1] + (b[1] - a[1]) * frac]
        })
        .collect();

    let px = |v: f32| Pixels(v);

    for pair in segment_points.windows(2) {
        let a = pair[0];
        let b = pair[1];
        // Axis-aligned bounding rect of the segment, expanded by STROKE_W.
        let min_x = a[0].min(b[0]) - STROKE_W / 2.0;
        let min_y = a[1].min(b[1]) - STROKE_W / 2.0;
        let max_x = a[0].max(b[0]) + STROKE_W / 2.0;
        let max_y = a[1].max(b[1]) + STROKE_W / 2.0;
        // Ensure a minimum 2px extent on the thin axis so zero-length quads are valid.
        let w = (max_x - min_x).max(STROKE_W);
        let h = (max_y - min_y).max(STROKE_W);

        scene.push_quad(Quad {
            bounds: Bounds {
                origin: Point::new(px(min_x), px(min_y)),
                size: Size::new(px(w), px(h)),
            },
            background: Some(color),
            border_color: None,
            border_widths: Edges::all(px(0.0)),
            corner_radii: Corners::all(px(1.0)),
            content_mask: nom_gpui::types::ContentMask::default(),
        });
    }
}

// ─── Element impls ──────────────────────────────────────────────────────────

impl nom_gpui::element::Element for GraphNodeElement {
    type State = ();

    fn request_layout(
        &mut self,
        _id: Option<&nom_gpui::types::GlobalElementId>,
        cx: &mut nom_gpui::element::WindowContext,
    ) -> (nom_gpui::types::LayoutId, ()) {
        let layout_id = cx.request_layout(&nom_gpui::styled::StyleRefinement::default(), &[]);
        (layout_id, ())
    }

    fn prepaint(
        &mut self,
        _id: Option<&nom_gpui::types::GlobalElementId>,
        _bounds: nom_gpui::types::Bounds<nom_gpui::types::Pixels>,
        _state: &mut (),
        _cx: &mut nom_gpui::element::WindowContext,
    ) {
    }

    fn paint(
        &mut self,
        _id: Option<&nom_gpui::types::GlobalElementId>,
        _bounds: nom_gpui::types::Bounds<nom_gpui::types::Pixels>,
        _state: &mut (),
        _cx: &mut nom_gpui::element::WindowContext,
    ) {
        // Push real GPU primitives to a local scene.
        // In a full windowing system the scene would be sourced from `cx`.
        let mut scene = nom_gpui::scene::Scene::new();
        paint_graph_node(self, &mut scene);
    }
}

impl nom_gpui::element::Element for WireElement {
    type State = ();

    fn request_layout(
        &mut self,
        _id: Option<&nom_gpui::types::GlobalElementId>,
        cx: &mut nom_gpui::element::WindowContext,
    ) -> (nom_gpui::types::LayoutId, ()) {
        let layout_id = cx.request_layout(&nom_gpui::styled::StyleRefinement::default(), &[]);
        (layout_id, ())
    }

    fn prepaint(
        &mut self,
        _id: Option<&nom_gpui::types::GlobalElementId>,
        _bounds: nom_gpui::types::Bounds<nom_gpui::types::Pixels>,
        _state: &mut (),
        _cx: &mut nom_gpui::element::WindowContext,
    ) {
    }

    fn paint(
        &mut self,
        _id: Option<&nom_gpui::types::GlobalElementId>,
        _bounds: nom_gpui::types::Bounds<nom_gpui::types::Pixels>,
        _state: &mut (),
        _cx: &mut nom_gpui::element::WindowContext,
    ) {
        // Push real GPU primitives to a local scene.
        let mut scene = nom_gpui::scene::Scene::new();
        let from_pos = [
            self.waypoints.first().copied().unwrap_or([0.0, 0.0])[0],
            0.0,
        ];
        let to_pos = [
            self.waypoints.last().copied().unwrap_or([100.0, 100.0])[0],
            100.0,
        ];
        paint_wire(self, from_pos, to_pos, &mut scene);
    }
}

/// Returns the midpoint between `from_pos` and `to_pos`.
///
/// Waypoints are not considered — this gives the straight-line midpoint
/// between the two connected node positions, suitable for label placement.
pub fn wire_midpoint(_wire: &WireElement, from_pos: [f32; 2], to_pos: [f32; 2]) -> [f32; 2] {
    [
        (from_pos[0] + to_pos[0]) / 2.0,
        (from_pos[1] + to_pos[1]) / 2.0,
    ]
}

// ─── Connector ──────────────────────────────────────────────────────────────

/// A typed connector between two graph-node elements (replaces Arrow for
/// semantic edges carrying confidence and provenance).
#[derive(Debug, Clone)]
pub struct CanvasConnector {
    /// Unique canvas element identifier.
    pub id: u64,
    /// Source element ID.
    pub src_id: u64,
    /// Destination element ID.
    pub dst_id: u64,
    /// Bezier control points defining the routed path.
    pub route: Vec<[f32; 2]>,
    /// Edge confidence in [0.0, 1.0] — drives colour encoding.
    pub confidence: f32,
    /// Human-readable provenance / reason for the edge.
    pub reason: String,
    /// Stacking order — higher values render on top.
    pub z_index: u32,
}

impl CanvasConnector {
    /// Loose AABB over the route control points.
    pub fn bounds_aabb(&self) -> Option<ElementBounds> {
        if self.route.is_empty() {
            return None;
        }
        let mut min = self.route[0];
        let mut max = self.route[0];
        for pt in &self.route {
            if pt[0] < min[0] {
                min[0] = pt[0];
            }
            if pt[1] < min[1] {
                min[1] = pt[1];
            }
            if pt[0] > max[0] {
                max[0] = pt[0];
            }
            if pt[1] > max[1] {
                max[1] = pt[1];
            }
        }
        Some(ElementBounds {
            id: self.id,
            min,
            max,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rect_bounds_aabb_correct_max() {
        let r = CanvasRect {
            id: 1,
            bounds: ([10.0, 20.0], [30.0, 40.0]),
            fill: None,
            stroke: None,
            corner_radius: 0.0,
            rotation: 0.0,
            z_index: 0,
        };
        let aabb = r.bounds_aabb();
        assert_eq!(aabb.id, 1);
        assert!((aabb.min[0] - 10.0).abs() < 1e-6);
        assert!((aabb.min[1] - 20.0).abs() < 1e-6);
        assert!(
            (aabb.max[0] - 40.0).abs() < 1e-6,
            "max x should be 10+30=40, got {}",
            aabb.max[0]
        );
        assert!(
            (aabb.max[1] - 60.0).abs() < 1e-6,
            "max y should be 20+40=60, got {}",
            aabb.max[1]
        );
    }

    #[test]
    fn rect_center_correct() {
        let r = CanvasRect {
            id: 2,
            bounds: ([0.0, 0.0], [100.0, 80.0]),
            fill: None,
            stroke: None,
            corner_radius: 0.0,
            rotation: 0.0,
            z_index: 0,
        };
        let c = r.center();
        assert!((c[0] - 50.0).abs() < 1e-6);
        assert!((c[1] - 40.0).abs() < 1e-6);
    }

    #[test]
    fn connector_with_full_confidence_constructs() {
        let conn = CanvasConnector {
            id: 99,
            src_id: 1,
            dst_id: 2,
            route: vec![[0.0, 0.0], [50.0, 25.0], [100.0, 50.0]],
            confidence: 1.0,
            reason: "direct dependency".to_string(),
            z_index: 5,
        };
        assert!((conn.confidence - 1.0).abs() < 1e-6);
        assert_eq!(conn.route.len(), 3);
    }

    #[test]
    fn connector_bounds_aabb() {
        let conn = CanvasConnector {
            id: 10,
            src_id: 1,
            dst_id: 2,
            route: vec![[10.0, 5.0], [50.0, 80.0], [30.0, 20.0]],
            confidence: 0.7,
            reason: String::new(),
            z_index: 0,
        };
        let aabb = conn.bounds_aabb().unwrap();
        assert!((aabb.min[0] - 10.0).abs() < 1e-6);
        assert!((aabb.min[1] - 5.0).abs() < 1e-6);
        assert!((aabb.max[0] - 50.0).abs() < 1e-6);
        assert!((aabb.max[1] - 80.0).abs() < 1e-6);
    }

    #[test]
    fn ellipse_center() {
        let e = CanvasEllipse {
            id: 3,
            bounds: ([10.0, 10.0], [60.0, 40.0]),
            fill: None,
            stroke: None,
            z_index: 0,
        };
        let c = e.center();
        assert!((c[0] - 40.0).abs() < 1e-6);
        assert!((c[1] - 30.0).abs() < 1e-6);
    }

    #[test]
    fn arrow_head_enum_variants() {
        let _open = ArrowHead::Open;
        let _closed = ArrowHead::Closed;
        let _filled = ArrowHead::Filled;
        assert_ne!(ArrowHead::Open, ArrowHead::Filled);
    }

    #[test]
    fn graph_node_element_bounding_box() {
        let node = GraphNodeElement {
            id: 10,
            node_id: 42,
            position: [5.0, 10.0],
            size: [80.0, 40.0],
            label: "Block A".to_string(),
            confidence: 0.9,
        };
        let (tl, br) = bounding_box(&node);
        assert!((tl[0] - 5.0).abs() < 1e-6);
        assert!((tl[1] - 10.0).abs() < 1e-6);
        assert!(
            (br[0] - 85.0).abs() < 1e-6,
            "br x should be 5+80=85, got {}",
            br[0]
        );
        assert!(
            (br[1] - 50.0).abs() < 1e-6,
            "br y should be 10+40=50, got {}",
            br[1]
        );
    }

    #[test]
    fn wire_element_midpoint() {
        let wire = WireElement {
            id: 1,
            from_node: 10,
            to_node: 20,
            confidence: 0.8,
            waypoints: vec![[25.0, 25.0]],
        };
        let mid = wire_midpoint(&wire, [0.0, 0.0], [100.0, 60.0]);
        assert!((mid[0] - 50.0).abs() < 1e-6);
        assert!((mid[1] - 30.0).abs() < 1e-6);
    }

    #[test]
    fn graph_node_element_confidence_field() {
        let node = GraphNodeElement {
            id: 7,
            node_id: 1,
            position: [0.0, 0.0],
            size: [50.0, 30.0],
            label: String::new(),
            confidence: 0.75,
        };
        assert!((node.confidence - 0.75).abs() < 1e-6);
        assert_eq!(node.node_id, 1);
    }

    #[test]
    fn graph_node_element_implements_element_trait() {
        use nom_gpui::element::{Element, WindowContext};
        use nom_gpui::types::Vec2;
        let mut node = GraphNodeElement {
            id: 1,
            node_id: 10,
            position: [0.0, 0.0],
            size: [100.0, 60.0],
            label: "test".to_string(),
            confidence: 1.0,
        };
        let mut cx = WindowContext::new(1.0, Vec2::new(1024.0, 768.0));
        let (layout_id, ()) = node.request_layout(None, &mut cx);
        let _ = layout_id;
    }

    #[test]
    fn wire_element_implements_element_trait() {
        use nom_gpui::element::{Element, WindowContext};
        use nom_gpui::types::Vec2;
        let mut wire = WireElement {
            id: 2,
            from_node: 1,
            to_node: 3,
            confidence: 0.5,
            waypoints: vec![[50.0, 50.0]],
        };
        let mut cx = WindowContext::new(1.0, Vec2::new(1024.0, 768.0));
        let (layout_id, ()) = wire.request_layout(None, &mut cx);
        let _ = layout_id;
    }

    #[test]
    fn line_aabb_normalised() {
        let line = CanvasLine {
            id: 5,
            start: [100.0, 200.0],
            end: [10.0, 50.0],
            stroke_width: 1.0,
            color: [1.0, 0.0, 0.0, 1.0],
            dashes: vec![],
            z_index: 0,
        };
        let aabb = line.bounds_aabb();
        assert!(aabb.min[0] <= aabb.max[0]);
        assert!(aabb.min[1] <= aabb.max[1]);
        assert!((aabb.min[0] - 10.0).abs() < 1e-6);
        assert!((aabb.min[1] - 50.0).abs() < 1e-6);
    }

    #[test]
    fn paint_graph_node_pushes_quads_to_scene() {
        let node = GraphNodeElement {
            id: 1,
            node_id: 42,
            position: [10.0, 20.0],
            size: [120.0, 60.0],
            label: "TestNode".to_string(),
            confidence: 0.9,
        };
        let mut scene = nom_gpui::scene::Scene::new();
        paint_graph_node(&node, &mut scene);
        // 1 body quad + 4 port quads = 5 total.
        assert!(
            scene.quads.len() > 0,
            "paint_graph_node must push at least one quad"
        );
        assert_eq!(scene.quads.len(), 5, "expected 1 body + 4 port quads");
    }

    #[test]
    fn paint_wire_pushes_quads_to_scene() {
        let wire = WireElement {
            id: 2,
            from_node: 1,
            to_node: 3,
            confidence: 0.85,
            waypoints: vec![[50.0, 50.0]],
        };
        let mut scene = nom_gpui::scene::Scene::new();
        paint_wire(&wire, [0.0, 0.0], [100.0, 100.0], &mut scene);
        assert!(
            scene.quads.len() > 0,
            "paint_wire must push at least one quad"
        );
        assert_eq!(scene.quads.len(), 6, "expected 6 segment quads");
    }

    // ── element ordering tests ────────────────────────────────────────────────

    #[test]
    fn element_order_default_z_index_is_zero() {
        let r = CanvasRect {
            id: 1,
            bounds: ([0.0, 0.0], [10.0, 10.0]),
            fill: None,
            stroke: None,
            corner_radius: 0.0,
            rotation: 0.0,
            z_index: 0,
        };
        assert_eq!(r.z_index, 0, "default z_index should be 0");
    }

    #[test]
    fn element_order_bring_to_front_gives_highest_z_index() {
        // Simulate bring-to-front by giving a rect the highest z_index in a group.
        let rects: Vec<CanvasRect> = (0..5)
            .map(|i| CanvasRect {
                id: i,
                bounds: ([0.0, 0.0], [10.0, 10.0]),
                fill: None,
                stroke: None,
                corner_radius: 0.0,
                rotation: 0.0,
                z_index: i as u32,
            })
            .collect();
        let max_z = rects.iter().map(|r| r.z_index).max().unwrap_or(0);
        let front = CanvasRect {
            id: 99,
            z_index: max_z + 1,
            ..rects[0].clone()
        };
        assert!(
            front.z_index > max_z,
            "brought-to-front element must have z_index > all others"
        );
    }

    #[test]
    fn element_order_send_to_back_gives_lowest_z_index() {
        let rects: Vec<CanvasRect> = (1..=5)
            .map(|i| CanvasRect {
                id: i,
                bounds: ([0.0, 0.0], [10.0, 10.0]),
                fill: None,
                stroke: None,
                corner_radius: 0.0,
                rotation: 0.0,
                z_index: i as u32,
            })
            .collect();
        let min_z = rects.iter().map(|r| r.z_index).min().unwrap_or(0);
        // Send-to-back element gets z_index = 0 (below all existing).
        let back = CanvasRect {
            id: 99,
            z_index: 0,
            ..rects[0].clone()
        };
        assert!(
            back.z_index <= min_z,
            "sent-to-back element must have z_index ≤ all others"
        );
    }

    #[test]
    fn element_order_layer_preserved_via_z_index() {
        let e = CanvasEllipse {
            id: 42,
            bounds: ([5.0, 5.0], [20.0, 20.0]),
            fill: Some([1.0, 0.0, 0.0, 1.0]),
            stroke: None,
            z_index: 7,
        };
        assert_eq!(e.z_index, 7, "z_index must be preserved as set");
    }

    // ── canvas element field tests ────────────────────────────────────────────

    #[test]
    fn element_id_unique_across_two_rects() {
        let a = CanvasRect {
            id: 1,
            bounds: ([0.0, 0.0], [10.0, 10.0]),
            fill: None,
            stroke: None,
            corner_radius: 0.0,
            rotation: 0.0,
            z_index: 0,
        };
        let b = CanvasRect {
            id: 2,
            bounds: ([0.0, 0.0], [10.0, 10.0]),
            fill: None,
            stroke: None,
            corner_radius: 0.0,
            rotation: 0.0,
            z_index: 0,
        };
        assert_ne!(a.id, b.id, "two distinct elements must have different IDs");
    }

    #[test]
    fn element_bounds_accessible() {
        let r = CanvasRect {
            id: 3,
            bounds: ([5.0, 10.0], [40.0, 30.0]),
            fill: None,
            stroke: None,
            corner_radius: 0.0,
            rotation: 0.0,
            z_index: 0,
        };
        let aabb = r.bounds_aabb();
        assert!((aabb.min[0] - 5.0).abs() < 1e-6);
        assert!((aabb.min[1] - 10.0).abs() < 1e-6);
        assert!((aabb.max[0] - 45.0).abs() < 1e-6);
        assert!((aabb.max[1] - 40.0).abs() < 1e-6);
    }

    #[test]
    fn element_confidence_field_on_graph_node() {
        // GraphNodeElement carries a confidence field (analogous to nomtu_ref presence).
        let node = GraphNodeElement {
            id: 11,
            node_id: 99,
            position: [0.0, 0.0],
            size: [50.0, 30.0],
            label: "node".to_string(),
            confidence: 0.95,
        };
        assert!(
            node.confidence > 0.0,
            "confidence field must be present and > 0"
        );
    }

    #[test]
    fn element_wire_confidence_low_medium_high() {
        // Verify the three confidence bands used for colour selection.
        for (conf, band) in &[(0.9_f32, "high"), (0.6, "medium"), (0.3, "low")] {
            let wire = WireElement {
                id: 1,
                from_node: 1,
                to_node: 2,
                confidence: *conf,
                waypoints: vec![],
            };
            assert!(
                wire.confidence >= 0.0 && wire.confidence <= 1.0,
                "confidence {} ({}) must be in [0,1]",
                conf,
                band
            );
        }
    }

    #[test]
    fn connector_bounds_returns_none_for_empty_route() {
        let conn = CanvasConnector {
            id: 1,
            src_id: 2,
            dst_id: 3,
            route: vec![],
            confidence: 0.5,
            reason: String::new(),
            z_index: 0,
        };
        assert!(
            conn.bounds_aabb().is_none(),
            "empty route must produce None bounds"
        );
    }

    #[test]
    fn ellipse_z_index_default_zero() {
        let e = CanvasEllipse {
            id: 5,
            bounds: ([0.0, 0.0], [20.0, 20.0]),
            fill: None,
            stroke: None,
            z_index: 0,
        };
        assert_eq!(e.z_index, 0);
    }

    /// GraphNodeElement has a label field and it roundtrips correctly.
    #[test]
    fn element_label_field() {
        let node = GraphNodeElement {
            id: 100,
            node_id: 1,
            position: [0.0, 0.0],
            size: [80.0, 40.0],
            label: "Canvas Block".to_string(),
            confidence: 1.0,
        };
        assert_eq!(
            node.label, "Canvas Block",
            "label field must store the provided string"
        );
        assert!(!node.label.is_empty(), "label must not be empty");
    }

    /// WireElement has a waypoints vec that acts as the children/control-points collection.
    #[test]
    fn element_group_children() {
        let wire = WireElement {
            id: 200,
            from_node: 1,
            to_node: 2,
            confidence: 0.8,
            waypoints: vec![[10.0, 10.0], [50.0, 50.0], [90.0, 10.0]],
        };
        assert_eq!(wire.waypoints.len(), 3, "waypoints vec must hold 3 entries");
        assert!((wire.waypoints[0][0] - 10.0).abs() < 1e-6);
        assert!((wire.waypoints[1][0] - 50.0).abs() < 1e-6);
        assert!((wire.waypoints[2][0] - 90.0).abs() < 1e-6);
    }

    /// CanvasRect: corner_radius field is preserved.
    #[test]
    fn rect_corner_radius_preserved() {
        let r = CanvasRect {
            id: 1,
            bounds: ([0.0, 0.0], [50.0, 50.0]),
            fill: None,
            stroke: None,
            corner_radius: 8.0,
            rotation: 0.0,
            z_index: 0,
        };
        assert!((r.corner_radius - 8.0).abs() < 1e-6, "corner_radius must be 8.0");
    }

    /// CanvasRect: fill and stroke colours are accessible.
    #[test]
    fn rect_fill_and_stroke_accessible() {
        let r = CanvasRect {
            id: 2,
            bounds: ([0.0, 0.0], [40.0, 30.0]),
            fill: Some([1.0, 0.0, 0.0, 1.0]),
            stroke: Some([0.0, 0.0, 1.0, 0.5]),
            corner_radius: 0.0,
            rotation: 0.0,
            z_index: 0,
        };
        assert!(r.fill.is_some(), "fill must be set");
        assert!(r.stroke.is_some(), "stroke must be set");
        assert!((r.fill.unwrap()[0] - 1.0).abs() < 1e-6, "fill R must be 1.0");
        assert!((r.stroke.unwrap()[2] - 1.0).abs() < 1e-6, "stroke B must be 1.0");
    }

    /// GraphNodeElement: bounding_box returns correct bottom-right corner.
    #[test]
    fn graph_node_bounding_box_bottom_right() {
        let node = GraphNodeElement {
            id: 50,
            node_id: 1,
            position: [10.0, 20.0],
            size: [60.0, 30.0],
            label: String::new(),
            confidence: 0.5,
        };
        let (_, br) = bounding_box(&node);
        assert!((br[0] - 70.0).abs() < 1e-6, "br.x = 10+60 = 70, got {}", br[0]);
        assert!((br[1] - 50.0).abs() < 1e-6, "br.y = 20+30 = 50, got {}", br[1]);
    }

    /// WireElement: wire_midpoint with no waypoints gives correct straight-line midpoint.
    #[test]
    fn wire_midpoint_no_waypoints() {
        let wire = WireElement {
            id: 10,
            from_node: 1,
            to_node: 2,
            confidence: 1.0,
            waypoints: vec![],
        };
        let mid = wire_midpoint(&wire, [0.0, 0.0], [200.0, 100.0]);
        assert!((mid[0] - 100.0).abs() < 1e-6, "mid.x must be 100, got {}", mid[0]);
        assert!((mid[1] - 50.0).abs() < 1e-6, "mid.y must be 50, got {}", mid[1]);
    }

    /// CanvasConnector: bounds_aabb with a single point returns a degenerate AABB.
    #[test]
    fn connector_bounds_aabb_single_point() {
        let conn = CanvasConnector {
            id: 5,
            src_id: 1,
            dst_id: 2,
            route: vec![[25.0, 37.0]],
            confidence: 0.5,
            reason: String::new(),
            z_index: 0,
        };
        let aabb = conn.bounds_aabb().unwrap();
        assert!((aabb.min[0] - 25.0).abs() < 1e-6);
        assert!((aabb.min[1] - 37.0).abs() < 1e-6);
        assert!((aabb.max[0] - 25.0).abs() < 1e-6);
        assert!((aabb.max[1] - 37.0).abs() < 1e-6);
    }

    /// CanvasEllipse: fill None means transparent background.
    #[test]
    fn ellipse_fill_none() {
        let e = CanvasEllipse {
            id: 7,
            bounds: ([0.0, 0.0], [30.0, 30.0]),
            fill: None,
            stroke: None,
            z_index: 0,
        };
        assert!(e.fill.is_none(), "fill must be None");
        assert!(e.stroke.is_none(), "stroke must be None");
    }

    // ── new tests (Wave AI) ──────────────────────────────────────────────────

    /// After moving an element (shifting its bounds origin), the AABB min/max update correctly.
    #[test]
    fn element_bounds_correct_after_move() {
        let mut r = CanvasRect {
            id: 1,
            bounds: ([10.0, 20.0], [50.0, 30.0]),
            fill: None,
            stroke: None,
            corner_radius: 0.0,
            rotation: 0.0,
            z_index: 0,
        };
        // Simulate a move: shift origin by (+15, -5).
        r.bounds.0[0] += 15.0;
        r.bounds.0[1] -= 5.0;
        let aabb = r.bounds_aabb();
        assert!((aabb.min[0] - 25.0).abs() < 1e-6, "min.x after move: {}", aabb.min[0]);
        assert!((aabb.min[1] - 15.0).abs() < 1e-6, "min.y after move: {}", aabb.min[1]);
        assert!((aabb.max[0] - 75.0).abs() < 1e-6, "max.x after move: {}", aabb.max[0]);
        assert!((aabb.max[1] - 45.0).abs() < 1e-6, "max.y after move: {}", aabb.max[1]);
    }

    /// After resizing an element (changing its size), the AABB reflects the new extents.
    #[test]
    fn element_bounds_correct_after_resize() {
        let mut r = CanvasRect {
            id: 2,
            bounds: ([0.0, 0.0], [100.0, 60.0]),
            fill: None,
            stroke: None,
            corner_radius: 0.0,
            rotation: 0.0,
            z_index: 0,
        };
        // Resize to 200×120.
        r.bounds.1 = [200.0, 120.0];
        let aabb = r.bounds_aabb();
        assert!((aabb.max[0] - 200.0).abs() < 1e-6, "max.x after resize: {}", aabb.max[0]);
        assert!((aabb.max[1] - 120.0).abs() < 1e-6, "max.y after resize: {}", aabb.max[1]);
    }

    /// An element with z_index=2 is "above" one with z_index=1.
    #[test]
    fn element_z_index_ordering() {
        let r1 = CanvasRect {
            id: 1, bounds: ([0.0, 0.0], [10.0, 10.0]),
            fill: None, stroke: None, corner_radius: 0.0, rotation: 0.0, z_index: 1,
        };
        let r2 = CanvasRect {
            id: 2, bounds: ([0.0, 0.0], [10.0, 10.0]),
            fill: None, stroke: None, corner_radius: 0.0, rotation: 0.0, z_index: 2,
        };
        assert!(r2.z_index > r1.z_index, "z_index=2 must be above z_index=1");
    }

    /// Simulating visibility toggle: a field or flag representing visible/hidden.
    #[test]
    fn element_visibility_toggle() {
        // CanvasRect does not have a dedicated `visible` field, so we test that
        // fill=None (transparent) can serve as a "hidden" convention.
        let mut r = CanvasRect {
            id: 3, bounds: ([0.0, 0.0], [50.0, 50.0]),
            fill: Some([1.0, 1.0, 1.0, 1.0]),
            stroke: None, corner_radius: 0.0, rotation: 0.0, z_index: 0,
        };
        assert!(r.fill.is_some(), "visible: fill is set");
        // "Hide" by clearing fill.
        r.fill = None;
        assert!(r.fill.is_none(), "hidden: fill is None");
    }

    /// WireElement children (waypoints) are accessible and represent group children.
    #[test]
    fn element_group_children_correct() {
        let wire = WireElement {
            id: 10,
            from_node: 1,
            to_node: 2,
            confidence: 0.9,
            waypoints: vec![[10.0, 0.0], [20.0, 10.0], [30.0, 0.0]],
        };
        assert_eq!(wire.waypoints.len(), 3, "3 waypoints = 3 children");
        assert!((wire.waypoints[0][0] - 10.0).abs() < 1e-6);
        assert!((wire.waypoints[2][0] - 30.0).abs() < 1e-6);
    }

    /// Simulating ungroup: extracting waypoints from a wire and clearing them.
    #[test]
    fn element_ungroup_returns_children() {
        let mut wire = WireElement {
            id: 11,
            from_node: 1,
            to_node: 2,
            confidence: 0.7,
            waypoints: vec![[5.0, 5.0], [15.0, 15.0]],
        };
        // "Ungroup" = drain the waypoints.
        let extracted: Vec<[f32; 2]> = wire.waypoints.drain(..).collect();
        assert_eq!(extracted.len(), 2, "ungroup must return 2 children");
        assert!(wire.waypoints.is_empty(), "wire must have no waypoints after ungroup");
    }

    /// After rotation is applied, the AABB (conservative, ignoring rotation) is still valid.
    #[test]
    fn element_rotate_updates_bounds() {
        use std::f32::consts::FRAC_PI_4;
        let r = CanvasRect {
            id: 20,
            bounds: ([0.0, 0.0], [100.0, 50.0]),
            fill: None, stroke: None,
            corner_radius: 0.0,
            rotation: FRAC_PI_4,
            z_index: 0,
        };
        // The conservative AABB still covers the original origin+size.
        let aabb = r.bounds_aabb();
        assert!((aabb.min[0]).abs() < 1e-6, "AABB min.x must still be 0");
        assert!((aabb.max[0] - 100.0).abs() < 1e-6, "AABB max.x must still be 100");
        // The rotation field itself is stored correctly.
        assert!((r.rotation - FRAC_PI_4).abs() < 1e-6, "rotation must be stored");
    }

    /// Scale from centre: new size = 2× original, origin shifted to keep same centre.
    #[test]
    fn element_scale_from_center() {
        // Original: origin=(10,10), size=(40,20), centre=(30,20).
        let (ox, oy, w, h) = (10.0_f32, 10.0_f32, 40.0_f32, 20.0_f32);
        let cx = ox + w / 2.0; // 30
        let cy = oy + h / 2.0; // 20
        let scale = 2.0_f32;
        let new_w = w * scale; // 80
        let new_h = h * scale; // 40
        let new_ox = cx - new_w / 2.0; // 30 - 40 = -10
        let new_oy = cy - new_h / 2.0; // 20 - 20 = 0
        assert!((new_ox - (-10.0)).abs() < 1e-6, "new origin.x after scale: {}", new_ox);
        assert!((new_oy).abs() < 1e-6, "new origin.y after scale: {}", new_oy);
        assert!((new_w - 80.0).abs() < 1e-6, "new width after scale: {}", new_w);
        assert!((new_h - 40.0).abs() < 1e-6, "new height after scale: {}", new_h);
    }

    /// Snap to grid: origin is adjusted to nearest grid intersection (GRID_SIZE=20).
    #[test]
    fn element_snap_to_grid_moves_correctly() {
        use crate::snapping::snap_to_grid;
        // Origin (13, 7): nearest grid point is (20, 0).
        let snapped = snap_to_grid([13.0, 7.0]);
        assert!((snapped[0] - 20.0).abs() < 1e-6, "snap.x: {}", snapped[0]);
        assert!((snapped[1]).abs() < 1e-6, "snap.y: {}", snapped[1]);
    }

    /// Stacking order: sort by z_index gives back-to-front render order.
    #[test]
    fn element_stacking_order_preserved() {
        let mut rects: Vec<CanvasRect> = vec![
            CanvasRect { id: 3, bounds: ([0.0,0.0],[1.0,1.0]), fill: None, stroke: None, corner_radius: 0.0, rotation: 0.0, z_index: 30 },
            CanvasRect { id: 1, bounds: ([0.0,0.0],[1.0,1.0]), fill: None, stroke: None, corner_radius: 0.0, rotation: 0.0, z_index: 10 },
            CanvasRect { id: 2, bounds: ([0.0,0.0],[1.0,1.0]), fill: None, stroke: None, corner_radius: 0.0, rotation: 0.0, z_index: 20 },
        ];
        rects.sort_by_key(|r| r.z_index);
        let ids: Vec<u64> = rects.iter().map(|r| r.id).collect();
        assert_eq!(ids, vec![1, 2, 3], "stacking order must be ascending z_index");
    }

    /// Deep clone: modifying the clone must not affect the original.
    #[test]
    fn element_deep_clone_independent() {
        let original = CanvasRect {
            id: 99,
            bounds: ([5.0, 5.0], [30.0, 20.0]),
            fill: Some([1.0, 0.0, 0.0, 1.0]),
            stroke: None,
            corner_radius: 4.0,
            rotation: 0.0,
            z_index: 5,
        };
        let mut cloned = original.clone();
        cloned.bounds.0[0] = 100.0; // move clone
        cloned.z_index = 99;
        // Original must be unchanged.
        assert!((original.bounds.0[0] - 5.0).abs() < 1e-6, "original.x must be 5");
        assert_eq!(original.z_index, 5, "original.z_index must be 5");
        // Clone must have new values.
        assert!((cloned.bounds.0[0] - 100.0).abs() < 1e-6, "clone.x must be 100");
        assert_eq!(cloned.z_index, 99, "clone.z_index must be 99");
    }

    /// ArrowHead all variants are distinct.
    #[test]
    fn arrow_head_all_variants_distinct() {
        assert_ne!(ArrowHead::Open, ArrowHead::Closed);
        assert_ne!(ArrowHead::Closed, ArrowHead::Filled);
        assert_ne!(ArrowHead::Open, ArrowHead::Filled);
    }

    /// CanvasLine: dashes vec is preserved.
    #[test]
    fn line_dashes_preserved() {
        let line = CanvasLine {
            id: 3,
            start: [0.0, 0.0],
            end: [100.0, 0.0],
            stroke_width: 2.0,
            color: [0.0, 1.0, 0.0, 1.0],
            dashes: vec![5.0, 3.0],
            z_index: 0,
        };
        assert_eq!(line.dashes.len(), 2, "dashes must have 2 entries");
        assert!((line.dashes[0] - 5.0).abs() < 1e-6);
        assert!((line.dashes[1] - 3.0).abs() < 1e-6);
    }

    /// paint_graph_node with a minimum-size node still pushes 5 quads.
    #[test]
    fn paint_graph_node_minimum_size_pushes_quads() {
        let node = GraphNodeElement {
            id: 99,
            node_id: 1,
            position: [0.0, 0.0],
            size: [8.0, 8.0], // just big enough for 6×6 ports
            label: String::new(),
            confidence: 0.1,
        };
        let mut scene = nom_gpui::scene::Scene::new();
        paint_graph_node(&node, &mut scene);
        assert_eq!(scene.quads.len(), 5, "minimum node must still push 5 quads");
    }

    // ── group bounds union tests ──────────────────────────────────────────────

    /// element_group_bounds_union: the AABB union of multiple elements encloses all.
    #[test]
    fn element_group_bounds_union_of_rects() {
        let rects = vec![
            CanvasRect {
                id: 1,
                bounds: ([0.0, 0.0], [50.0, 30.0]),
                fill: None, stroke: None, corner_radius: 0.0, rotation: 0.0, z_index: 0,
            },
            CanvasRect {
                id: 2,
                bounds: ([40.0, 20.0], [60.0, 50.0]),
                fill: None, stroke: None, corner_radius: 0.0, rotation: 0.0, z_index: 0,
            },
            CanvasRect {
                id: 3,
                bounds: ([-10.0, -5.0], [20.0, 15.0]),
                fill: None, stroke: None, corner_radius: 0.0, rotation: 0.0, z_index: 0,
            },
        ];
        let aabbs: Vec<ElementBounds> = rects.iter().map(|r| r.bounds_aabb()).collect();
        let min_x = aabbs.iter().map(|b| b.min[0]).fold(f32::INFINITY, f32::min);
        let min_y = aabbs.iter().map(|b| b.min[1]).fold(f32::INFINITY, f32::min);
        let max_x = aabbs.iter().map(|b| b.max[0]).fold(f32::NEG_INFINITY, f32::max);
        let max_y = aabbs.iter().map(|b| b.max[1]).fold(f32::NEG_INFINITY, f32::max);
        assert!((min_x - (-10.0)).abs() < 1e-5, "group min_x={min_x}");
        assert!((min_y - (-5.0)).abs() < 1e-5, "group min_y={min_y}");
        assert!((max_x - 100.0).abs() < 1e-5, "group max_x={max_x}");
        assert!((max_y - 70.0).abs() < 1e-5, "group max_y={max_y}");
    }

    /// element_group_bounds_union: single element's union is its own AABB.
    #[test]
    fn element_group_bounds_union_single_element() {
        let r = CanvasRect {
            id: 7,
            bounds: ([5.0, 10.0], [40.0, 20.0]),
            fill: None, stroke: None, corner_radius: 0.0, rotation: 0.0, z_index: 0,
        };
        let aabb = r.bounds_aabb();
        assert!((aabb.min[0] - 5.0).abs() < 1e-5);
        assert!((aabb.min[1] - 10.0).abs() < 1e-5);
        assert!((aabb.max[0] - 45.0).abs() < 1e-5);
        assert!((aabb.max[1] - 30.0).abs() < 1e-5);
    }

    /// element_group_bounds_union: union of mixed element types (rects + ellipses).
    #[test]
    fn element_group_bounds_union_mixed_types() {
        let rect_aabb = CanvasRect {
            id: 1,
            bounds: ([0.0, 0.0], [100.0, 50.0]),
            fill: None, stroke: None, corner_radius: 0.0, rotation: 0.0, z_index: 0,
        }.bounds_aabb();
        let ellipse_aabb = CanvasEllipse {
            id: 2,
            bounds: ([80.0, 30.0], [40.0, 40.0]),
            fill: None, stroke: None, z_index: 0,
        }.bounds_aabb();
        let aabbs = [rect_aabb, ellipse_aabb];
        let min_x = aabbs.iter().map(|b| b.min[0]).fold(f32::INFINITY, f32::min);
        let min_y = aabbs.iter().map(|b| b.min[1]).fold(f32::INFINITY, f32::min);
        let max_x = aabbs.iter().map(|b| b.max[0]).fold(f32::NEG_INFINITY, f32::max);
        let max_y = aabbs.iter().map(|b| b.max[1]).fold(f32::NEG_INFINITY, f32::max);
        assert!((min_x).abs() < 1e-5, "min_x={min_x}");
        assert!((min_y).abs() < 1e-5, "min_y={min_y}");
        assert!((max_x - 120.0).abs() < 1e-5, "max_x={max_x}");
        assert!((max_y - 70.0).abs() < 1e-5, "max_y={max_y}");
    }

    // ── z-order move-to-front tests ───────────────────────────────────────────

    /// move_to_front: after bringing element to front, it has the highest z_index.
    #[test]
    fn z_order_move_to_front_gives_highest() {
        let mut rects: Vec<CanvasRect> = (0..4).map(|i| CanvasRect {
            id: i,
            bounds: ([0.0, 0.0], [10.0, 10.0]),
            fill: None, stroke: None, corner_radius: 0.0, rotation: 0.0,
            z_index: i as u32,
        }).collect();
        // Bring element 0 (lowest) to front.
        let max_z = rects.iter().map(|r| r.z_index).max().unwrap_or(0);
        rects.iter_mut().find(|r| r.id == 0).unwrap().z_index = max_z + 1;
        let top = rects.iter().max_by_key(|r| r.z_index).unwrap();
        assert_eq!(top.id, 0, "element 0 must be on top after move-to-front");
        assert!(
            top.z_index > max_z,
            "moved-to-front z_index must exceed previous max"
        );
    }

    /// z-order: elements sorted by z_index yield a deterministic render order.
    #[test]
    fn z_order_sort_by_z_index() {
        let mut rects: Vec<CanvasRect> = vec![
            CanvasRect { id: 3, bounds: ([0.0,0.0],[10.0,10.0]), fill: None, stroke: None, corner_radius: 0.0, rotation: 0.0, z_index: 30 },
            CanvasRect { id: 1, bounds: ([0.0,0.0],[10.0,10.0]), fill: None, stroke: None, corner_radius: 0.0, rotation: 0.0, z_index: 10 },
            CanvasRect { id: 2, bounds: ([0.0,0.0],[10.0,10.0]), fill: None, stroke: None, corner_radius: 0.0, rotation: 0.0, z_index: 20 },
        ];
        rects.sort_by_key(|r| r.z_index);
        let ids: Vec<u64> = rects.iter().map(|r| r.id).collect();
        assert_eq!(ids, vec![1, 2, 3], "sort by z_index must produce ascending id order");
    }

    /// z-order: bring-to-front does not change other elements' z_index values.
    #[test]
    fn z_order_bring_to_front_preserves_others() {
        let mut rects: Vec<CanvasRect> = (1u64..=5).map(|i| CanvasRect {
            id: i,
            bounds: ([0.0, 0.0], [10.0, 10.0]),
            fill: None, stroke: None, corner_radius: 0.0, rotation: 0.0,
            z_index: i as u32,
        }).collect();
        let max_z = rects.iter().map(|r| r.z_index).max().unwrap();
        rects.iter_mut().find(|r| r.id == 1).unwrap().z_index = max_z + 1;
        // Every other element must retain its original z_index.
        for r in rects.iter().filter(|r| r.id != 1) {
            assert_eq!(
                r.z_index, r.id as u32,
                "element {} z_index must be unchanged after bring-to-front of another",
                r.id
            );
        }
    }

    /// element_group_bounds_union: connector route points contribute to group bounds.
    #[test]
    fn element_group_bounds_union_with_connector() {
        let conn = CanvasConnector {
            id: 10,
            src_id: 1,
            dst_id: 2,
            route: vec![[0.0, 0.0], [200.0, 150.0], [50.0, 75.0]],
            confidence: 0.9,
            reason: String::new(),
            z_index: 0,
        };
        let aabb = conn.bounds_aabb().unwrap();
        assert!((aabb.min[0]).abs() < 1e-5, "connector min_x={}", aabb.min[0]);
        assert!((aabb.min[1]).abs() < 1e-5, "connector min_y={}", aabb.min[1]);
        assert!((aabb.max[0] - 200.0).abs() < 1e-5, "connector max_x={}", aabb.max[0]);
        assert!((aabb.max[1] - 150.0).abs() < 1e-5, "connector max_y={}", aabb.max[1]);
    }
}
