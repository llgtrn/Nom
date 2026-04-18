/// A point prompt for SAM (x, y in pixel coordinates).
#[derive(Debug, Clone, Copy)]
pub struct PointPrompt {
    pub x: f32,
    pub y: f32,
    pub label: PointLabel, // foreground or background
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum PointLabel {
    Foreground = 1,
    Background = 0,
}

/// A bbox prompt for SAM (xyxy pixel coordinates).
#[derive(Debug, Clone, Copy)]
pub struct BboxPrompt {
    pub x1: f32,
    pub y1: f32,
    pub x2: f32,
    pub y2: f32,
}

/// Collection of prompts for one segmentation request.
#[derive(Debug, Clone, Default)]
pub struct SamPrompts {
    pub points: Vec<PointPrompt>,
    pub bboxes: Vec<BboxPrompt>,
}

impl SamPrompts {
    pub fn from_point(x: f32, y: f32) -> Self {
        Self {
            points: vec![PointPrompt { x, y, label: PointLabel::Foreground }],
            bboxes: vec![],
        }
    }

    pub fn from_bbox(x1: f32, y1: f32, x2: f32, y2: f32) -> Self {
        Self {
            points: vec![],
            bboxes: vec![BboxPrompt { x1, y1, x2, y2 }],
        }
    }

    pub fn is_empty(&self) -> bool {
        self.points.is_empty() && self.bboxes.is_empty()
    }
}

/// Binary segmentation mask.
#[derive(Debug, Clone)]
pub struct BinaryMask {
    pub data: Vec<bool>, // row-major flattened
    pub width: u32,
    pub height: u32,
}

impl BinaryMask {
    pub fn new(width: u32, height: u32) -> Self {
        Self { data: vec![false; (width * height) as usize], width, height }
    }

    pub fn pixel(&self, x: u32, y: u32) -> bool {
        if x >= self.width || y >= self.height {
            return false;
        }
        self.data[(y * self.width + x) as usize]
    }

    pub fn set_pixel(&mut self, x: u32, y: u32, val: bool) {
        if x < self.width && y < self.height {
            self.data[(y * self.width + x) as usize] = val;
        }
    }

    pub fn pixel_count(&self) -> usize {
        self.data.iter().filter(|&&v| v).count()
    }

    /// Derive bounding box from mask extent.
    pub fn bounding_box(&self) -> Option<(u32, u32, u32, u32)> {
        let mut min_x = self.width;
        let mut min_y = self.height;
        let mut max_x = 0u32;
        let mut max_y = 0u32;
        for y in 0..self.height {
            for x in 0..self.width {
                if self.pixel(x, y) {
                    min_x = min_x.min(x);
                    min_y = min_y.min(y);
                    max_x = max_x.max(x);
                    max_y = max_y.max(y);
                }
            }
        }
        if min_x > max_x {
            None
        } else {
            Some((min_x, min_y, max_x, max_y))
        }
    }
}

/// Result from SAM segmentation.
#[derive(Debug, Clone)]
pub struct SegmentationResult {
    pub masks: Vec<BinaryMask>,
    pub iou_scores: Vec<f32>,
    pub stability_scores: Vec<f32>,
}

impl SegmentationResult {
    pub fn mask_count(&self) -> usize {
        self.masks.len()
    }

    pub fn best_mask(&self) -> Option<&BinaryMask> {
        if self.masks.is_empty() {
            return None;
        }
        let best_idx = self
            .iou_scores
            .iter()
            .enumerate()
            .max_by(|a, b| a.1.partial_cmp(b.1).unwrap_or(std::cmp::Ordering::Equal))
            .map(|(i, _)| i)?;
        self.masks.get(best_idx)
    }
}

/// SAM configuration.
#[derive(Debug, Clone)]
pub struct SamConfig {
    pub image_size: u32,       // 1024
    pub mask_size: u32,        // 256
    pub mask_threshold: f32,   // 0.0
    pub stability_thresh: f32, // 0.95
    pub points_per_side: u32,  // 32 for auto generation
}

impl Default for SamConfig {
    fn default() -> Self {
        Self {
            image_size: 1024,
            mask_size: 256,
            mask_threshold: 0.0,
            stability_thresh: 0.95,
            points_per_side: 32,
        }
    }
}

/// Point grid for automatic mask generation.
pub struct PointGrid {
    pub config: SamConfig,
}

impl PointGrid {
    pub fn generate(&self, image_w: u32, image_h: u32) -> Vec<PointPrompt> {
        let n = self.config.points_per_side;
        let mut points = Vec::with_capacity((n * n) as usize);
        for i in 0..n {
            for j in 0..n {
                let x = (j as f32 + 0.5) / n as f32 * image_w as f32;
                let y = (i as f32 + 0.5) / n as f32 * image_h as f32;
                points.push(PointPrompt { x, y, label: PointLabel::Foreground });
            }
        }
        points
    }
}

/// SAM inference pipeline (stub — returns synthetic masks).
pub struct SamPipeline {
    pub config: SamConfig,
}

impl SamPipeline {
    pub fn new() -> Self {
        Self { config: SamConfig::default() }
    }

    /// Predict masks from prompts (stub returns one mask per bbox or point).
    pub fn predict(
        &self,
        image_w: u32,
        image_h: u32,
        prompts: &SamPrompts,
    ) -> SegmentationResult {
        if prompts.is_empty() {
            return SegmentationResult {
                masks: vec![],
                iou_scores: vec![],
                stability_scores: vec![],
            };
        }
        let n_masks = (prompts.points.len() + prompts.bboxes.len()).max(1);
        let mut masks = Vec::with_capacity(n_masks);
        let mut iou_scores = Vec::with_capacity(n_masks);
        let mut stability_scores = Vec::with_capacity(n_masks);
        for bbox in &prompts.bboxes {
            let mut mask = BinaryMask::new(image_w, image_h);
            let x1 = bbox.x1 as u32;
            let y1 = bbox.y1 as u32;
            let x2 = (bbox.x2 as u32).min(image_w - 1);
            let y2 = (bbox.y2 as u32).min(image_h - 1);
            for y in y1..=y2 {
                for x in x1..=x2 {
                    mask.set_pixel(x, y, true);
                }
            }
            masks.push(mask);
            iou_scores.push(0.92);
            stability_scores.push(0.97);
        }
        for pt in &prompts.points {
            let mut mask = BinaryMask::new(image_w, image_h);
            let cx = pt.x as u32;
            let cy = pt.y as u32;
            let r = 20u32;
            for dy in 0..r * 2 {
                for dx in 0..r * 2 {
                    let x = cx.saturating_add(dx).saturating_sub(r);
                    let y = cy.saturating_add(dy).saturating_sub(r);
                    mask.set_pixel(x, y, true);
                }
            }
            masks.push(mask);
            iou_scores.push(0.85);
            stability_scores.push(0.90);
        }
        SegmentationResult { masks, iou_scores, stability_scores }
    }
}

impl Default for SamPipeline {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod segmentation_tests {
    use super::*;

    #[test]
    fn test_sam_prompts_from_point() {
        let p = SamPrompts::from_point(100.0, 200.0);
        assert_eq!(p.points.len(), 1);
        assert_eq!(p.points[0].label, PointLabel::Foreground);
    }

    #[test]
    fn test_sam_prompts_from_bbox() {
        let p = SamPrompts::from_bbox(10.0, 10.0, 50.0, 50.0);
        assert_eq!(p.bboxes.len(), 1);
        assert!(p.points.is_empty());
    }

    #[test]
    fn test_binary_mask_pixel_count() {
        let mut mask = BinaryMask::new(10, 10);
        mask.set_pixel(0, 0, true);
        mask.set_pixel(5, 5, true);
        assert_eq!(mask.pixel_count(), 2);
    }

    #[test]
    fn test_binary_mask_bounding_box() {
        let mut mask = BinaryMask::new(100, 100);
        mask.set_pixel(10, 10, true);
        mask.set_pixel(20, 30, true);
        let bb = mask.bounding_box();
        assert!(bb.is_some());
        let (x1, y1, x2, y2) = bb.unwrap();
        assert_eq!(x1, 10);
        assert_eq!(y1, 10);
        assert_eq!(x2, 20);
        assert_eq!(y2, 30);
    }

    #[test]
    fn test_empty_mask_no_bbox() {
        let mask = BinaryMask::new(100, 100);
        assert!(mask.bounding_box().is_none());
    }

    #[test]
    fn test_predict_bbox_mask() {
        let pipeline = SamPipeline::new();
        let prompts = SamPrompts::from_bbox(10.0, 10.0, 50.0, 50.0);
        let result = pipeline.predict(640, 640, &prompts);
        assert_eq!(result.mask_count(), 1);
        assert!(result.masks[0].pixel_count() > 0);
    }

    #[test]
    fn test_best_mask() {
        let pipeline = SamPipeline::new();
        let mut prompts = SamPrompts::default();
        prompts.bboxes.push(BboxPrompt { x1: 10.0, y1: 10.0, x2: 50.0, y2: 50.0 });
        prompts.bboxes.push(BboxPrompt { x1: 100.0, y1: 100.0, x2: 150.0, y2: 150.0 });
        let result = pipeline.predict(640, 640, &prompts);
        assert!(result.best_mask().is_some());
    }

    #[test]
    fn test_point_grid_count() {
        let config = SamConfig { points_per_side: 8, ..Default::default() };
        let grid = PointGrid { config };
        let pts = grid.generate(640, 480);
        assert_eq!(pts.len(), 64); // 8x8
    }

    #[test]
    fn test_empty_prompts_no_masks() {
        let pipeline = SamPipeline::new();
        let result = pipeline.predict(640, 640, &SamPrompts::default());
        assert_eq!(result.mask_count(), 0);
    }
}
