//! ABI vision pipeline stubs: bounding-box detection, SAM segmentation,
//! layout analysis, animation generation, and orchestration.
//!
//! All types are pure-computation (no I/O) and suitable for lightweight tests.

// ─── BBox ────────────────────────────────────────────────────────────────────

/// An axis-aligned bounding box with a confidence score.
#[derive(Debug, Clone, PartialEq)]
pub struct BBox {
    /// Left edge in pixels.
    pub x: f32,
    /// Top edge in pixels.
    pub y: f32,
    /// Width in pixels.
    pub w: f32,
    /// Height in pixels.
    pub h: f32,
    /// Detection confidence in `0.0..=1.0`.
    pub confidence: f32,
}

impl BBox {
    /// Create a new bounding box.
    pub fn new(x: f32, y: f32, w: f32, h: f32, confidence: f32) -> Self {
        Self { x, y, w, h, confidence }
    }

    /// Area of the bounding box.
    pub fn area(&self) -> f32 {
        self.w * self.h
    }
}

// ─── BBoxDetector ────────────────────────────────────────────────────────────

/// Lightweight bounding-box detector stub (no model weights loaded).
#[derive(Debug, Default)]
pub struct BBoxDetector {
    /// Minimum confidence threshold for returned boxes.
    pub confidence_threshold: f32,
}

/// Result returned by [`BBoxDetector::postprocess`].
#[derive(Debug)]
pub struct BBoxResult {
    /// Boxes that passed the confidence threshold.
    pub boxes: Vec<BBox>,
}

impl BBoxResult {
    /// Number of accepted boxes.
    pub fn box_count(&self) -> usize {
        self.boxes.len()
    }
}

impl BBoxDetector {
    /// Create a detector with the given confidence threshold.
    pub fn new(confidence_threshold: f32) -> Self {
        Self { confidence_threshold }
    }

    /// Filter `raw_boxes` by `confidence_threshold` and return the result.
    pub fn postprocess(&self, raw_boxes: Vec<BBox>) -> BBoxResult {
        let boxes = raw_boxes
            .into_iter()
            .filter(|b| b.confidence >= self.confidence_threshold)
            .collect();
        BBoxResult { boxes }
    }
}

// ─── SamPipeline ─────────────────────────────────────────────────────────────

/// A bounding-box prompt fed to [`SamPipeline::predict`].
#[derive(Debug, Clone)]
pub struct BBoxPrompt {
    /// The box to segment within.
    pub bbox: BBox,
}

impl BBoxPrompt {
    /// Create a new prompt from a bounding box.
    pub fn new(bbox: BBox) -> Self {
        Self { bbox }
    }
}

/// Segmentation mask returned by [`SamPipeline::predict`].
#[derive(Debug)]
pub struct SegmentMask {
    /// Flat run-length-encoded mask pixels (stub: length encodes bbox area).
    pub rle_len: usize,
    /// Confidence of the mask prediction.
    pub confidence: f32,
}

impl SegmentMask {
    /// Return `true` if the mask covers a non-zero area.
    pub fn is_non_empty(&self) -> bool {
        self.rle_len > 0
    }
}

/// SAM (segment-anything) pipeline stub.
#[derive(Debug, Default)]
pub struct SamPipeline;

impl SamPipeline {
    /// Create a new pipeline instance.
    pub fn new() -> Self {
        Self
    }

    /// Produce a stub segmentation mask for the given bbox prompt.
    pub fn predict(&self, prompt: &BBoxPrompt) -> SegmentMask {
        let area = prompt.bbox.area() as usize;
        SegmentMask {
            rle_len: area.max(1),
            confidence: prompt.bbox.confidence,
        }
    }
}

// ─── LayoutAnalyzer ──────────────────────────────────────────────────────────

/// A single document token fed to [`LayoutAnalyzer::analyze`].
#[derive(Debug, Clone)]
pub struct DocToken {
    /// Token text.
    pub text: String,
    /// Bounding box of the token on the page.
    pub bbox: BBox,
}

impl DocToken {
    /// Create a new document token.
    pub fn new(text: impl Into<String>, bbox: BBox) -> Self {
        Self { text: text.into(), bbox }
    }
}

/// Layout block: a group of tokens classified as a semantic region.
#[derive(Debug, Clone)]
pub struct LayoutBlock {
    /// Semantic label (e.g. `"paragraph"`, `"heading"`, `"table"`).
    pub label: String,
    /// Number of tokens in the block.
    pub token_count: usize,
}

/// Result returned by [`LayoutAnalyzer::analyze`].
#[derive(Debug)]
pub struct LayoutResult {
    /// Discovered layout blocks.
    pub blocks: Vec<LayoutBlock>,
}

impl LayoutResult {
    /// Total number of tokens across all blocks.
    pub fn total_tokens(&self) -> usize {
        self.blocks.iter().map(|b| b.token_count).sum()
    }
}

/// Document layout analyzer stub.
#[derive(Debug, Default)]
pub struct LayoutAnalyzer;

impl LayoutAnalyzer {
    /// Create a new analyzer.
    pub fn new() -> Self {
        Self
    }

    /// Group `tokens` into layout blocks (stub: one block per token).
    pub fn analyze(&self, tokens: Vec<DocToken>) -> LayoutResult {
        let blocks = tokens
            .into_iter()
            .map(|t| LayoutBlock {
                label: "paragraph".into(),
                token_count: t.text.split_whitespace().count().max(1),
            })
            .collect();
        LayoutResult { blocks }
    }
}

// ─── AnimationPipeline ───────────────────────────────────────────────────────

/// Configuration for [`AnimationPipeline`].
#[derive(Debug, Clone)]
pub struct AnimationConfig {
    /// Number of frames to generate.
    pub frame_count: usize,
    /// Target frames per second.
    pub fps: f32,
}

impl AnimationConfig {
    /// Create a config with the given frame count and fps.
    pub fn new(frame_count: usize, fps: f32) -> Self {
        Self { frame_count, fps }
    }

    /// Total duration of the animation in seconds.
    pub fn duration_secs(&self) -> f32 {
        self.frame_count as f32 / self.fps.max(1.0)
    }
}

/// A single generated animation frame (stub).
#[derive(Debug, Clone)]
pub struct AnimFrame {
    /// Zero-based frame index.
    pub index: usize,
    /// Timestamp in seconds.
    pub timestamp_secs: f32,
}

/// Animation generation pipeline stub.
#[derive(Debug)]
pub struct AnimationPipeline {
    /// Configuration for this pipeline.
    pub config: AnimationConfig,
}

impl AnimationPipeline {
    /// Create a pipeline with the given config.
    pub fn new(config: AnimationConfig) -> Self {
        Self { config }
    }

    /// Generate stub frames according to the pipeline config.
    pub fn generate(&self) -> Vec<AnimFrame> {
        let dt = 1.0 / self.config.fps.max(1.0);
        (0..self.config.frame_count)
            .map(|i| AnimFrame {
                index: i,
                timestamp_secs: i as f32 * dt,
            })
            .collect()
    }
}

// ─── VisionOrchestrator ──────────────────────────────────────────────────────

/// Output produced by [`VisionOrchestrator::process`].
#[derive(Debug)]
pub struct VisionOutput {
    /// Number of bounding boxes that were processed.
    pub processed_boxes: usize,
    /// Stub `.nomx` representation of the vision results.
    pub nomx_output: String,
}

impl VisionOutput {
    /// Return `true` if the nomx output is non-empty.
    pub fn has_nomx(&self) -> bool {
        !self.nomx_output.is_empty()
    }
}

/// High-level orchestrator that wires together detection, segmentation,
/// layout analysis, and animation in a single pipeline.
#[derive(Debug, Default)]
pub struct VisionOrchestrator {
    detector: BBoxDetector,
    sam: SamPipeline,
}

impl VisionOrchestrator {
    /// Create a new orchestrator with default sub-pipelines.
    pub fn new() -> Self {
        Self {
            detector: BBoxDetector::new(0.5),
            sam: SamPipeline::new(),
        }
    }

    /// Process a list of raw boxes through the full vision pipeline and return
    /// a [`VisionOutput`] containing the count and a stub `.nomx` string.
    pub fn process(&self, raw_boxes: Vec<BBox>) -> VisionOutput {
        let detected = self.detector.postprocess(raw_boxes);
        let masks: Vec<SegmentMask> = detected
            .boxes
            .iter()
            .map(|b| self.sam.predict(&BBoxPrompt::new(b.clone())))
            .collect();
        let nomx_lines: Vec<String> = masks
            .iter()
            .enumerate()
            .map(|(i, m)| {
                format!(
                    "define vision_result_{i} that confidence({:.2}) rle_len({})",
                    m.confidence, m.rle_len
                )
            })
            .collect();
        VisionOutput {
            processed_boxes: detected.boxes.len(),
            nomx_output: nomx_lines.join("\n"),
        }
    }
}

// ─── Tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bbox_area() {
        let b = BBox::new(0.0, 0.0, 10.0, 20.0, 0.9);
        assert!((b.area() - 200.0).abs() < 1e-6);
    }

    #[test]
    fn bbox_detector_filters() {
        let det = BBoxDetector::new(0.6);
        let boxes = vec![
            BBox::new(0.0, 0.0, 10.0, 10.0, 0.5),
            BBox::new(0.0, 0.0, 10.0, 10.0, 0.8),
        ];
        let result = det.postprocess(boxes);
        assert_eq!(result.box_count(), 1);
    }

    #[test]
    fn sam_pipeline_non_empty_mask() {
        let sam = SamPipeline::new();
        let prompt = BBoxPrompt::new(BBox::new(0.0, 0.0, 100.0, 50.0, 0.9));
        let mask = sam.predict(&prompt);
        assert!(mask.is_non_empty());
    }

    #[test]
    fn layout_analyzer_token_count() {
        let analyzer = LayoutAnalyzer::new();
        let tokens = vec![
            DocToken::new("hello world", BBox::new(0.0, 0.0, 50.0, 10.0, 1.0)),
            DocToken::new("foo", BBox::new(0.0, 20.0, 20.0, 10.0, 1.0)),
        ];
        let result = analyzer.analyze(tokens);
        assert_eq!(result.total_tokens(), 3); // "hello world" = 2, "foo" = 1
    }

    #[test]
    fn animation_pipeline_frame_count() {
        let cfg = AnimationConfig::new(4, 24.0);
        let pipeline = AnimationPipeline::new(cfg);
        let frames = pipeline.generate();
        assert_eq!(frames.len(), 4);
    }

    #[test]
    fn vision_orchestrator_nomx() {
        let orch = VisionOrchestrator::new();
        let boxes = vec![
            BBox::new(0.0, 0.0, 100.0, 50.0, 0.9),
            BBox::new(10.0, 10.0, 40.0, 40.0, 0.7),
        ];
        let output = orch.process(boxes);
        assert_eq!(output.processed_boxes, 2);
        assert!(output.has_nomx());
    }
}
