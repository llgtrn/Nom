/// Quality level for an ingested entry.
#[derive(Debug, Clone, PartialEq)]
pub enum IngestQuality {
    BodyOnly,
    Partial,
    Complete,
}

impl IngestQuality {
    pub fn quality_score(&self) -> f32 {
        match self {
            IngestQuality::BodyOnly => 0.3,
            IngestQuality::Partial => 0.6,
            IngestQuality::Complete => 1.0,
        }
    }

    pub fn can_promote(&self) -> bool {
        matches!(self, IngestQuality::BodyOnly | IngestQuality::Partial)
    }
}

/// A partially-ingested entry with a hash, body text, and quality level.
#[derive(Debug, Clone)]
pub struct PartialEntry {
    pub hash: u64,
    pub body: String,
    pub quality: IngestQuality,
}

impl PartialEntry {
    pub fn new(hash: u64, body: impl Into<String>, quality: IngestQuality) -> Self {
        Self {
            hash,
            body: body.into(),
            quality,
        }
    }

    pub fn word_count(&self) -> usize {
        self.body.split_whitespace().count()
    }
}

/// Promotes `PartialEntry` values to `Complete` when they meet a minimum word count.
pub struct IngestPromoter {
    pub min_word_count: usize,
}

impl IngestPromoter {
    pub fn new(min_word_count: usize) -> Self {
        Self { min_word_count }
    }

    pub fn should_promote(&self, entry: &PartialEntry) -> bool {
        entry.word_count() >= self.min_word_count && entry.quality.can_promote()
    }

    pub fn promote(entry: PartialEntry) -> PartialEntry {
        PartialEntry {
            hash: entry.hash,
            body: entry.body,
            quality: IngestQuality::Complete,
        }
    }

    pub fn batch_promote(&self, entries: Vec<PartialEntry>) -> (Vec<PartialEntry>, usize) {
        let mut count_promoted = 0usize;
        let promoted: Vec<PartialEntry> = entries
            .into_iter()
            .map(|e| {
                if self.should_promote(&e) {
                    count_promoted += 1;
                    Self::promote(e)
                } else {
                    e
                }
            })
            .collect();
        (promoted, count_promoted)
    }
}

/// Ingests raw body strings as `BodyOnly` entries.
pub struct BodyIngestor;

impl Default for BodyIngestor {
    fn default() -> Self {
        Self::new()
    }
}

impl BodyIngestor {
    pub fn new() -> Self {
        Self
    }

    pub fn ingest_raw(body: &str, base_hash: u64) -> PartialEntry {
        PartialEntry::new(base_hash, body, IngestQuality::BodyOnly)
    }

    pub fn ingest_batch(bodies: &[&str], start_hash: u64) -> Vec<PartialEntry> {
        bodies
            .iter()
            .enumerate()
            .map(|(i, body)| Self::ingest_raw(body, start_hash + i as u64))
            .collect()
    }
}

#[cfg(test)]
mod ingest_partial_tests {
    use super::*;

    #[test]
    fn test_quality_score() {
        assert!((IngestQuality::BodyOnly.quality_score() - 0.3).abs() < f32::EPSILON);
        assert!((IngestQuality::Partial.quality_score() - 0.6).abs() < f32::EPSILON);
        assert!((IngestQuality::Complete.quality_score() - 1.0).abs() < f32::EPSILON);
    }

    #[test]
    fn test_can_promote() {
        assert!(IngestQuality::BodyOnly.can_promote());
        assert!(IngestQuality::Partial.can_promote());
        assert!(!IngestQuality::Complete.can_promote());
    }

    #[test]
    fn test_word_count() {
        let entry = PartialEntry::new(1, "hello world foo bar", IngestQuality::BodyOnly);
        assert_eq!(entry.word_count(), 4);
    }

    #[test]
    fn test_should_promote_true_when_enough_words() {
        let promoter = IngestPromoter::new(3);
        let entry = PartialEntry::new(1, "one two three four", IngestQuality::Partial);
        assert!(promoter.should_promote(&entry));
    }

    #[test]
    fn test_should_promote_false_when_too_few_words() {
        let promoter = IngestPromoter::new(10);
        let entry = PartialEntry::new(1, "only two words", IngestQuality::Partial);
        assert!(!promoter.should_promote(&entry));
    }

    #[test]
    fn test_promote_sets_quality_complete() {
        let entry = PartialEntry::new(42, "some body text", IngestQuality::BodyOnly);
        let promoted = IngestPromoter::promote(entry);
        assert_eq!(promoted.quality, IngestQuality::Complete);
    }

    #[test]
    fn test_batch_promote_count_matches_eligible() {
        let promoter = IngestPromoter::new(3);
        let entries = vec![
            PartialEntry::new(1, "one two three four", IngestQuality::Partial),   // eligible
            PartialEntry::new(2, "short", IngestQuality::Partial),                // not eligible
            PartialEntry::new(3, "alpha beta gamma delta", IngestQuality::BodyOnly), // eligible
        ];
        let (result, count) = promoter.batch_promote(entries);
        assert_eq!(count, 2);
        assert_eq!(result[0].quality, IngestQuality::Complete);
        assert_eq!(result[1].quality, IngestQuality::Partial);
        assert_eq!(result[2].quality, IngestQuality::Complete);
    }

    #[test]
    fn test_ingest_raw_produces_body_only() {
        let entry = BodyIngestor::ingest_raw("hello world", 99);
        assert_eq!(entry.hash, 99);
        assert_eq!(entry.body, "hello world");
        assert_eq!(entry.quality, IngestQuality::BodyOnly);
    }

    #[test]
    fn test_ingest_batch_produces_correct_count() {
        let bodies = ["first entry", "second entry", "third entry"];
        let entries = BodyIngestor::ingest_batch(&bodies, 10);
        assert_eq!(entries.len(), 3);
        assert_eq!(entries[0].hash, 10);
        assert_eq!(entries[1].hash, 11);
        assert_eq!(entries[2].hash, 12);
        for e in &entries {
            assert_eq!(e.quality, IngestQuality::BodyOnly);
        }
    }
}
