use crate::detection::{BBoxDetector, BBox, DetectionResult, LetterBoxResult};
use crate::segmentation::{SamPipeline, SamPrompts, BboxPrompt, SegmentationResult};
use crate::layout::{LayoutAnalyzer, DocBBox, LayoutAnalysis};

/// Input to the vision orchestration pipeline.
#[derive(Debug, Clone)]
pub struct VisionInput {
    pub width: u32,
    pub height: u32,
    pub component_labels: Vec<String>, // detected component labels (from LLM or detection model)
}

/// Output of the vision orchestration pipeline.
#[derive(Debug, Clone)]
pub struct VisionOutput {
    pub detection: DetectionResult,
    pub segments: SegmentationResult,
    pub layout: LayoutAnalysis,
    pub nomx_source: String,
    pub component_count: usize,
}

impl VisionOutput {
    pub fn is_empty(&self) -> bool { self.component_count == 0 }
}

/// Orchestrates the full pipeline: detect → segment → layout → nomx.
pub struct VisionOrchestrator {
    pub detector: BBoxDetector,
    pub segmenter: SamPipeline,
    pub layout_analyzer: LayoutAnalyzer,
}

impl VisionOrchestrator {
    pub fn new() -> Self {
        Self {
            detector: BBoxDetector::new(),
            segmenter: SamPipeline::new(),
            layout_analyzer: LayoutAnalyzer::new(),
        }
    }

    /// Run the full pipeline from a list of detected component boxes.
    pub fn process(&self, input: &VisionInput, boxes: Vec<BBox>) -> VisionOutput {
        // Stage 1: Post-process detections (NMS + scale)
        let lb = LetterBoxResult::compute(input.width, input.height, &self.detector.config);
        let detection = self.detector.postprocess(boxes, &lb);

        // Stage 2: Segment each detected bbox
        let mut prompts = SamPrompts::default();
        for b in &detection.boxes {
            prompts.bboxes.push(BboxPrompt { x1: b.x1, y1: b.y1, x2: b.x2, y2: b.y2 });
        }
        let segments = self.segmenter.predict(input.width, input.height, &prompts);

        // Stage 3: Layout analysis — convert bboxes to DocBBoxes + labels
        let layout_inputs: Vec<(String, DocBBox)> = detection.boxes.iter().enumerate().map(|(i, b)| {
            let label = input.component_labels.get(i).cloned()
                .unwrap_or_else(|| format!("component_{}", i));
            let doc_bbox = DocBBox::from_pixels(b.x1 as u32, b.y1 as u32, b.x2 as u32, b.y2 as u32, input.width, input.height);
            (label, doc_bbox)
        }).collect();
        let layout = self.layout_analyzer.analyze(layout_inputs);

        // Stage 4: Generate .nomx from layout reading order
        let nomx_source = self.to_nomx(&layout);
        let component_count = detection.count();

        VisionOutput { detection, segments, layout, nomx_source, component_count }
    }

    /// Convert layout analysis to .nomx source.
    fn to_nomx(&self, layout: &LayoutAnalysis) -> String {
        let mut lines = vec!["@nomx natural".to_string()];
        for &idx in &layout.reading_order {
            let token = &layout.tokens[idx];
            let name = token.token.text.replace(' ', "_").to_lowercase();
            let x = token.token.bbox.center_x();
            let y = token.token.bbox.center_y();
            lines.push(format!("define {} that ui_element(x={}, y={})", name, x, y));
        }
        lines.join("\n")
    }
}

impl Default for VisionOrchestrator {
    fn default() -> Self { Self::new() }
}

impl VisionOutput {
    /// Convert vision pipeline output into InspectFindings for NomInspector.
    pub fn to_inspect_findings(&self) -> Vec<crate::inspector::InspectFinding> {
        let mut findings = Vec::new();
        // One finding per detected component
        for (i, bbox) in self.detection.boxes.iter().enumerate() {
            let label = self.layout.tokens.get(i)
                .map(|t| t.token.text.clone())
                .unwrap_or_else(|| format!("component_{}", i));
            let value = format!("x1={},y1={},x2={},y2={}", bbox.x1, bbox.y1, bbox.x2, bbox.y2);
            findings.push(crate::inspector::InspectFinding::new(
                "visual_component",
                &label,
                &value,
                bbox.confidence,
            ));
        }
        // One finding for the generated nomx source
        if !self.nomx_source.is_empty() {
            findings.push(crate::inspector::InspectFinding::new(
                "nomx_generated",
                "nomx_source",
                &self.nomx_source,
                0.95,
            ));
        }
        findings
    }
}

#[cfg(test)]
mod bridge_tests {
    use super::*;

    fn make_box(x1: f32, y1: f32, x2: f32, y2: f32, conf: f32) -> crate::detection::BBox {
        crate::detection::BBox::new(x1, y1, x2, y2, conf, 0)
    }

    #[test]
    fn test_to_inspect_findings_produces_findings() {
        let orch = VisionOrchestrator::new();
        let input = VisionInput { width: 640, height: 480, component_labels: vec!["button".into()] };
        let boxes = vec![make_box(10.0, 10.0, 100.0, 50.0, 0.9)];
        let output = orch.process(&input, boxes);
        let findings = output.to_inspect_findings();
        assert!(!findings.is_empty());
    }

    #[test]
    fn test_to_inspect_findings_includes_nomx() {
        let orch = VisionOrchestrator::new();
        let input = VisionInput { width: 640, height: 480, component_labels: vec!["nav".into()] };
        let boxes = vec![make_box(0.0, 0.0, 640.0, 40.0, 0.95)];
        let output = orch.process(&input, boxes);
        let findings = output.to_inspect_findings();
        let has_nomx = findings.iter().any(|f| {
            // check category or value contains nomx
            format!("{:?}", f).contains("nomx")
        });
        assert!(has_nomx, "expected a nomx_generated finding");
    }

    #[test]
    fn test_empty_output_minimal_findings() {
        let orch = VisionOrchestrator::new();
        let input = VisionInput { width: 640, height: 480, component_labels: vec![] };
        let output = orch.process(&input, vec![]);
        let findings = output.to_inspect_findings();
        // nomx_source for empty input has only the header line — still produces nomx finding
        assert!(findings.len() <= 2);
    }
}

#[cfg(test)]
mod vision_orchestrator_tests {
    use super::*;

    fn make_box(x1: f32, y1: f32, x2: f32, y2: f32, conf: f32) -> BBox {
        BBox::new(x1, y1, x2, y2, conf, 0)
    }

    #[test]
    fn test_orchestrator_empty_boxes() {
        let orch = VisionOrchestrator::new();
        let input = VisionInput { width: 640, height: 480, component_labels: vec![] };
        let output = orch.process(&input, vec![]);
        assert!(output.is_empty());
    }

    #[test]
    fn test_orchestrator_single_box() {
        let orch = VisionOrchestrator::new();
        let input = VisionInput { width: 640, height: 480, component_labels: vec!["button".into()] };
        let boxes = vec![make_box(10.0, 10.0, 100.0, 50.0, 0.9)];
        let output = orch.process(&input, boxes);
        assert_eq!(output.component_count, 1);
        assert!(!output.nomx_source.is_empty());
    }

    #[test]
    fn test_nomx_starts_with_header() {
        let orch = VisionOrchestrator::new();
        let input = VisionInput { width: 640, height: 480, component_labels: vec!["nav".into()] };
        let boxes = vec![make_box(0.0, 0.0, 640.0, 50.0, 0.95)];
        let output = orch.process(&input, boxes);
        assert!(output.nomx_source.starts_with("@nomx natural"));
    }

    #[test]
    fn test_orchestrator_multiple_boxes() {
        let orch = VisionOrchestrator::new();
        let input = VisionInput {
            width: 1280, height: 720,
            component_labels: vec!["header".into(), "sidebar".into(), "content".into()],
        };
        let boxes = vec![
            make_box(0.0, 0.0, 1280.0, 60.0, 0.95),
            make_box(0.0, 60.0, 200.0, 720.0, 0.88),
            make_box(200.0, 60.0, 1280.0, 720.0, 0.92),
        ];
        let output = orch.process(&input, boxes);
        assert_eq!(output.component_count, 3);
    }

    #[test]
    fn test_segment_count_matches_boxes() {
        let orch = VisionOrchestrator::new();
        let input = VisionInput { width: 800, height: 600, component_labels: vec!["a".into(), "b".into()] };
        let boxes = vec![
            make_box(10.0, 10.0, 100.0, 100.0, 0.9),
            make_box(200.0, 10.0, 300.0, 100.0, 0.85),
        ];
        let output = orch.process(&input, boxes);
        assert_eq!(output.segments.mask_count(), 2);
    }

    #[test]
    fn test_nomx_contains_define() {
        let orch = VisionOrchestrator::new();
        let input = VisionInput { width: 640, height: 480, component_labels: vec!["login_button".into()] };
        let boxes = vec![make_box(100.0, 100.0, 200.0, 140.0, 0.9)];
        let output = orch.process(&input, boxes);
        assert!(output.nomx_source.contains("define "));
        assert!(output.nomx_source.contains("ui_element"));
    }

    #[test]
    fn test_orchestrator_new_default() {
        let orch = VisionOrchestrator::default();
        assert_eq!(orch.detector.conf_threshold, 0.25);
    }
}
