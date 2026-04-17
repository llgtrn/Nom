/// Canvas element primitives.
///
/// All coordinates are in canvas-space (f32).  Colours are RGBA in linear [0,1].

/// Arrow-head styles for `CanvasArrow`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ArrowHead {
    Open,
    Closed,
    Filled,
}

/// Axis-aligned bounding box envelope used by the spatial index.
#[derive(Debug, Clone, Copy)]
pub struct ElementBounds {
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
    pub id: u64,
    /// `(origin, size)` — origin is the top-left corner before rotation.
    pub bounds: ([f32; 2], [f32; 2]),
    /// Optional fill colour (RGBA).
    pub fill: Option<[f32; 4]>,
    /// Optional stroke colour (RGBA).
    pub stroke: Option<[f32; 4]>,
    pub corner_radius: f32,
    /// Rotation in radians (counter-clockwise, applied around centre).
    pub rotation: f32,
    pub z_index: u32,
}

impl CanvasRect {
    /// Axis-aligned bounding box (ignores rotation — conservative broadphase).
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
    pub id: u64,
    /// `(origin, size)` — bounding rectangle of the ellipse.
    pub bounds: ([f32; 2], [f32; 2]),
    pub fill: Option<[f32; 4]>,
    pub stroke: Option<[f32; 4]>,
    pub z_index: u32,
}

impl CanvasEllipse {
    pub fn bounds_aabb(&self) -> ElementBounds {
        let (origin, size) = self.bounds;
        ElementBounds {
            id: self.id,
            min: origin,
            max: [origin[0] + size[0], origin[1] + size[1]],
        }
    }

    pub fn center(&self) -> [f32; 2] {
        let (origin, size) = self.bounds;
        [origin[0] + size[0] / 2.0, origin[1] + size[1] / 2.0]
    }
}

// ─── Line ───────────────────────────────────────────────────────────────────

/// A straight line segment.
#[derive(Debug, Clone)]
pub struct CanvasLine {
    pub id: u64,
    pub start: [f32; 2],
    pub end: [f32; 2],
    pub stroke_width: f32,
    pub color: [f32; 4],
    /// Dash pattern lengths in canvas units.  Empty = solid line.
    pub dashes: Vec<f32>,
    pub z_index: u32,
}

impl CanvasLine {
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
    pub id: u64,
    pub start: [f32; 2],
    pub end: [f32; 2],
    pub stroke_width: f32,
    pub color: [f32; 4],
    pub head_style: ArrowHead,
    pub z_index: u32,
}

impl CanvasArrow {
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

/// Returns the midpoint between `from_pos` and `to_pos`.
///
/// Waypoints are not considered — this gives the straight-line midpoint
/// between the two connected node positions, suitable for label placement.
pub fn wire_midpoint(
    _wire: &WireElement,
    from_pos: [f32; 2],
    to_pos: [f32; 2],
) -> [f32; 2] {
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
            if pt[0] < min[0] { min[0] = pt[0]; }
            if pt[1] < min[1] { min[1] = pt[1]; }
            if pt[0] > max[0] { max[0] = pt[0]; }
            if pt[1] > max[1] { max[1] = pt[1]; }
        }
        Some(ElementBounds { id: self.id, min, max })
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
        assert!((aabb.max[0] - 40.0).abs() < 1e-6, "max x should be 10+30=40, got {}", aabb.max[0]);
        assert!((aabb.max[1] - 60.0).abs() < 1e-6, "max y should be 20+40=60, got {}", aabb.max[1]);
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
        assert!((br[0] - 85.0).abs() < 1e-6, "br x should be 5+80=85, got {}", br[0]);
        assert!((br[1] - 50.0).abs() < 1e-6, "br y should be 10+40=50, got {}", br[1]);
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
}
