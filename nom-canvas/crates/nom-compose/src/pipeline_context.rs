/// Status of a compose pipeline execution.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PipelineStatus {
    Pending,
    Running,
    Paused,
    Completed,
    Failed,
}

impl PipelineStatus {
    pub fn status_name(&self) -> &str {
        match self {
            PipelineStatus::Pending => "Pending",
            PipelineStatus::Running => "Running",
            PipelineStatus::Paused => "Paused",
            PipelineStatus::Completed => "Completed",
            PipelineStatus::Failed => "Failed",
        }
    }

    pub fn is_terminal(&self) -> bool {
        matches!(self, PipelineStatus::Completed | PipelineStatus::Failed)
    }

    pub fn can_pause(&self) -> bool {
        matches!(self, PipelineStatus::Running)
    }
}

/// A single chunk of streamed output from a pipeline.
#[derive(Debug, Clone)]
pub struct StreamChunk {
    pub sequence: u64,
    pub data: String,
    pub is_final: bool,
}

impl StreamChunk {
    pub fn new(sequence: u64, data: impl Into<String>, is_final: bool) -> Self {
        Self {
            sequence,
            data: data.into(),
            is_final,
        }
    }

    pub fn byte_len(&self) -> usize {
        self.data.len()
    }
}

/// Context for a running compose pipeline, tracking status and streamed output.
pub struct PipelineContext {
    pub id: u64,
    pub status: PipelineStatus,
    pub chunks: Vec<StreamChunk>,
}

impl PipelineContext {
    pub fn new(id: u64) -> Self {
        Self {
            id,
            status: PipelineStatus::Pending,
            chunks: Vec::new(),
        }
    }

    pub fn start(&mut self) {
        self.status = PipelineStatus::Running;
    }

    /// Sets status to Paused if currently Running. Returns true if the status changed.
    pub fn pause(&mut self) -> bool {
        if self.status == PipelineStatus::Running {
            self.status = PipelineStatus::Paused;
            true
        } else {
            false
        }
    }

    pub fn resume(&mut self) {
        if self.status == PipelineStatus::Paused {
            self.status = PipelineStatus::Running;
        }
    }

    /// Appends a chunk. Sequence is assigned as the current chunk count.
    /// If `is_final` is true, status transitions to Completed.
    pub fn push_chunk(&mut self, data: impl Into<String>, is_final: bool) {
        let sequence = self.chunks.len() as u64;
        self.chunks.push(StreamChunk::new(sequence, data, is_final));
        if is_final {
            self.status = PipelineStatus::Completed;
        }
    }

    /// Returns all chunk data joined into a single string.
    pub fn collected_output(&self) -> String {
        self.chunks.iter().map(|c| c.data.as_str()).collect::<Vec<_>>().join("")
    }

    /// Returns the total byte length across all chunks.
    pub fn total_bytes(&self) -> usize {
        self.chunks.iter().map(|c| c.byte_len()).sum()
    }
}

#[cfg(test)]
mod pipeline_context_tests {
    use super::*;

    // Test 1: PipelineStatus::is_terminal()
    #[test]
    fn pipeline_status_is_terminal() {
        assert!(!PipelineStatus::Pending.is_terminal());
        assert!(!PipelineStatus::Running.is_terminal());
        assert!(!PipelineStatus::Paused.is_terminal());
        assert!(PipelineStatus::Completed.is_terminal());
        assert!(PipelineStatus::Failed.is_terminal());
    }

    // Test 2: PipelineStatus::can_pause()
    #[test]
    fn pipeline_status_can_pause() {
        assert!(!PipelineStatus::Pending.can_pause());
        assert!(PipelineStatus::Running.can_pause());
        assert!(!PipelineStatus::Paused.can_pause());
        assert!(!PipelineStatus::Completed.can_pause());
        assert!(!PipelineStatus::Failed.can_pause());
    }

    // Test 3: PipelineContext::start() sets Running
    #[test]
    fn pipeline_context_start_sets_running() {
        let mut ctx = PipelineContext::new(1);
        assert_eq!(ctx.status, PipelineStatus::Pending);
        ctx.start();
        assert_eq!(ctx.status, PipelineStatus::Running);
    }

    // Test 4: pause() returns true from Running
    #[test]
    fn pause_returns_true_from_running() {
        let mut ctx = PipelineContext::new(2);
        ctx.start();
        let changed = ctx.pause();
        assert!(changed);
        assert_eq!(ctx.status, PipelineStatus::Paused);
    }

    // Test 5: pause() returns false from non-Running
    #[test]
    fn pause_returns_false_from_non_running() {
        let mut ctx = PipelineContext::new(3);
        // Pending — not running
        let changed = ctx.pause();
        assert!(!changed);
        assert_eq!(ctx.status, PipelineStatus::Pending);
    }

    // Test 6: push_chunk() increments chunk count
    #[test]
    fn push_chunk_increments_count() {
        let mut ctx = PipelineContext::new(4);
        ctx.start();
        assert_eq!(ctx.chunks.len(), 0);
        ctx.push_chunk("hello", false);
        assert_eq!(ctx.chunks.len(), 1);
        ctx.push_chunk(" world", false);
        assert_eq!(ctx.chunks.len(), 2);
    }

    // Test 7: push_chunk() with is_final sets Completed
    #[test]
    fn push_chunk_final_sets_completed() {
        let mut ctx = PipelineContext::new(5);
        ctx.start();
        ctx.push_chunk("done", true);
        assert_eq!(ctx.status, PipelineStatus::Completed);
    }

    // Test 8: collected_output() joins chunks
    #[test]
    fn collected_output_joins_chunks() {
        let mut ctx = PipelineContext::new(6);
        ctx.start();
        ctx.push_chunk("foo", false);
        ctx.push_chunk("bar", false);
        ctx.push_chunk("baz", true);
        assert_eq!(ctx.collected_output(), "foobarbaz");
    }

    // Test 9: total_bytes() sums correctly
    #[test]
    fn total_bytes_sums_correctly() {
        let mut ctx = PipelineContext::new(7);
        ctx.start();
        ctx.push_chunk("abc", false);   // 3 bytes
        ctx.push_chunk("de", false);    // 2 bytes
        ctx.push_chunk("f", true);      // 1 byte
        assert_eq!(ctx.total_bytes(), 6);
    }
}
