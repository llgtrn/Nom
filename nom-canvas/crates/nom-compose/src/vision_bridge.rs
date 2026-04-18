use crate::inspector::{InspectFinding, InspectReport};
use crate::vision_orchestrator::VisionOutput;

/// Configuration for VisionBridge.
pub struct VisionBridgeConfig {
    pub confidence_threshold: f32,
    pub max_findings: usize,
}

impl Default for VisionBridgeConfig {
    fn default() -> Self {
        Self {
            confidence_threshold: 0.5,
            max_findings: 50,
        }
    }
}

/// Bridges VisionOrchestrator output into NomInspector InspectFinding entries.
pub struct VisionBridge {
    pub config: VisionBridgeConfig,
}

impl VisionBridge {
    pub fn new(config: VisionBridgeConfig) -> Self {
        Self { config }
    }

    /// Converts VisionOutput boxes/segments/layout tokens to InspectFinding entries.
    pub fn vision_to_findings(output: &VisionOutput) -> Vec<InspectFinding> {
        let mut findings = Vec::new();

        // One finding per detection bbox
        for bbox in &output.detection.boxes {
            findings.push(InspectFinding::new(
                "vision_detection",
                "bbox",
                &format!("{:?}", bbox),
                bbox.confidence,
            ));
        }

        // One finding per segment with non-zero mask area
        for mask in &output.segments.masks {
            let pixel_count = mask.pixel_count();
            if pixel_count > 0 {
                findings.push(InspectFinding::new(
                    "vision_segment",
                    "mask",
                    &format!("pixels:{}", pixel_count),
                    0.8,
                ));
            }
        }

        // One finding per layout token (field key = token text, value = bbox coords)
        for token in &output.layout.tokens {
            let field_key = &token.token.text;
            let field_value = format!(
                "x={},y={}",
                token.token.bbox.center_x(),
                token.token.bbox.center_y()
            );
            findings.push(InspectFinding::new(
                "vision_layout",
                field_key,
                &field_value,
                0.9,
            ));
        }

        findings
    }

    /// Keep only findings where confidence >= threshold, limited to max_findings.
    pub fn filter_by_confidence(&self, findings: Vec<InspectFinding>) -> Vec<InspectFinding> {
        findings
            .into_iter()
            .filter(|f| f.confidence >= self.config.confidence_threshold)
            .take(self.config.max_findings)
            .collect()
    }

    /// Converts VisionOutput to findings, filters them, and appends to report.
    pub fn enrich_report(report: &mut InspectReport, output: &VisionOutput, config: &VisionBridgeConfig) {
        let bridge = VisionBridge::new(VisionBridgeConfig {
            confidence_threshold: config.confidence_threshold,
            max_findings: config.max_findings,
        });
        let findings = Self::vision_to_findings(output);
        let filtered = bridge.filter_by_confidence(findings);
        for finding in filtered {
            report.findings.push(finding);
        }
    }
}

#[cfg(test)]
mod vision_bridge_tests {
    use super::*;
    use crate::detection::{BBox, DetectionResult};
    use crate::segmentation::SegmentationResult;
    use crate::layout::LayoutAnalysis;
    use crate::vision_orchestrator::VisionOutput;
    use crate::inspector::{InspectReport, InspectTarget};

    fn empty_vision_output() -> VisionOutput {
        VisionOutput {
            detection: DetectionResult { boxes: vec![], image_w: 640, image_h: 480 },
            segments: SegmentationResult { masks: vec![], iou_scores: vec![], stability_scores: vec![] },
            layout: LayoutAnalysis { tokens: vec![], reading_order: vec![], line_groups: vec![] },
            nomx_source: String::new(),
            component_count: 0,
        }
    }

    // Test 1: VisionBridgeConfig defaults
    #[test]
    fn test_vision_bridge_config_defaults() {
        let config = VisionBridgeConfig::default();
        assert_eq!(config.confidence_threshold, 0.5);
        assert_eq!(config.max_findings, 50);
    }

    // Test 2: vision_to_findings with empty VisionOutput returns empty vec
    #[test]
    fn test_vision_to_findings_empty_output() {
        let output = empty_vision_output();
        let findings = VisionBridge::vision_to_findings(&output);
        assert!(findings.is_empty());
    }

    // Test 3: vision_to_findings with boxes produces findings with category "vision_detection"
    #[test]
    fn test_vision_to_findings_with_boxes() {
        let mut output = empty_vision_output();
        output.detection.boxes.push(BBox::new(10.0, 20.0, 100.0, 80.0, 0.9, 0));
        output.component_count = 1;

        let findings = VisionBridge::vision_to_findings(&output);
        assert!(!findings.is_empty());
        assert!(findings.iter().any(|f| f.category == "vision_detection"));
    }

    // Test 4: filter_by_confidence filters low-confidence findings
    #[test]
    fn test_filter_by_confidence_removes_low() {
        let config = VisionBridgeConfig { confidence_threshold: 0.7, max_findings: 50 };
        let bridge = VisionBridge::new(config);

        let findings = vec![
            InspectFinding::new("cat", "k", "v", 0.9),
            InspectFinding::new("cat", "k", "v", 0.3),
            InspectFinding::new("cat", "k", "v", 0.8),
        ];
        let filtered = bridge.filter_by_confidence(findings);
        assert_eq!(filtered.len(), 2);
        assert!(filtered.iter().all(|f| f.confidence >= 0.7));
    }

    // Test 5: filter_by_confidence respects max_findings cap
    #[test]
    fn test_filter_by_confidence_respects_max() {
        let config = VisionBridgeConfig { confidence_threshold: 0.0, max_findings: 3 };
        let bridge = VisionBridge::new(config);

        let findings: Vec<InspectFinding> = (0..10)
            .map(|i| InspectFinding::new("cat", "k", "v", 0.5 + i as f32 * 0.01))
            .collect();
        let filtered = bridge.filter_by_confidence(findings);
        assert_eq!(filtered.len(), 3);
    }

    // Test 6: enrich_report appends findings to existing report
    #[test]
    fn test_enrich_report_appends_findings() {
        let mut output = empty_vision_output();
        output.detection.boxes.push(BBox::new(0.0, 0.0, 50.0, 50.0, 0.9, 0));
        output.component_count = 1;

        let mut report = InspectReport::new(InspectTarget::Website { url: "http://x.com".into() });
        report.findings.push(InspectFinding::new("existing", "k", "v", 0.9));

        let config = VisionBridgeConfig::default();
        VisionBridge::enrich_report(&mut report, &output, &config);

        assert!(report.findings.len() > 1, "should have pre-existing + new findings");
        assert!(report.findings.iter().any(|f| f.category == "vision_detection"));
    }

    // Test 7: enrich_report with empty output adds no findings
    #[test]
    fn test_enrich_report_empty_output_no_change() {
        let output = empty_vision_output();
        let mut report = InspectReport::new(InspectTarget::Website { url: "http://x.com".into() });
        let config = VisionBridgeConfig::default();
        VisionBridge::enrich_report(&mut report, &output, &config);
        assert!(report.findings.is_empty());
    }

    // Test 8: VisionBridge::new accepts config
    #[test]
    fn test_vision_bridge_new_accepts_config() {
        let config = VisionBridgeConfig { confidence_threshold: 0.6, max_findings: 20 };
        let bridge = VisionBridge::new(config);
        assert_eq!(bridge.config.confidence_threshold, 0.6);
        assert_eq!(bridge.config.max_findings, 20);
    }
}
