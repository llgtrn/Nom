#[derive(Debug, Clone, PartialEq)]
pub enum MediaKind {
    PixelGrid,
    AudioBuffer,
    VideoStream,
    VectorPath,
    GlyphOutline,
    MeshGeometry,
}

impl MediaKind {
    pub fn kind_name(&self) -> &str {
        match self {
            MediaKind::PixelGrid => "PixelGrid",
            MediaKind::AudioBuffer => "AudioBuffer",
            MediaKind::VideoStream => "VideoStream",
            MediaKind::VectorPath => "VectorPath",
            MediaKind::GlyphOutline => "GlyphOutline",
            MediaKind::MeshGeometry => "MeshGeometry",
        }
    }

    pub fn is_visual(&self) -> bool {
        matches!(
            self,
            MediaKind::PixelGrid
                | MediaKind::VideoStream
                | MediaKind::VectorPath
                | MediaKind::GlyphOutline
                | MediaKind::MeshGeometry
        )
    }

    pub fn is_audio(&self) -> bool {
        matches!(self, MediaKind::AudioBuffer)
    }
}

#[derive(Debug, Clone)]
pub struct MediaUnit {
    pub id: u64,
    pub kind: MediaKind,
    pub width: u32,
    pub height: u32,
    pub duration_ms: u64,
}

impl MediaUnit {
    pub fn new(id: u64, kind: MediaKind, width: u32, height: u32) -> Self {
        Self {
            id,
            kind,
            width,
            height,
            duration_ms: 0,
        }
    }

    pub fn with_duration(mut self, duration_ms: u64) -> Self {
        self.duration_ms = duration_ms;
        self
    }

    pub fn is_time_based(&self) -> bool {
        self.duration_ms > 0
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum ComposeOp {
    Sequence,
    Parallel,
    Overlay,
}

impl ComposeOp {
    pub fn op_symbol(&self) -> &str {
        match self {
            ComposeOp::Sequence => "→",
            ComposeOp::Parallel => "||",
            ComposeOp::Overlay => "⊕",
        }
    }
}

#[derive(Debug, Clone)]
pub struct MediaComposePipeline {
    pub steps: Vec<(ComposeOp, u64)>,
}

impl MediaComposePipeline {
    pub fn new() -> Self {
        Self { steps: Vec::new() }
    }

    pub fn add_step(&mut self, op: ComposeOp, unit_id: u64) {
        self.steps.push((op, unit_id));
    }

    pub fn step_count(&self) -> usize {
        self.steps.len()
    }

    pub fn ops_used(&self) -> Vec<&ComposeOp> {
        self.steps.iter().map(|(op, _)| op).collect()
    }
}

impl Default for MediaComposePipeline {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone)]
pub struct ComposePipelineResult {
    pub output_kind: MediaKind,
    pub estimated_duration_ms: u64,
    pub step_count: usize,
}

impl ComposePipelineResult {
    pub fn from_pipeline(pipeline: &MediaComposePipeline, units: &[MediaUnit]) -> Self {
        let output_kind = units
            .first()
            .map(|u| u.kind.clone())
            .unwrap_or(MediaKind::PixelGrid);

        let estimated_duration_ms = units
            .iter()
            .filter(|u| u.is_time_based())
            .map(|u| u.duration_ms)
            .sum();

        Self {
            output_kind,
            estimated_duration_ms,
            step_count: pipeline.step_count(),
        }
    }
}

#[cfg(test)]
mod compose_pipeline_tests {
    use super::*;

    #[test]
    fn media_kind_name() {
        assert_eq!(MediaKind::PixelGrid.kind_name(), "PixelGrid");
        assert_eq!(MediaKind::AudioBuffer.kind_name(), "AudioBuffer");
        assert_eq!(MediaKind::VideoStream.kind_name(), "VideoStream");
        assert_eq!(MediaKind::VectorPath.kind_name(), "VectorPath");
        assert_eq!(MediaKind::GlyphOutline.kind_name(), "GlyphOutline");
        assert_eq!(MediaKind::MeshGeometry.kind_name(), "MeshGeometry");
    }

    #[test]
    fn media_kind_is_visual_and_is_audio() {
        assert!(MediaKind::PixelGrid.is_visual());
        assert!(MediaKind::VideoStream.is_visual());
        assert!(MediaKind::VectorPath.is_visual());
        assert!(MediaKind::GlyphOutline.is_visual());
        assert!(MediaKind::MeshGeometry.is_visual());
        assert!(!MediaKind::AudioBuffer.is_visual());

        assert!(MediaKind::AudioBuffer.is_audio());
        assert!(!MediaKind::PixelGrid.is_audio());
        assert!(!MediaKind::VideoStream.is_audio());
    }

    #[test]
    fn media_unit_new_fields() {
        let u = MediaUnit::new(42, MediaKind::PixelGrid, 1920, 1080);
        assert_eq!(u.id, 42);
        assert_eq!(u.kind, MediaKind::PixelGrid);
        assert_eq!(u.width, 1920);
        assert_eq!(u.height, 1080);
        assert_eq!(u.duration_ms, 0);
    }

    #[test]
    fn media_unit_is_time_based_when_duration_gt_0() {
        let u = MediaUnit::new(1, MediaKind::VideoStream, 1280, 720).with_duration(5000);
        assert!(u.is_time_based());
        let u2 = MediaUnit::new(2, MediaKind::PixelGrid, 800, 600);
        assert!(!u2.is_time_based());
    }

    #[test]
    fn compose_op_symbol() {
        assert_eq!(ComposeOp::Sequence.op_symbol(), "→");
        assert_eq!(ComposeOp::Parallel.op_symbol(), "||");
        assert_eq!(ComposeOp::Overlay.op_symbol(), "⊕");
    }

    #[test]
    fn add_step_increments_step_count() {
        let mut p = MediaComposePipeline::new();
        assert_eq!(p.step_count(), 0);
        p.add_step(ComposeOp::Sequence, 1);
        assert_eq!(p.step_count(), 1);
        p.add_step(ComposeOp::Overlay, 2);
        assert_eq!(p.step_count(), 2);
    }

    #[test]
    fn ops_used_returns_correct_ops() {
        let mut p = MediaComposePipeline::new();
        p.add_step(ComposeOp::Sequence, 10);
        p.add_step(ComposeOp::Parallel, 20);
        p.add_step(ComposeOp::Overlay, 30);
        let ops = p.ops_used();
        assert_eq!(ops.len(), 3);
        assert_eq!(ops[0], &ComposeOp::Sequence);
        assert_eq!(ops[1], &ComposeOp::Parallel);
        assert_eq!(ops[2], &ComposeOp::Overlay);
    }

    #[test]
    fn result_from_pipeline_duration_sum() {
        let units = vec![
            MediaUnit::new(1, MediaKind::VideoStream, 1920, 1080).with_duration(3000),
            MediaUnit::new(2, MediaKind::AudioBuffer, 0, 0).with_duration(2000),
            MediaUnit::new(3, MediaKind::PixelGrid, 800, 600), // not time-based
        ];
        let mut p = MediaComposePipeline::new();
        p.add_step(ComposeOp::Sequence, 1);
        p.add_step(ComposeOp::Parallel, 2);
        let result = ComposePipelineResult::from_pipeline(&p, &units);
        assert_eq!(result.estimated_duration_ms, 5000);
        assert_eq!(result.output_kind, MediaKind::VideoStream);
        assert_eq!(result.step_count, 2);
    }

    #[test]
    fn result_from_pipeline_empty() {
        let p = MediaComposePipeline::new();
        let result = ComposePipelineResult::from_pipeline(&p, &[]);
        assert_eq!(result.output_kind, MediaKind::PixelGrid);
        assert_eq!(result.estimated_duration_ms, 0);
        assert_eq!(result.step_count, 0);
    }
}
