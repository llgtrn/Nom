use std::collections::VecDeque;

/// A single chunk of a result that may arrive in pieces.
#[derive(Debug, Clone)]
pub struct PartialResult<T> {
    pub chunk_index: usize,
    pub total_chunks: usize,
    pub data: T,
}

impl<T> PartialResult<T> {
    pub fn new(chunk_index: usize, total_chunks: usize, data: T) -> Self {
        Self { chunk_index, total_chunks, data }
    }

    /// Returns true when this is the final chunk.
    pub fn is_last(&self) -> bool {
        self.chunk_index + 1 == self.total_chunks
    }

    /// Returns a value in [0.0, 1.0] representing how far through the stream we are.
    pub fn progress(&self) -> f32 {
        (self.chunk_index + 1) as f32 / self.total_chunks as f32
    }
}

/// Accumulates `PartialResult` chunks until all expected chunks have arrived.
#[derive(Debug, Clone)]
pub struct StreamingOutput<T: Clone> {
    pub chunks: Vec<T>,
    pub expected_total: usize,
}

impl<T: Clone> StreamingOutput<T> {
    pub fn new(expected_total: usize) -> Self {
        Self { chunks: Vec::new(), expected_total }
    }

    /// Push a chunk into the output buffer.
    pub fn push_chunk(&mut self, result: PartialResult<T>) {
        self.chunks.push(result.data);
    }

    /// Returns true once all expected chunks have been received.
    pub fn is_complete(&self) -> bool {
        self.chunks.len() == self.expected_total
    }

    /// Returns a clone of all accumulated chunk data in insertion order.
    pub fn collect(&self) -> Vec<T> {
        self.chunks.clone()
    }
}

/// A ring-buffer of completed `StreamingOutput` values, capped at `max_entries`.
#[derive(Debug, Clone)]
pub struct ResultBuffer<T: Clone> {
    pub entries: VecDeque<StreamingOutput<T>>,
    pub max_entries: usize,
}

impl<T: Clone> ResultBuffer<T> {
    pub fn new(max_entries: usize) -> Self {
        Self { entries: VecDeque::new(), max_entries }
    }

    /// Push a completed output; evicts the oldest entry when the buffer is full.
    pub fn push(&mut self, output: StreamingOutput<T>) {
        if self.entries.len() == self.max_entries {
            self.entries.pop_front();
        }
        self.entries.push_back(output);
    }

    /// Returns a reference to the most recently pushed output, if any.
    pub fn latest(&self) -> Option<&StreamingOutput<T>> {
        self.entries.back()
    }

    /// Number of outputs currently held in the buffer.
    pub fn len(&self) -> usize {
        self.entries.len()
    }
}

#[cfg(test)]
mod streaming_result_tests {
    use super::*;

    #[test]
    fn partial_result_new() {
        let r = PartialResult::new(0, 3, "hello");
        assert_eq!(r.chunk_index, 0);
        assert_eq!(r.total_chunks, 3);
        assert_eq!(r.data, "hello");
    }

    #[test]
    fn partial_result_is_last() {
        let not_last = PartialResult::new(1, 3, 0u32);
        assert!(!not_last.is_last());

        let last = PartialResult::new(2, 3, 0u32);
        assert!(last.is_last());
    }

    #[test]
    fn partial_result_progress() {
        let r = PartialResult::new(1, 4, ());
        let expected = 2.0_f32 / 4.0_f32;
        assert!((r.progress() - expected).abs() < f32::EPSILON);
    }

    #[test]
    fn streaming_output_push_chunk() {
        let mut out: StreamingOutput<u32> = StreamingOutput::new(3);
        out.push_chunk(PartialResult::new(0, 3, 10));
        assert_eq!(out.chunks.len(), 1);
        out.push_chunk(PartialResult::new(1, 3, 20));
        assert_eq!(out.chunks.len(), 2);
    }

    #[test]
    fn streaming_output_is_complete_after_all_chunks() {
        let mut out: StreamingOutput<i32> = StreamingOutput::new(2);
        assert!(!out.is_complete());
        out.push_chunk(PartialResult::new(0, 2, 1));
        assert!(!out.is_complete());
        out.push_chunk(PartialResult::new(1, 2, 2));
        assert!(out.is_complete());
    }

    #[test]
    fn streaming_output_collect() {
        let mut out: StreamingOutput<&str> = StreamingOutput::new(3);
        out.push_chunk(PartialResult::new(0, 3, "a"));
        out.push_chunk(PartialResult::new(1, 3, "b"));
        out.push_chunk(PartialResult::new(2, 3, "c"));
        assert_eq!(out.collect(), vec!["a", "b", "c"]);
    }

    #[test]
    fn result_buffer_len() {
        let mut buf: ResultBuffer<u8> = ResultBuffer::new(5);
        assert_eq!(buf.len(), 0);
        buf.push(StreamingOutput::new(1));
        assert_eq!(buf.len(), 1);
        buf.push(StreamingOutput::new(1));
        assert_eq!(buf.len(), 2);
    }

    #[test]
    fn result_buffer_latest() {
        let mut buf: ResultBuffer<u8> = ResultBuffer::new(5);
        assert!(buf.latest().is_none());

        let mut first = StreamingOutput::new(1);
        first.push_chunk(PartialResult::new(0, 1, 7u8));
        buf.push(first);

        let mut second = StreamingOutput::new(1);
        second.push_chunk(PartialResult::new(0, 1, 42u8));
        buf.push(second);

        let latest = buf.latest().expect("should have an entry");
        assert_eq!(latest.chunks[0], 42u8);
    }

    #[test]
    fn result_buffer_push_evicts_when_full() {
        let mut buf: ResultBuffer<u32> = ResultBuffer::new(3);
        for i in 0..4u32 {
            let mut out = StreamingOutput::new(1);
            out.push_chunk(PartialResult::new(0, 1, i));
            buf.push(out);
        }
        // Buffer capped at 3; oldest (0) should have been evicted.
        assert_eq!(buf.len(), 3);
        // The front entry should now be the second-inserted (value=1).
        assert_eq!(buf.entries.front().unwrap().chunks[0], 1u32);
        // The latest should be the last-inserted (value=3).
        assert_eq!(buf.latest().unwrap().chunks[0], 3u32);
    }
}
