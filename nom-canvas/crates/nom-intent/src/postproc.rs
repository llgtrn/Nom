/// LlamaIndex-style post-processor pattern for retrieved documents.
///
/// Provides filtering and deduplication stages that run after retrieval,
/// before documents are passed to synthesis.

/// A retrieved document with an associated relevance score.
#[derive(Debug, Clone)]
pub struct PostDoc {
    pub id: u64,
    pub content: String,
    pub score: f32,
}

impl PostDoc {
    pub fn new(id: u64, content: impl Into<String>, score: f32) -> Self {
        Self {
            id,
            content: content.into(),
            score,
        }
    }

    /// FNV-1a 64-bit hash of the content bytes.
    pub fn content_hash(&self) -> u64 {
        const OFFSET: u64 = 14695981039346656037;
        const PRIME: u64 = 1099511628211;
        let mut hash = OFFSET;
        for byte in self.content.as_bytes() {
            hash ^= *byte as u64;
            hash = hash.wrapping_mul(PRIME);
        }
        hash
    }
}

/// Removes duplicate documents by content hash, keeping the first occurrence.
pub struct DeduplicateFilter;

impl DeduplicateFilter {
    pub fn new() -> Self {
        Self
    }

    pub fn apply(&self, docs: Vec<PostDoc>) -> Vec<PostDoc> {
        let mut seen = std::collections::HashSet::new();
        docs.into_iter()
            .filter(|doc| seen.insert(doc.content_hash()))
            .collect()
    }

    /// Returns the number of duplicate documents (total - unique).
    pub fn duplicate_count(&self, docs: &[PostDoc]) -> usize {
        let unique: std::collections::HashSet<u64> =
            docs.iter().map(|d| d.content_hash()).collect();
        docs.len() - unique.len()
    }
}

impl Default for DeduplicateFilter {
    fn default() -> Self {
        Self::new()
    }
}

/// Removes documents whose score falls below a minimum threshold.
pub struct ScoreThresholdFilter {
    pub min_score: f32,
}

impl ScoreThresholdFilter {
    pub fn new(min_score: f32) -> Self {
        Self { min_score }
    }

    pub fn apply(&self, docs: Vec<PostDoc>) -> Vec<PostDoc> {
        docs.into_iter()
            .filter(|doc| doc.score >= self.min_score)
            .collect()
    }

    /// Returns true if this document would be filtered out.
    pub fn would_filter(&self, doc: &PostDoc) -> bool {
        doc.score < self.min_score
    }
}

/// A two-stage post-processing pipeline: score threshold first, then deduplication.
pub struct PostPipeline {
    pub threshold: ScoreThresholdFilter,
    pub dedup: DeduplicateFilter,
}

impl PostPipeline {
    pub fn new(min_score: f32) -> Self {
        Self {
            threshold: ScoreThresholdFilter::new(min_score),
            dedup: DeduplicateFilter::new(),
        }
    }

    pub fn process(&self, docs: Vec<PostDoc>) -> Vec<PostDoc> {
        let after_threshold = self.threshold.apply(docs);
        self.dedup.apply(after_threshold)
    }
}

#[cfg(test)]
mod postproc_tests {
    use super::*;

    #[test]
    fn post_doc_content_hash_deterministic() {
        let doc = PostDoc::new(1, "hello world", 0.9);
        assert_eq!(doc.content_hash(), doc.content_hash());
    }

    #[test]
    fn post_doc_different_content_different_hash() {
        let doc_a = PostDoc::new(1, "alpha", 0.8);
        let doc_b = PostDoc::new(2, "beta", 0.8);
        assert_ne!(doc_a.content_hash(), doc_b.content_hash());
    }

    #[test]
    fn deduplicate_filter_removes_duplicates() {
        let filter = DeduplicateFilter::new();
        let docs = vec![
            PostDoc::new(1, "same content", 0.9),
            PostDoc::new(2, "same content", 0.8),
            PostDoc::new(3, "different content", 0.7),
        ];
        let result = filter.apply(docs);
        assert_eq!(result.len(), 2);
        assert_eq!(result[0].id, 1);
        assert_eq!(result[1].id, 3);
    }

    #[test]
    fn deduplicate_filter_keeps_unique() {
        let filter = DeduplicateFilter::new();
        let docs = vec![
            PostDoc::new(1, "alpha", 0.9),
            PostDoc::new(2, "beta", 0.8),
            PostDoc::new(3, "gamma", 0.7),
        ];
        let result = filter.apply(docs);
        assert_eq!(result.len(), 3);
    }

    #[test]
    fn deduplicate_filter_duplicate_count() {
        let filter = DeduplicateFilter::new();
        let docs = vec![
            PostDoc::new(1, "dup", 0.9),
            PostDoc::new(2, "dup", 0.8),
            PostDoc::new(3, "dup", 0.7),
            PostDoc::new(4, "unique", 0.6),
        ];
        assert_eq!(filter.duplicate_count(&docs), 2);
    }

    #[test]
    fn score_threshold_filter_filters_below() {
        let filter = ScoreThresholdFilter::new(0.5);
        let docs = vec![
            PostDoc::new(1, "a", 0.3),
            PostDoc::new(2, "b", 0.6),
            PostDoc::new(3, "c", 0.1),
        ];
        let result = filter.apply(docs);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].id, 2);
    }

    #[test]
    fn score_threshold_filter_keeps_above() {
        let filter = ScoreThresholdFilter::new(0.5);
        let docs = vec![
            PostDoc::new(1, "a", 0.5),
            PostDoc::new(2, "b", 0.9),
            PostDoc::new(3, "c", 1.0),
        ];
        let result = filter.apply(docs);
        assert_eq!(result.len(), 3);
    }

    #[test]
    fn score_threshold_filter_would_filter() {
        let filter = ScoreThresholdFilter::new(0.5);
        let low = PostDoc::new(1, "low", 0.3);
        let high = PostDoc::new(2, "high", 0.7);
        assert!(filter.would_filter(&low));
        assert!(!filter.would_filter(&high));
    }

    #[test]
    fn post_pipeline_process_combined() {
        let pipeline = PostPipeline::new(0.5);
        let docs = vec![
            PostDoc::new(1, "keep unique", 0.9),
            PostDoc::new(2, "filter out", 0.2),       // below threshold
            PostDoc::new(3, "keep unique", 0.8),       // dup of id=1, removed after threshold
            PostDoc::new(4, "also keep", 0.6),
        ];
        let result = pipeline.process(docs);
        // After threshold (>=0.5): ids 1, 3, 4
        // After dedup: id=1 (first "keep unique"), id=4
        assert_eq!(result.len(), 2);
        assert_eq!(result[0].id, 1);
        assert_eq!(result[1].id, 4);
    }
}
