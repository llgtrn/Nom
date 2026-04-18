/// Bounding box in normalized document coordinates [0, 1023].
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DocBBox {
    pub x0: u16,  // left (0-1023)
    pub y0: u16,  // top (0-1023)
    pub x1: u16,  // right (0-1023)
    pub y1: u16,  // bottom (0-1023)
}

impl DocBBox {
    pub fn new(x0: u16, y0: u16, x1: u16, y1: u16) -> Self {
        Self { x0: x0.min(1023), y0: y0.min(1023), x1: x1.min(1023), y1: y1.min(1023) }
    }

    pub fn width(&self) -> u16 { self.x1.saturating_sub(self.x0) }
    pub fn height(&self) -> u16 { self.y1.saturating_sub(self.y0) }
    pub fn center_x(&self) -> u16 { (self.x0 + self.x1) / 2 }
    pub fn center_y(&self) -> u16 { (self.y0 + self.y1) / 2 }
    pub fn area(&self) -> u32 { self.width() as u32 * self.height() as u32 }

    /// Normalize coordinates from pixel space to [0, 1023] range.
    pub fn from_pixels(px0: u32, py0: u32, px1: u32, py1: u32, img_w: u32, img_h: u32) -> Self {
        let scale = |v: u32, max: u32| ((v as f32 / max as f32) * 1023.0) as u16;
        Self::new(scale(px0, img_w), scale(py0, img_h), scale(px1, img_w), scale(py1, img_h))
    }
}

/// One token in a document with its text and spatial position.
#[derive(Debug, Clone)]
pub struct DocumentToken {
    pub text: String,
    pub bbox: DocBBox,
    pub token_id: u32,  // vocabulary token ID
}

impl DocumentToken {
    pub fn new(text: impl Into<String>, bbox: DocBBox) -> Self {
        let text = text.into();
        let token_id = text.len() as u32 % 50265; // stub hash
        Self { text, bbox, token_id }
    }
}

/// Spatial features extracted from a bbox (6 components: left, top, right, bottom, width, height).
#[derive(Debug, Clone)]
pub struct SpatialFeatures {
    pub left: u16,
    pub top: u16,
    pub right: u16,
    pub bottom: u16,
    pub width: u16,
    pub height: u16,
}

impl SpatialFeatures {
    pub fn from_bbox(bbox: &DocBBox) -> Self {
        Self {
            left: bbox.x0,
            top: bbox.y0,
            right: bbox.x1,
            bottom: bbox.y1,
            width: bbox.width(),
            height: bbox.height(),
        }
    }

    pub fn as_array(&self) -> [u16; 6] {
        [self.left, self.top, self.right, self.bottom, self.width, self.height]
    }
}

/// Document layout token with spatial features extracted.
#[derive(Debug, Clone)]
pub struct LayoutToken {
    pub token: DocumentToken,
    pub spatial: SpatialFeatures,
}

impl LayoutToken {
    pub fn new(token: DocumentToken) -> Self {
        let spatial = SpatialFeatures::from_bbox(&token.bbox);
        Self { token, spatial }
    }
}

/// Document structure understanding result.
#[derive(Debug, Clone)]
pub struct LayoutAnalysis {
    pub tokens: Vec<LayoutToken>,
    pub reading_order: Vec<usize>,   // indices sorted by reading order (top-left to bottom-right)
    pub line_groups: Vec<Vec<usize>>, // tokens grouped by line (similar y)
}

impl LayoutAnalysis {
    pub fn token_count(&self) -> usize { self.tokens.len() }

    pub fn text_content(&self) -> String {
        self.reading_order.iter()
            .map(|&i| self.tokens[i].token.text.as_str())
            .collect::<Vec<_>>()
            .join(" ")
    }

    pub fn spatial_density(&self) -> f32 {
        if self.tokens.is_empty() { return 0.0; }
        let total_area: u32 = self.tokens.iter().map(|t| t.token.bbox.area()).sum();
        let doc_area = 1023u32 * 1023;
        total_area as f32 / doc_area as f32
    }
}

/// LayoutAnalyzer: tokenize a document page into spatial tokens.
pub struct LayoutAnalyzer {
    pub line_tolerance: u16, // y-diff to consider same line
}

impl LayoutAnalyzer {
    pub fn new() -> Self { Self { line_tolerance: 10 } }

    /// Analyze a set of (text, bbox) pairs into a LayoutAnalysis.
    pub fn analyze(&self, inputs: Vec<(String, DocBBox)>) -> LayoutAnalysis {
        let tokens: Vec<LayoutToken> = inputs.into_iter()
            .map(|(text, bbox)| LayoutToken::new(DocumentToken::new(text, bbox)))
            .collect();

        // Sort by reading order: top-to-bottom, left-to-right
        let mut reading_order: Vec<usize> = (0..tokens.len()).collect();
        reading_order.sort_by(|&a, &b| {
            let ta = &tokens[a].token.bbox;
            let tb = &tokens[b].token.bbox;
            ta.y0.cmp(&tb.y0).then(ta.x0.cmp(&tb.x0))
        });

        // Group into lines by y proximity
        let mut line_groups: Vec<Vec<usize>> = Vec::new();
        for &idx in &reading_order {
            let y = tokens[idx].token.bbox.y0;
            let placed = line_groups.iter_mut().find(|g| {
                let last_y = tokens[*g.last().unwrap()].token.bbox.y0;
                y.abs_diff(last_y) <= self.line_tolerance
            });
            if let Some(group) = placed {
                group.push(idx);
            } else {
                line_groups.push(vec![idx]);
            }
        }

        LayoutAnalysis { tokens, reading_order, line_groups }
    }
}

impl Default for LayoutAnalyzer {
    fn default() -> Self { Self::new() }
}

#[cfg(test)]
mod layout_tests {
    use super::*;

    #[test]
    fn test_doc_bbox_width_height() {
        let b = DocBBox::new(100, 200, 300, 250);
        assert_eq!(b.width(), 200);
        assert_eq!(b.height(), 50);
    }

    #[test]
    fn test_doc_bbox_from_pixels() {
        let b = DocBBox::from_pixels(0, 0, 1000, 500, 1000, 500);
        assert_eq!(b.x1, 1023);
        assert_eq!(b.y1, 1023);
    }

    #[test]
    fn test_spatial_features_as_array() {
        let b = DocBBox::new(10, 20, 110, 70);
        let sf = SpatialFeatures::from_bbox(&b);
        let arr = sf.as_array();
        assert_eq!(arr[0], 10);  // left
        assert_eq!(arr[4], 100); // width
        assert_eq!(arr[5], 50);  // height
    }

    #[test]
    fn test_layout_analyzer_reading_order() {
        let analyzer = LayoutAnalyzer::new();
        let inputs = vec![
            ("bottom".into(), DocBBox::new(100, 500, 200, 520)),
            ("top".into(), DocBBox::new(100, 100, 200, 120)),
        ];
        let analysis = analyzer.analyze(inputs);
        assert_eq!(analysis.tokens[analysis.reading_order[0]].token.text, "top");
    }

    #[test]
    fn test_layout_text_content() {
        let analyzer = LayoutAnalyzer::new();
        let inputs = vec![
            ("hello".into(), DocBBox::new(0, 0, 50, 20)),
            ("world".into(), DocBBox::new(60, 0, 110, 20)),
        ];
        let analysis = analyzer.analyze(inputs);
        let text = analysis.text_content();
        assert!(text.contains("hello") && text.contains("world"));
    }

    #[test]
    fn test_line_grouping() {
        let analyzer = LayoutAnalyzer::new();
        let inputs = vec![
            ("a".into(), DocBBox::new(0, 100, 10, 110)),
            ("b".into(), DocBBox::new(20, 102, 30, 112)), // same line (y diff=2)
            ("c".into(), DocBBox::new(0, 200, 10, 210)), // different line
        ];
        let analysis = analyzer.analyze(inputs);
        assert_eq!(analysis.line_groups.len(), 2);
    }

    #[test]
    fn test_doc_bbox_area() {
        let b = DocBBox::new(0, 0, 100, 100);
        assert_eq!(b.area(), 10000);
    }

    #[test]
    fn test_empty_layout() {
        let analyzer = LayoutAnalyzer::new();
        let analysis = analyzer.analyze(vec![]);
        assert_eq!(analysis.token_count(), 0);
        assert_eq!(analysis.spatial_density(), 0.0);
    }

    #[test]
    fn test_layout_token_new() {
        let bbox = DocBBox::new(10, 20, 50, 40);
        let token = DocumentToken::new("test", bbox);
        let lt = LayoutToken::new(token);
        assert_eq!(lt.spatial.left, 10);
        assert_eq!(lt.spatial.top, 20);
    }
}
