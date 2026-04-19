//! Delta compression primitives for incremental graph state updates.

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DeltaKind {
    Insert,
    Delete,
    Replace,
    Noop,
}

impl DeltaKind {
    pub fn is_mutation(&self) -> bool {
        matches!(self, DeltaKind::Insert | DeltaKind::Delete | DeltaKind::Replace)
    }

    pub fn code(&self) -> u8 {
        match self {
            DeltaKind::Insert => 1,
            DeltaKind::Delete => 2,
            DeltaKind::Replace => 3,
            DeltaKind::Noop => 0,
        }
    }
}

#[derive(Debug, Clone)]
pub struct DeltaFrame {
    pub kind: DeltaKind,
    pub offset: usize,
    pub data: Vec<u8>,
}

impl DeltaFrame {
    pub fn byte_size(&self) -> usize {
        self.data.len()
    }

    pub fn is_empty(&self) -> bool {
        self.data.is_empty() || self.kind == DeltaKind::Noop
    }
}

#[derive(Debug, Clone, Default)]
pub struct DeltaEncoder {
    pub frames: Vec<DeltaFrame>,
}

impl DeltaEncoder {
    pub fn push(&mut self, f: DeltaFrame) {
        self.frames.push(f);
    }

    pub fn total_bytes(&self) -> usize {
        self.frames.iter().map(|f| f.byte_size()).sum()
    }

    pub fn mutation_count(&self) -> usize {
        self.frames.iter().filter(|f| f.kind.is_mutation()).count()
    }

    pub fn encode_summary(&self) -> String {
        format!(
            "frames:{} mutations:{} bytes:{}",
            self.frames.len(),
            self.mutation_count(),
            self.total_bytes()
        )
    }
}

#[derive(Debug, Clone)]
pub struct DeltaDecoder {
    pub max_frame_size: usize,
}

impl DeltaDecoder {
    pub fn new(max_frame_size: usize) -> Self {
        Self { max_frame_size }
    }

    pub fn validate_frame(&self, f: &DeltaFrame) -> bool {
        f.byte_size() <= self.max_frame_size
    }

    pub fn decode_all<'a>(&self, frames: &'a [DeltaFrame]) -> Vec<&'a DeltaFrame> {
        frames.iter().filter(|f| self.validate_frame(f)).collect()
    }
}

#[derive(Debug, Clone)]
pub struct DeltaStream {
    pub encoder: DeltaEncoder,
    pub sequence: u64,
}

impl DeltaStream {
    pub fn new() -> Self {
        Self {
            encoder: DeltaEncoder::default(),
            sequence: 0,
        }
    }

    pub fn emit(&mut self, f: DeltaFrame) {
        self.encoder.push(f);
        self.sequence += 1;
    }

    pub fn sequence_id(&self) -> u64 {
        self.sequence
    }

    pub fn flush(&mut self) -> Vec<DeltaFrame> {
        std::mem::take(&mut self.encoder.frames)
    }
}

impl Default for DeltaStream {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod delta_compress_tests {
    use super::*;

    #[test]
    fn kind_is_mutation_noop_false() {
        assert!(!DeltaKind::Noop.is_mutation());
    }

    #[test]
    fn kind_code_replace_is_3() {
        assert_eq!(DeltaKind::Replace.code(), 3);
    }

    #[test]
    fn frame_byte_size() {
        let f = DeltaFrame { kind: DeltaKind::Insert, offset: 0, data: vec![1, 2, 3] };
        assert_eq!(f.byte_size(), 3);
    }

    #[test]
    fn frame_is_empty_noop() {
        let f = DeltaFrame { kind: DeltaKind::Noop, offset: 0, data: vec![1, 2, 3] };
        assert!(f.is_empty());
    }

    #[test]
    fn encoder_total_bytes() {
        let mut enc = DeltaEncoder::default();
        enc.push(DeltaFrame { kind: DeltaKind::Insert, offset: 0, data: vec![1, 2] });
        enc.push(DeltaFrame { kind: DeltaKind::Replace, offset: 2, data: vec![3, 4, 5] });
        assert_eq!(enc.total_bytes(), 5);
    }

    #[test]
    fn encoder_mutation_count() {
        let mut enc = DeltaEncoder::default();
        enc.push(DeltaFrame { kind: DeltaKind::Insert, offset: 0, data: vec![1] });
        enc.push(DeltaFrame { kind: DeltaKind::Noop, offset: 1, data: vec![] });
        enc.push(DeltaFrame { kind: DeltaKind::Delete, offset: 2, data: vec![2] });
        assert_eq!(enc.mutation_count(), 2);
    }

    #[test]
    fn encoder_encode_summary_format() {
        let mut enc = DeltaEncoder::default();
        enc.push(DeltaFrame { kind: DeltaKind::Insert, offset: 0, data: vec![10, 20] });
        enc.push(DeltaFrame { kind: DeltaKind::Noop, offset: 2, data: vec![] });
        let s = enc.encode_summary();
        assert_eq!(s, "frames:2 mutations:1 bytes:2");
    }

    #[test]
    fn decoder_validate_frame_true_and_false() {
        let dec = DeltaDecoder::new(4);
        let small = DeltaFrame { kind: DeltaKind::Insert, offset: 0, data: vec![1, 2, 3, 4] };
        let large = DeltaFrame { kind: DeltaKind::Insert, offset: 0, data: vec![1, 2, 3, 4, 5] };
        assert!(dec.validate_frame(&small));
        assert!(!dec.validate_frame(&large));
    }

    #[test]
    fn stream_emit_increments_sequence() {
        let mut stream = DeltaStream::new();
        assert_eq!(stream.sequence_id(), 0);
        stream.emit(DeltaFrame { kind: DeltaKind::Insert, offset: 0, data: vec![1] });
        stream.emit(DeltaFrame { kind: DeltaKind::Delete, offset: 1, data: vec![2] });
        assert_eq!(stream.sequence_id(), 2);
    }

    #[test]
    fn stream_flush_empties_encoder() {
        let mut stream = DeltaStream::new();
        stream.emit(DeltaFrame { kind: DeltaKind::Replace, offset: 0, data: vec![0xAB] });
        let flushed = stream.flush();
        assert_eq!(flushed.len(), 1);
        assert!(stream.encoder.frames.is_empty());
    }
}
