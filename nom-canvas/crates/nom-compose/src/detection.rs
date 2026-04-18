/// LetterBox image transformation: resize + pad preserving aspect ratio.
#[derive(Debug, Clone)]
pub struct LetterBoxConfig {
    pub target_size: (u32, u32), // (height, width)
    pub stride: u32,             // model stride, default 32
    pub pad_value: u8,           // fill color, default 114
    pub center: bool,            // center vs top-left padding
}

impl Default for LetterBoxConfig {
    fn default() -> Self {
        Self { target_size: (640, 640), stride: 32, pad_value: 114, center: true }
    }
}

/// Result of LetterBox transformation — tracks the reverse transform.
#[derive(Debug, Clone)]
pub struct LetterBoxResult {
    pub scale: f32,         // scale applied to original
    pub pad_x: f32,         // horizontal padding added
    pub pad_y: f32,         // vertical padding added
    pub original_w: u32,
    pub original_h: u32,
    pub target_w: u32,
    pub target_h: u32,
}

impl LetterBoxResult {
    /// Scale a bounding box from model space back to original image space.
    pub fn scale_box_to_original(&self, x1: f32, y1: f32, x2: f32, y2: f32) -> (f32, f32, f32, f32) {
        let ox1 = (x1 - self.pad_x) / self.scale;
        let oy1 = (y1 - self.pad_y) / self.scale;
        let ox2 = (x2 - self.pad_x) / self.scale;
        let oy2 = (y2 - self.pad_y) / self.scale;
        // Clip to original image bounds
        let ox1 = ox1.max(0.0).min(self.original_w as f32);
        let oy1 = oy1.max(0.0).min(self.original_h as f32);
        let ox2 = ox2.max(0.0).min(self.original_w as f32);
        let oy2 = oy2.max(0.0).min(self.original_h as f32);
        (ox1, oy1, ox2, oy2)
    }

    /// Compute scale + padding for a given original size → target.
    pub fn compute(orig_w: u32, orig_h: u32, config: &LetterBoxConfig) -> Self {
        let scale_w = config.target_size.1 as f32 / orig_w as f32;
        let scale_h = config.target_size.0 as f32 / orig_h as f32;
        let scale = scale_w.min(scale_h);
        let new_w = (orig_w as f32 * scale).round() as u32;
        let new_h = (orig_h as f32 * scale).round() as u32;
        let pad_x = (config.target_size.1 as f32 - new_w as f32) / 2.0;
        let pad_y = (config.target_size.0 as f32 - new_h as f32) / 2.0;
        Self {
            scale, pad_x, pad_y,
            original_w: orig_w, original_h: orig_h,
            target_w: config.target_size.1, target_h: config.target_size.0,
        }
    }
}

/// A detection bounding box in xyxy format.
#[derive(Debug, Clone)]
pub struct BBox {
    pub x1: f32, pub y1: f32,
    pub x2: f32, pub y2: f32,
    pub confidence: f32,
    pub class_id: u32,
    pub class_label: Option<String>,
}

impl BBox {
    pub fn new(x1: f32, y1: f32, x2: f32, y2: f32, confidence: f32, class_id: u32) -> Self {
        Self { x1, y1, x2, y2, confidence, class_id, class_label: None }
    }

    pub fn area(&self) -> f32 {
        (self.x2 - self.x1).max(0.0) * (self.y2 - self.y1).max(0.0)
    }

    pub fn iou(&self, other: &BBox) -> f32 {
        let ix1 = self.x1.max(other.x1);
        let iy1 = self.y1.max(other.y1);
        let ix2 = self.x2.min(other.x2);
        let iy2 = self.y2.min(other.y2);
        let inter = (ix2 - ix1).max(0.0) * (iy2 - iy1).max(0.0);
        if inter == 0.0 { return 0.0; }
        inter / (self.area() + other.area() - inter)
    }
}

/// Non-Maximum Suppression: filter overlapping boxes.
pub fn nms(mut boxes: Vec<BBox>, iou_threshold: f32) -> Vec<BBox> {
    // Sort by confidence descending
    boxes.sort_by(|a, b| b.confidence.partial_cmp(&a.confidence).unwrap_or(std::cmp::Ordering::Equal));
    let mut kept: Vec<BBox> = Vec::new();
    for candidate in boxes {
        let suppressed = kept.iter().any(|k| candidate.iou(k) > iou_threshold);
        if !suppressed {
            kept.push(candidate);
        }
    }
    kept
}

/// Detection result for one image.
#[derive(Debug, Clone)]
pub struct DetectionResult {
    pub boxes: Vec<BBox>,
    pub image_w: u32,
    pub image_h: u32,
}

impl DetectionResult {
    pub fn count(&self) -> usize { self.boxes.len() }
    pub fn by_class(&self, class_id: u32) -> Vec<&BBox> {
        self.boxes.iter().filter(|b| b.class_id == class_id).collect()
    }
}

/// Detection pipeline coordinator.
pub struct BBoxDetector {
    pub config: LetterBoxConfig,
    pub conf_threshold: f32,
    pub iou_threshold: f32,
}

impl BBoxDetector {
    pub fn new() -> Self {
        Self {
            config: LetterBoxConfig::default(),
            conf_threshold: 0.25,
            iou_threshold: 0.45,
        }
    }

    /// Filter boxes below confidence threshold then run NMS.
    pub fn postprocess(&self, boxes: Vec<BBox>, letterbox: &LetterBoxResult) -> DetectionResult {
        let filtered: Vec<BBox> = boxes.into_iter()
            .filter(|b| b.confidence >= self.conf_threshold)
            .map(|b| {
                let (x1, y1, x2, y2) = letterbox.scale_box_to_original(b.x1, b.y1, b.x2, b.y2);
                BBox { x1, y1, x2, y2, ..b }
            })
            .collect();
        let kept = nms(filtered, self.iou_threshold);
        DetectionResult {
            image_w: letterbox.original_w,
            image_h: letterbox.original_h,
            boxes: kept,
        }
    }
}

impl Default for BBoxDetector {
    fn default() -> Self { Self::new() }
}

#[cfg(test)]
mod detection_tests {
    use super::*;

    #[test]
    fn test_letterbox_compute() {
        let config = LetterBoxConfig::default();
        let lb = LetterBoxResult::compute(1280, 720, &config);
        assert!((lb.scale - 0.5).abs() < 0.01);
    }

    #[test]
    fn test_bbox_area() {
        let b = BBox::new(0.0, 0.0, 100.0, 50.0, 0.9, 0);
        assert_eq!(b.area(), 5000.0);
    }

    #[test]
    fn test_bbox_iou_perfect() {
        let b = BBox::new(0.0, 0.0, 10.0, 10.0, 0.9, 0);
        assert!((b.iou(&b) - 1.0).abs() < 0.001);
    }

    #[test]
    fn test_bbox_iou_no_overlap() {
        let b1 = BBox::new(0.0, 0.0, 10.0, 10.0, 0.9, 0);
        let b2 = BBox::new(20.0, 20.0, 30.0, 30.0, 0.8, 0);
        assert_eq!(b1.iou(&b2), 0.0);
    }

    #[test]
    fn test_nms_removes_duplicate() {
        let b1 = BBox::new(0.0, 0.0, 10.0, 10.0, 0.9, 0);
        let b2 = BBox::new(1.0, 1.0, 11.0, 11.0, 0.8, 0); // overlaps b1
        let result = nms(vec![b1, b2], 0.5);
        assert_eq!(result.len(), 1);
        assert!((result[0].confidence - 0.9).abs() < 0.01);
    }

    #[test]
    fn test_nms_keeps_distinct() {
        let b1 = BBox::new(0.0, 0.0, 10.0, 10.0, 0.9, 0);
        let b2 = BBox::new(50.0, 50.0, 60.0, 60.0, 0.8, 1);
        let result = nms(vec![b1, b2], 0.5);
        assert_eq!(result.len(), 2);
    }

    #[test]
    fn test_postprocess_filters_low_conf() {
        let detector = BBoxDetector::new();
        let lb = LetterBoxResult::compute(640, 640, &detector.config);
        let boxes = vec![
            BBox::new(10.0, 10.0, 50.0, 50.0, 0.1, 0), // below threshold
            BBox::new(10.0, 10.0, 50.0, 50.0, 0.9, 1), // above threshold
        ];
        let result = detector.postprocess(boxes, &lb);
        assert_eq!(result.count(), 1);
    }

    #[test]
    fn test_scale_box_to_original() {
        let config = LetterBoxConfig { target_size: (640, 640), stride: 32, pad_value: 114, center: true };
        let lb = LetterBoxResult::compute(320, 320, &config); // scale=2, pad=0
        let (x1, y1, x2, y2) = lb.scale_box_to_original(100.0, 100.0, 200.0, 200.0);
        assert!((x1 - 50.0).abs() < 1.0);
        assert!((y1 - 50.0).abs() < 1.0);
    }

    #[test]
    fn test_detection_result_by_class() {
        let result = DetectionResult {
            boxes: vec![
                BBox::new(0.0,0.0,1.0,1.0,0.9,0),
                BBox::new(0.0,0.0,1.0,1.0,0.8,1),
                BBox::new(0.0,0.0,1.0,1.0,0.7,0),
            ],
            image_w: 640, image_h: 640,
        };
        assert_eq!(result.by_class(0).len(), 2);
        assert_eq!(result.by_class(1).len(), 1);
    }
}
