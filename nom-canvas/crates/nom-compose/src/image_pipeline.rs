/// Pipeline orchestrator for image processing stages.
/// Distinct from image_dispatch.rs (model/capability dispatch layer).

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ImageStageKind {
    Preprocess,
    Infer,
    Postprocess,
    Encode,
}

impl ImageStageKind {
    pub fn is_gpu_accelerated(&self) -> bool {
        matches!(self, ImageStageKind::Infer | ImageStageKind::Encode)
    }

    pub fn stage_index(&self) -> usize {
        match self {
            ImageStageKind::Preprocess => 0,
            ImageStageKind::Infer => 1,
            ImageStageKind::Postprocess => 2,
            ImageStageKind::Encode => 3,
        }
    }
}

#[derive(Debug, Clone)]
pub struct ImageStage {
    pub kind: ImageStageKind,
    pub name: String,
    pub enabled: bool,
}

impl ImageStage {
    pub fn toggle(&mut self) {
        self.enabled = !self.enabled;
    }

    pub fn stage_label(&self) -> String {
        format!("[{}] {}", self.kind.stage_index(), self.name)
    }
}

#[derive(Debug, Clone, Default)]
pub struct ImagePipeline {
    pub stages: Vec<ImageStage>,
}

impl ImagePipeline {
    pub fn new() -> Self {
        Self { stages: Vec::new() }
    }

    pub fn add_stage(&mut self, s: ImageStage) {
        self.stages.push(s);
    }

    pub fn enabled_stages(&self) -> Vec<&ImageStage> {
        self.stages.iter().filter(|s| s.enabled).collect()
    }

    pub fn ordered_stages(&self) -> Vec<&ImageStage> {
        let mut refs: Vec<&ImageStage> = self.stages.iter().collect();
        refs.sort_by_key(|s| s.kind.stage_index());
        refs
    }

    pub fn has_infer(&self) -> bool {
        self.stages
            .iter()
            .any(|s| s.enabled && s.kind == ImageStageKind::Infer)
    }
}

#[derive(Debug, Clone)]
pub struct PipelineResult {
    pub success: bool,
    pub output_bytes: u64,
    pub stages_run: u32,
}

impl PipelineResult {
    pub fn is_empty(&self) -> bool {
        self.output_bytes == 0
    }

    pub fn throughput_kbps(&self, elapsed_ms: u64) -> f64 {
        if elapsed_ms == 0 {
            return 0.0;
        }
        self.output_bytes as f64 / elapsed_ms as f64 * 8.0
    }
}

pub struct ImagePipelineRunner {
    pub pipeline: ImagePipeline,
}

impl ImagePipelineRunner {
    pub fn new(p: ImagePipeline) -> Self {
        Self { pipeline: p }
    }

    pub fn run_stub(&self) -> PipelineResult {
        let enabled = self.pipeline.enabled_stages();
        let count = enabled.len();
        PipelineResult {
            success: true,
            output_bytes: 1024 * count as u64,
            stages_run: count as u32,
        }
    }

    pub fn stage_count(&self) -> usize {
        self.pipeline.stages.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Test 1: is_gpu_accelerated returns true only for Infer and Encode
    #[test]
    fn stage_kind_is_gpu_accelerated() {
        assert!(!ImageStageKind::Preprocess.is_gpu_accelerated());
        assert!(ImageStageKind::Infer.is_gpu_accelerated());
        assert!(!ImageStageKind::Postprocess.is_gpu_accelerated());
        assert!(ImageStageKind::Encode.is_gpu_accelerated());
    }

    // Test 2: stage_index returns correct ordinal per variant
    #[test]
    fn stage_kind_stage_index() {
        assert_eq!(ImageStageKind::Preprocess.stage_index(), 0);
        assert_eq!(ImageStageKind::Infer.stage_index(), 1);
        assert_eq!(ImageStageKind::Postprocess.stage_index(), 2);
        assert_eq!(ImageStageKind::Encode.stage_index(), 3);
    }

    // Test 3: toggle flips enabled state back and forth
    #[test]
    fn stage_toggle() {
        let mut stage = ImageStage {
            kind: ImageStageKind::Infer,
            name: "infer".to_string(),
            enabled: true,
        };
        stage.toggle();
        assert!(!stage.enabled);
        stage.toggle();
        assert!(stage.enabled);
    }

    // Test 4: stage_label formats as "[{index}] {name}"
    #[test]
    fn stage_stage_label_format() {
        let stage = ImageStage {
            kind: ImageStageKind::Postprocess,
            name: "sharpen".to_string(),
            enabled: true,
        };
        assert_eq!(stage.stage_label(), "[2] sharpen");
    }

    // Test 5: enabled_stages returns only enabled stages
    #[test]
    fn pipeline_enabled_stages_count() {
        let mut pipeline = ImagePipeline::new();
        pipeline.add_stage(ImageStage { kind: ImageStageKind::Preprocess, name: "prep".to_string(), enabled: true });
        pipeline.add_stage(ImageStage { kind: ImageStageKind::Infer, name: "infer".to_string(), enabled: false });
        pipeline.add_stage(ImageStage { kind: ImageStageKind::Encode, name: "enc".to_string(), enabled: true });

        let enabled = pipeline.enabled_stages();
        assert_eq!(enabled.len(), 2);
    }

    // Test 6: ordered_stages returns stages sorted by stage_index ascending
    #[test]
    fn pipeline_ordered_stages_sorted() {
        let mut pipeline = ImagePipeline::new();
        // Add in reverse order
        pipeline.add_stage(ImageStage { kind: ImageStageKind::Encode, name: "enc".to_string(), enabled: true });
        pipeline.add_stage(ImageStage { kind: ImageStageKind::Preprocess, name: "prep".to_string(), enabled: true });
        pipeline.add_stage(ImageStage { kind: ImageStageKind::Infer, name: "infer".to_string(), enabled: true });

        let ordered = pipeline.ordered_stages();
        assert_eq!(ordered[0].kind.stage_index(), 0);
        assert_eq!(ordered[1].kind.stage_index(), 1);
        assert_eq!(ordered[2].kind.stage_index(), 3);
    }

    // Test 7: has_infer returns true only when an enabled Infer stage is present
    #[test]
    fn pipeline_has_infer() {
        let mut pipeline = ImagePipeline::new();
        pipeline.add_stage(ImageStage { kind: ImageStageKind::Preprocess, name: "prep".to_string(), enabled: true });
        assert!(!pipeline.has_infer());

        pipeline.add_stage(ImageStage { kind: ImageStageKind::Infer, name: "infer".to_string(), enabled: false });
        assert!(!pipeline.has_infer(), "disabled Infer must not count");

        pipeline.stages[1].enabled = true;
        assert!(pipeline.has_infer());
    }

    // Test 8: run_stub output_bytes equals 1024 * enabled stage count
    #[test]
    fn runner_run_stub_output_bytes() {
        let mut pipeline = ImagePipeline::new();
        pipeline.add_stage(ImageStage { kind: ImageStageKind::Preprocess, name: "prep".to_string(), enabled: true });
        pipeline.add_stage(ImageStage { kind: ImageStageKind::Infer, name: "infer".to_string(), enabled: true });
        pipeline.add_stage(ImageStage { kind: ImageStageKind::Encode, name: "enc".to_string(), enabled: false });

        let runner = ImagePipelineRunner::new(pipeline);
        let result = runner.run_stub();
        assert!(result.success);
        assert_eq!(result.output_bytes, 2048); // 2 enabled * 1024
        assert_eq!(result.stages_run, 2);
    }

    // Test 9: throughput_kbps returns correct value; 0.0 when elapsed_ms==0
    #[test]
    fn result_throughput_kbps() {
        let r = PipelineResult { success: true, output_bytes: 1000, stages_run: 1 };
        // 1000 bytes / 100 ms * 8 = 80.0 kbps
        assert!((r.throughput_kbps(100) - 80.0).abs() < f64::EPSILON);
        assert_eq!(r.throughput_kbps(0), 0.0);
    }
}
