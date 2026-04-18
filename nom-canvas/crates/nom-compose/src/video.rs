/// Captures frames from a source, tracking position and dimensions.
pub struct FrameCapture {
    pub width: u32,
    pub height: u32,
    pub frame_index: u32,
}

impl FrameCapture {
    pub fn new(width: u32, height: u32) -> Self {
        Self {
            width,
            height,
            frame_index: 0,
        }
    }

    /// Returns RGBA bytes for the current frame (width * height * 4 bytes).
    /// Each byte is seeded from frame_index for deterministic test output.
    pub fn capture_frame(&self) -> Vec<u8> {
        let len = (self.width * self.height * 4) as usize;
        let seed = (self.frame_index & 0xFF) as u8;
        vec![seed; len]
    }

    /// Advances the frame index by one.
    pub fn advance(&mut self) {
        self.frame_index += 1;
    }
}

/// Names the active stage of the two-stage pipeline.
#[derive(Debug, Clone, PartialEq)]
pub enum PipelineStage {
    Capture,
    Encode,
    Idle,
}

/// Coordinates the capture and encode stages for a fixed number of frames.
pub struct TwoStagePipeline {
    pub capture: FrameCapture,
    pub stage: PipelineStage,
    pub frames_captured: u32,
    pub frames_encoded: u32,
    pub target_frames: u32,
}

impl TwoStagePipeline {
    pub fn new(width: u32, height: u32, target_frames: u32) -> Self {
        Self {
            capture: FrameCapture::new(width, height),
            stage: PipelineStage::Idle,
            frames_captured: 0,
            frames_encoded: 0,
            target_frames,
        }
    }

    /// Captures one frame then encodes one frame, advancing the stage accordingly.
    pub fn step(&mut self) {
        if self.is_complete() {
            self.stage = PipelineStage::Idle;
            return;
        }
        // Capture stage
        self.stage = PipelineStage::Capture;
        let _frame = self.capture.capture_frame();
        self.capture.advance();
        self.frames_captured += 1;

        // Encode stage
        self.stage = PipelineStage::Encode;
        self.frames_encoded += 1;

        if self.is_complete() {
            self.stage = PipelineStage::Idle;
        }
    }

    /// Returns true when frames_encoded has reached target_frames.
    pub fn is_complete(&self) -> bool {
        self.frames_encoded >= self.target_frames
    }

    /// Returns the ratio of encoded frames to target frames (0.0 – 1.0).
    pub fn progress_ratio(&self) -> f32 {
        if self.target_frames == 0 {
            return 1.0;
        }
        self.frames_encoded as f32 / self.target_frames as f32
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn frame_capture_new_fields() {
        let fc = FrameCapture::new(320, 240);
        assert_eq!(fc.width, 320);
        assert_eq!(fc.height, 240);
        assert_eq!(fc.frame_index, 0);
    }

    #[test]
    fn frame_capture_advance_increments() {
        let mut fc = FrameCapture::new(10, 10);
        fc.advance();
        assert_eq!(fc.frame_index, 1);
        fc.advance();
        assert_eq!(fc.frame_index, 2);
    }

    #[test]
    fn frame_capture_returns_correct_byte_count() {
        let fc = FrameCapture::new(4, 4);
        let bytes = fc.capture_frame();
        assert_eq!(bytes.len(), 4 * 4 * 4);
    }

    #[test]
    fn two_stage_pipeline_new() {
        let p = TwoStagePipeline::new(1920, 1080, 30);
        assert_eq!(p.capture.width, 1920);
        assert_eq!(p.capture.height, 1080);
        assert_eq!(p.target_frames, 30);
        assert_eq!(p.frames_captured, 0);
        assert_eq!(p.frames_encoded, 0);
        assert_eq!(p.stage, PipelineStage::Idle);
    }

    #[test]
    fn two_stage_pipeline_step_and_complete() {
        let mut p = TwoStagePipeline::new(2, 2, 3);
        assert!(!p.is_complete());
        p.step();
        assert_eq!(p.frames_encoded, 1);
        p.step();
        assert_eq!(p.frames_encoded, 2);
        p.step();
        assert_eq!(p.frames_encoded, 3);
        assert!(p.is_complete());
        assert_eq!(p.stage, PipelineStage::Idle);
    }

    #[test]
    fn two_stage_pipeline_progress_ratio() {
        let mut p = TwoStagePipeline::new(2, 2, 4);
        assert_eq!(p.progress_ratio(), 0.0);
        p.step();
        assert!((p.progress_ratio() - 0.25).abs() < f32::EPSILON);
        p.step();
        assert!((p.progress_ratio() - 0.5).abs() < f32::EPSILON);
        p.step();
        p.step();
        assert!((p.progress_ratio() - 1.0).abs() < f32::EPSILON);
    }
}
