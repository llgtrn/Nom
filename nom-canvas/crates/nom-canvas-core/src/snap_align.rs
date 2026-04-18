/// Axis an alignment guide operates on.
#[derive(Debug, Clone, PartialEq)]
pub enum AlignAxis {
    /// Guide runs horizontally (constrains the Y axis).
    Horizontal,
    /// Guide runs vertically (constrains the X axis).
    Vertical,
    /// Guide operates on both axes simultaneously.
    Both,
}

impl AlignAxis {
    /// Returns `true` when this axis includes horizontal alignment.
    pub fn includes_horizontal(&self) -> bool {
        matches!(self, AlignAxis::Horizontal | AlignAxis::Both)
    }

    /// Returns `true` when this axis includes vertical alignment.
    pub fn includes_vertical(&self) -> bool {
        matches!(self, AlignAxis::Vertical | AlignAxis::Both)
    }
}

/// A single smart alignment guide at a fixed canvas position.
#[derive(Debug, Clone)]
pub struct AlignGuide {
    /// Position of the guide on its axis (canvas coordinates).
    pub position: f32,
    /// Which axis this guide operates on.
    pub axis: AlignAxis,
    /// Human-readable label for this guide (e.g. `"center-x"`, `"top-edge"`).
    pub label: String,
}

impl AlignGuide {
    /// Absolute distance from the guide's position to `point`.
    pub fn distance_to(&self, point: f32) -> f32 {
        (self.position - point).abs()
    }

    /// Returns `true` when the distance to `point` is within `threshold` (inclusive).
    pub fn is_close(&self, point: f32, threshold: f32) -> bool {
        self.distance_to(point) <= threshold
    }
}

/// The element being snapped — defined by its top-left origin and dimensions.
#[derive(Debug, Clone)]
pub struct SnapTarget {
    /// Unique element identifier.
    pub element_id: u64,
    /// Left edge X coordinate.
    pub x: f32,
    /// Top edge Y coordinate.
    pub y: f32,
    /// Element width.
    pub width: f32,
    /// Element height.
    pub height: f32,
}

impl SnapTarget {
    /// Horizontal centre of the element.
    pub fn center_x(&self) -> f32 {
        self.x + self.width / 2.0
    }

    /// Vertical centre of the element.
    pub fn center_y(&self) -> f32 {
        self.y + self.height / 2.0
    }

    /// Right edge X coordinate.
    pub fn right_edge(&self) -> f32 {
        self.x + self.width
    }

    /// Bottom edge Y coordinate.
    pub fn bottom_edge(&self) -> f32 {
        self.y + self.height
    }
}

/// Result of an alignment snap operation.
#[derive(Debug, Clone, Default)]
pub struct AlignResult {
    /// Snapped X position, or `None` if no vertical guide matched.
    pub snapped_x: Option<f32>,
    /// Snapped Y position, or `None` if no horizontal guide matched.
    pub snapped_y: Option<f32>,
    /// Labels of every guide that contributed to the snap.
    pub active_guides: Vec<String>,
}

impl AlignResult {
    /// Returns `true` if at least one axis was snapped.
    pub fn is_snapped(&self) -> bool {
        self.snapped_x.is_some() || self.snapped_y.is_some()
    }

    /// Number of active guides that fired.
    pub fn guide_count(&self) -> usize {
        self.active_guides.len()
    }
}

/// Engine that evaluates a set of [`AlignGuide`]s against a [`SnapTarget`].
pub struct AlignmentEngine {
    /// All registered guides.
    pub guides: Vec<AlignGuide>,
    /// Snap threshold in canvas units.
    pub threshold: f32,
}

impl AlignmentEngine {
    /// Creates a new engine with no guides and the given threshold.
    pub fn new(threshold: f32) -> Self {
        Self {
            guides: Vec::new(),
            threshold,
        }
    }

    /// Adds a guide to the engine.
    pub fn add_guide(&mut self, g: AlignGuide) {
        self.guides.push(g);
    }

    /// Evaluates all guides against `target`.
    ///
    /// - Horizontal guides are checked against `target.center_y()`.
    /// - Vertical guides are checked against `target.center_x()`.
    /// - Both-axis guides are checked against both.
    ///
    /// Returns an [`AlignResult`] with the first matched position per axis and
    /// all matching guide labels.
    pub fn snap(&self, target: &SnapTarget) -> AlignResult {
        let mut result = AlignResult::default();

        for guide in &self.guides {
            if guide.axis.includes_horizontal() {
                let cy = target.center_y();
                if guide.is_close(cy, self.threshold) {
                    if result.snapped_y.is_none() {
                        result.snapped_y = Some(guide.position);
                    }
                    result.active_guides.push(guide.label.clone());
                }
            }
            if guide.axis.includes_vertical() {
                let cx = target.center_x();
                if guide.is_close(cx, self.threshold) {
                    if result.snapped_x.is_none() {
                        result.snapped_x = Some(guide.position);
                    }
                    if !result.active_guides.contains(&guide.label) {
                        result.active_guides.push(guide.label.clone());
                    }
                }
            }
        }

        result
    }
}

#[cfg(test)]
mod snap_align_tests {
    use super::*;

    // axis includes_horizontal
    #[test]
    fn axis_includes_horizontal() {
        assert!(AlignAxis::Horizontal.includes_horizontal());
        assert!(AlignAxis::Both.includes_horizontal());
        assert!(!AlignAxis::Vertical.includes_horizontal());
    }

    // axis includes_vertical (Both)
    #[test]
    fn axis_includes_vertical_both() {
        assert!(AlignAxis::Vertical.includes_vertical());
        assert!(AlignAxis::Both.includes_vertical());
        assert!(!AlignAxis::Horizontal.includes_vertical());
    }

    // guide distance_to
    #[test]
    fn guide_distance_to() {
        let g = AlignGuide {
            position: 100.0,
            axis: AlignAxis::Horizontal,
            label: "h".to_string(),
        };
        assert!((g.distance_to(95.0) - 5.0).abs() < f32::EPSILON);
        assert!((g.distance_to(110.0) - 10.0).abs() < f32::EPSILON);
    }

    // guide is_close true
    #[test]
    fn guide_is_close_true() {
        let g = AlignGuide {
            position: 50.0,
            axis: AlignAxis::Vertical,
            label: "v".to_string(),
        };
        assert!(g.is_close(47.0, 5.0));
        assert!(g.is_close(50.0, 0.0)); // exactly on guide
    }

    // guide is_close false
    #[test]
    fn guide_is_close_false() {
        let g = AlignGuide {
            position: 50.0,
            axis: AlignAxis::Vertical,
            label: "v".to_string(),
        };
        assert!(!g.is_close(60.0, 5.0));
    }

    // target center_x and center_y
    #[test]
    fn target_center_x_center_y() {
        let t = SnapTarget {
            element_id: 1,
            x: 10.0,
            y: 20.0,
            width: 80.0,
            height: 40.0,
        };
        assert!((t.center_x() - 50.0).abs() < f32::EPSILON);
        assert!((t.center_y() - 40.0).abs() < f32::EPSILON);
    }

    // target right_edge
    #[test]
    fn target_right_edge() {
        let t = SnapTarget {
            element_id: 2,
            x: 5.0,
            y: 0.0,
            width: 95.0,
            height: 10.0,
        };
        assert!((t.right_edge() - 100.0).abs() < f32::EPSILON);
        assert!((t.bottom_edge() - 10.0).abs() < f32::EPSILON);
    }

    // result is_snapped
    #[test]
    fn result_is_snapped() {
        let mut r = AlignResult::default();
        assert!(!r.is_snapped());
        r.snapped_x = Some(42.0);
        assert!(r.is_snapped());
    }

    // engine snap finds matching guide
    #[test]
    fn engine_snap_finds_matching_guide() {
        let mut engine = AlignmentEngine::new(8.0);
        engine.add_guide(AlignGuide {
            position: 100.0,
            axis: AlignAxis::Horizontal,
            label: "center-row".to_string(),
        });
        // Target with center_y = 104 (within threshold 8)
        let target = SnapTarget {
            element_id: 10,
            x: 0.0,
            y: 84.0,  // center_y = 84 + 40/2 = 104
            width: 60.0,
            height: 40.0,
        };
        let result = engine.snap(&target);
        assert!(result.snapped_y.is_some());
        assert_eq!(result.guide_count(), 1);
        assert_eq!(result.active_guides[0], "center-row");
    }

    // engine snap no match
    #[test]
    fn engine_snap_no_match() {
        let mut engine = AlignmentEngine::new(4.0);
        engine.add_guide(AlignGuide {
            position: 200.0,
            axis: AlignAxis::Vertical,
            label: "far-guide".to_string(),
        });
        // Target center_x = 50 — far from guide at 200
        let target = SnapTarget {
            element_id: 99,
            x: 20.0,
            y: 20.0,
            width: 60.0,
            height: 60.0,
        };
        let result = engine.snap(&target);
        assert!(!result.is_snapped());
        assert_eq!(result.guide_count(), 0);
    }
}
