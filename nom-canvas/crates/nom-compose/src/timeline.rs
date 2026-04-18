#[derive(Debug, Clone, Copy)]
pub struct SequenceContext {
    pub cumulated_from: u32,
    pub relative_from: u32,
    pub duration_frames: u32,
}

impl SequenceContext {
    pub fn new(relative_from: u32, duration_frames: u32) -> Self {
        Self {
            cumulated_from: 0,
            relative_from,
            duration_frames,
        }
    }

    /// Nest a child sequence inside this one.
    pub fn nested(&self, child_from: u32, child_duration: u32) -> Self {
        Self {
            cumulated_from: self.cumulated_from + self.relative_from,
            relative_from: child_from,
            duration_frames: child_duration,
        }
    }
}

/// Returns the frame relative to this sequence (0-based within sequence).
pub fn current_frame_in_sequence(absolute_frame: u32, ctx: &SequenceContext) -> u32 {
    absolute_frame.saturating_sub(ctx.cumulated_from + ctx.relative_from)
}

/// Whether a frame is within the sequence's active window.
pub fn is_frame_active(absolute_frame: u32, ctx: &SequenceContext) -> bool {
    let start = ctx.cumulated_from + ctx.relative_from;
    if absolute_frame < start {
        return false;
    }
    let local = absolute_frame - start;
    local < ctx.duration_frames
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sequence_frame_offset() {
        let ctx = SequenceContext::new(10, 30);
        // absolute frame 15 → local frame 5
        assert_eq!(current_frame_in_sequence(15, &ctx), 5);
    }

    #[test]
    fn test_sequence_nested_compound_offset() {
        let parent = SequenceContext::new(10, 60);
        let child = parent.nested(5, 20);
        // cumulated = 0 + 10 = 10, relative = 5
        // absolute frame 20 → local = 20 - (10 + 5) = 5
        assert_eq!(current_frame_in_sequence(20, &child), 5);
    }

    #[test]
    fn test_sequence_is_frame_active() {
        let ctx = SequenceContext::new(10, 30);
        // frame 10 → local 0, active (0 < 30)
        assert!(is_frame_active(10, &ctx));
        // frame 39 → local 29, active (29 < 30)
        assert!(is_frame_active(39, &ctx));
        // frame 40 → local 30, NOT active (30 < 30 is false)
        assert!(!is_frame_active(40, &ctx));
    }

    #[test]
    fn test_sequence_frame_before_start_inactive() {
        let ctx = SequenceContext::new(10, 30);
        // absolute frame 5 is before the sequence start of 10
        assert!(!is_frame_active(5, &ctx), "frame before sequence start must be inactive");
    }
}
