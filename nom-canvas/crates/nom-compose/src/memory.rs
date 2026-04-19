#![deny(unsafe_code)]

use std::collections::HashMap;

/// A single memory entry with verbatim content.
pub struct MemoryEntry {
    pub timestamp: u64,
    pub role: String,
    pub content: String,
    pub token_count: usize,
}

impl MemoryEntry {
    pub fn new(timestamp: u64, role: impl Into<String>, content: impl Into<String>) -> Self {
        let content = content.into();
        let token_count = Self::estimate_tokens(&content);
        Self {
            timestamp,
            role: role.into(),
            content,
            token_count,
        }
    }

    /// Simple token estimator: split on whitespace.
    fn estimate_tokens(text: &str) -> usize {
        text.split_whitespace().count().max(1)
    }
}

/// Verbatim memory storage with no summarization loss.
pub struct VerbatimMemory {
    entries: Vec<MemoryEntry>,
    max_tokens: usize,
    current_tokens: usize,
}

impl VerbatimMemory {
    pub fn new(max_tokens: usize) -> Self {
        Self {
            entries: Vec::new(),
            max_tokens,
            current_tokens: 0,
        }
    }

    /// Add an entry to memory, evicting oldest if over token budget.
    pub fn add(&mut self, entry: MemoryEntry) {
        self.current_tokens += entry.token_count;
        self.entries.push(entry);

        // Evict oldest entries if over budget.
        while self.current_tokens > self.max_tokens && !self.entries.is_empty() {
            if let Some(oldest) = self.entries.first() {
                self.current_tokens -= oldest.token_count;
            }
            self.entries.remove(0);
        }
    }

    /// Retrieve the top-k entries most relevant to the query.
    pub fn retrieve_relevant(&self, query: &str, top_k: usize) -> Vec<&MemoryEntry> {
        let query_lower = query.to_lowercase();
        let mut scored: Vec<(f32, &MemoryEntry)> = self
            .entries
            .iter()
            .map(|entry| {
                let entry_lower = entry.content.to_lowercase();
                let score = if entry_lower.contains(&query_lower) {
                    1.0
                } else {
                    let query_words: Vec<&str> = query_lower.split_whitespace().collect();
                    let entry_words: Vec<&str> = entry_lower.split_whitespace().collect();
                    let matches = query_words
                        .iter()
                        .filter(|w| entry_words.contains(w))
                        .count();
                    matches as f32 / query_words.len().max(1) as f32
                };
                (score, entry)
            })
            .filter(|(score, _)| *score > 0.0)
            .collect();

        scored.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));
        scored.into_iter().take(top_k).map(|(_, entry)| entry).collect()
    }

    /// Hybrid search combining BM25 keyword scoring with sparse embedding cosine
    /// similarity fused via reciprocal rank fusion (RRF).
    pub fn search_hybrid(&self, query: &str, top_k: usize) -> Vec<&MemoryEntry> {
        let retriever = HybridRetriever::new();
        let fused = retriever.search(&self.entries, query, top_k);
        fused
            .into_iter()
            .filter_map(|(idx, _)| self.entries.get(idx))
            .collect()
    }

    /// Total token count currently stored.
    pub fn token_count(&self) -> usize {
        self.current_tokens
    }

    /// Number of entries stored.
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}

// ---------------------------------------------------------------------------
// Embedding store
// ---------------------------------------------------------------------------

/// Trait for storing and searching text embeddings.
pub trait EmbeddingStore {
    /// Add a document with the given ID and text.
    fn add(&mut self, id: usize, text: &str);
    /// Search for the top-k most similar documents to the query text.
    fn search(&self, query: &str, top_k: usize) -> Vec<(usize, f32)>;
}

/// A sparse vector represented as token -> weight map.
#[derive(Debug, Clone)]
struct SparseVector {
    weights: HashMap<String, f32>,
    norm: f32,
}

impl SparseVector {
    fn from_text(text: &str) -> Self {
        let mut weights = HashMap::new();
        for token in text.split_whitespace().map(|t| t.to_lowercase()) {
            *weights.entry(token).or_insert(0.0) += 1.0;
        }
        let norm = weights.values().map(|v| v * v).sum::<f32>().sqrt();
        Self { weights, norm }
    }

    /// Cosine similarity between two sparse vectors.
    fn cosine_similarity(&self, other: &Self) -> f32 {
        if self.norm == 0.0 || other.norm == 0.0 {
            return 0.0;
        }
        let (small, large) = if self.weights.len() < other.weights.len() {
            (&self.weights, &other.weights)
        } else {
            (&other.weights, &self.weights)
        };
        let mut dot = 0.0f32;
        for (token, weight) in small {
            if let Some(&other_weight) = large.get(token) {
                dot += weight * other_weight;
            }
        }
        dot / (self.norm * other.norm)
    }
}

/// Simple in-memory embedding store using token-frequency sparse vectors
/// and cosine similarity.
pub struct InMemoryEmbeddingStore {
    docs: Vec<(usize, SparseVector)>,
}

impl InMemoryEmbeddingStore {
    pub fn new() -> Self {
        Self { docs: Vec::new() }
    }
}

impl EmbeddingStore for InMemoryEmbeddingStore {
    fn add(&mut self, id: usize, text: &str) {
        self.docs.push((id, SparseVector::from_text(text)));
    }

    fn search(&self, query: &str, top_k: usize) -> Vec<(usize, f32)> {
        let query_vec = SparseVector::from_text(query);
        let mut scored: Vec<(usize, f32)> = self
            .docs
            .iter()
            .map(|(id, vec)| (*id, vec.cosine_similarity(&query_vec)))
            .filter(|(_, score)| *score > 0.0)
            .collect();
        scored.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        scored.truncate(top_k);
        scored
    }
}

impl Default for InMemoryEmbeddingStore {
    fn default() -> Self {
        Self::new()
    }
}

// ---------------------------------------------------------------------------
// Hybrid retriever
// ---------------------------------------------------------------------------

/// Hybrid retriever that fuses BM25 keyword search with embedding cosine
/// similarity via reciprocal rank fusion (RRF).
pub struct HybridRetriever {
    /// RRF constant k (default 60).
    pub rrf_k: u32,
}

impl HybridRetriever {
    pub fn new() -> Self {
        Self { rrf_k: 60 }
    }

    /// Search across entries using BM25 + embedding similarity fused with RRF.
    ///
    /// Returns `(entry_index, rrf_score)` tuples sorted by fused score
    /// descending.
    pub fn search(
        &self,
        entries: &[MemoryEntry],
        query: &str,
        top_k: usize,
    ) -> Vec<(usize, f32)> {
        if entries.is_empty() || query.trim().is_empty() {
            return Vec::new();
        }

        // --- BM25 ranks ---
        let mut bm25 = nom_canvas_intent::BM25Retriever::new();
        for (i, entry) in entries.iter().enumerate() {
            bm25.add_document(&i.to_string(), &entry.content);
        }
        let bm25_results = bm25.retrieve(query, entries.len());
        let bm25_ranks: Vec<u64> = bm25_results
            .iter()
            .filter(|(_, score)| *score > 0.0)
            .map(|(doc, _)| doc.id.parse::<u64>().unwrap())
            .collect();

        // --- Embedding ranks ---
        let mut embed_store = InMemoryEmbeddingStore::new();
        for (i, entry) in entries.iter().enumerate() {
            embed_store.add(i, &entry.content);
        }
        let embed_results = embed_store.search(query, entries.len());
        let embed_ranks: Vec<u64> = embed_results.iter().map(|(id, _)| *id as u64).collect();

        // --- RRF fusion ---
        let fused = nom_canvas_intent::HybridRetriever::reciprocal_rank_fusion(
            &bm25_ranks,
            &embed_ranks,
            self.rrf_k,
        );

        fused
            .into_iter()
            .take(top_k)
            .map(|(id, score)| (id as usize, score))
            .collect()
    }
}

impl Default for HybridRetriever {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_memory_add_and_count() {
        let mut mem = VerbatimMemory::new(100);
        mem.add(MemoryEntry::new(1, "user", "hello world"));
        assert_eq!(mem.token_count(), 2);
        assert_eq!(mem.len(), 1);
    }

    #[test]
    fn test_memory_retrieve_relevant() {
        let mut mem = VerbatimMemory::new(100);
        mem.add(MemoryEntry::new(1, "user", "hello world"));
        mem.add(MemoryEntry::new(2, "assistant", "goodbye universe"));
        let relevant = mem.retrieve_relevant("hello", 1);
        assert_eq!(relevant.len(), 1);
        assert_eq!(relevant[0].content, "hello world");
    }

    #[test]
    fn test_memory_max_tokens() {
        let mut mem = VerbatimMemory::new(3);
        mem.add(MemoryEntry::new(1, "user", "one two"));
        mem.add(MemoryEntry::new(2, "user", "three four"));
        // Should evict oldest to stay under budget.
        assert!(mem.token_count() <= 3);
        assert!(mem.len() <= 2);
    }

    #[test]
    fn test_memory_empty_retrieve() {
        let mem = VerbatimMemory::new(100);
        let relevant = mem.retrieve_relevant("anything", 5);
        assert!(relevant.is_empty());
    }

    #[test]
    fn test_memory_timestamp_order() {
        let mut mem = VerbatimMemory::new(100);
        mem.add(MemoryEntry::new(3, "user", "third"));
        mem.add(MemoryEntry::new(1, "user", "first"));
        mem.add(MemoryEntry::new(2, "user", "second"));
        assert_eq!(mem.entries[0].timestamp, 3);
        assert_eq!(mem.entries[1].timestamp, 1);
        assert_eq!(mem.entries[2].timestamp, 2);
    }

    #[test]
    fn test_memory_retrieve_top_k() {
        let mut mem = VerbatimMemory::new(100);
        mem.add(MemoryEntry::new(1, "user", "rust programming"));
        mem.add(MemoryEntry::new(2, "user", "rust language"));
        mem.add(MemoryEntry::new(3, "user", "python code"));
        let relevant = mem.retrieve_relevant("rust", 2);
        assert_eq!(relevant.len(), 2);
    }

    #[test]
    fn test_memory_eviction_order() {
        let mut mem = VerbatimMemory::new(4);
        mem.add(MemoryEntry::new(1, "user", "one two")); // 2 tokens
        mem.add(MemoryEntry::new(2, "user", "three four")); // 2 tokens, total 4
        mem.add(MemoryEntry::new(3, "user", "five")); // 1 token, total 5, evict oldest
        assert_eq!(mem.len(), 2);
        assert_eq!(mem.entries[0].timestamp, 2);
        assert_eq!(mem.entries[1].timestamp, 3);
    }

    // -----------------------------------------------------------------------
    // Embedding store tests
    // -----------------------------------------------------------------------

    #[test]
    fn embedding_store_empty_returns_empty() {
        let store = InMemoryEmbeddingStore::new();
        let results = store.search("hello", 5);
        assert!(results.is_empty());
    }

    #[test]
    fn embedding_store_exact_match_scores_one() {
        let mut store = InMemoryEmbeddingStore::new();
        store.add(0, "hello world");
        let results = store.search("hello world", 5);
        assert_eq!(results.len(), 1);
        assert!((results[0].1 - 1.0).abs() < 1e-6);
    }

    #[test]
    fn embedding_store_partial_match() {
        let mut store = InMemoryEmbeddingStore::new();
        store.add(0, "hello world foo bar");
        store.add(1, "goodbye universe");
        let results = store.search("hello world", 5);
        // Both may match depending on overlap; id 0 should rank first
        assert!(!results.is_empty());
        assert_eq!(results[0].0, 0);
    }

    #[test]
    fn embedding_store_top_k_truncates() {
        let mut store = InMemoryEmbeddingStore::new();
        store.add(0, "a b c d e");
        store.add(1, "a b c");
        store.add(2, "x y z");
        let results = store.search("a b c", 2);
        assert_eq!(results.len(), 2);
    }

    // -----------------------------------------------------------------------
    // Hybrid retriever tests
    // -----------------------------------------------------------------------

    #[test]
    fn hybrid_empty_entries_returns_empty() {
        let retriever = HybridRetriever::new();
        let entries: Vec<MemoryEntry> = vec![];
        let results = retriever.search(&entries, "query", 5);
        assert!(results.is_empty());
    }

    #[test]
    fn hybrid_empty_query_returns_empty() {
        let retriever = HybridRetriever::new();
        let entries = vec![MemoryEntry::new(1, "user", "hello world")];
        let results = retriever.search(&entries, "", 5);
        assert!(results.is_empty());
    }

    #[test]
    fn hybrid_finds_keyword_match() {
        let retriever = HybridRetriever::new();
        let entries = vec![
            MemoryEntry::new(1, "user", "the quick brown fox"),
            MemoryEntry::new(2, "user", "rust programming language"),
        ];
        let results = retriever.search(&entries, "rust", 5);
        assert!(!results.is_empty());
        assert_eq!(results[0].0, 1); // index 1 contains "rust"
    }

    #[test]
    fn hybrid_fuses_signals_rrf() {
        // Set up a case where both BM25 and embedding contribute.
        let retriever = HybridRetriever::new();
        let entries = vec![
            MemoryEntry::new(1, "user", "quick brown fox jumps"),
            MemoryEntry::new(2, "user", "slow red dog sleeps"),
            MemoryEntry::new(3, "user", "the quick foxes run"),
        ];
        let results = retriever.search(&entries, "quick fox", 3);
        // Entry 0 and 2 both contain quick/fox variants.
        // Both BM25 and embedding should rank them top; RRF should keep them top.
        assert!(!results.is_empty());
        let top_ids: Vec<usize> = results.iter().map(|(id, _)| *id).collect();
        // The exact order depends on BM25 vs embedding scores, but entries 0 and 2
        // should dominate.
        assert!(
            top_ids.contains(&0) || top_ids.contains(&2),
            "top results should contain relevant entries"
        );
    }

    #[test]
    fn hybrid_respects_top_k() {
        let retriever = HybridRetriever::new();
        let entries = vec![
            MemoryEntry::new(1, "user", "alpha beta gamma"),
            MemoryEntry::new(2, "user", "alpha beta delta"),
            MemoryEntry::new(3, "user", "alpha beta epsilon"),
        ];
        let results = retriever.search(&entries, "alpha", 2);
        assert_eq!(results.len(), 2);
    }

    // -----------------------------------------------------------------------
    // VerbatimMemory::search_hybrid integration tests
    // -----------------------------------------------------------------------

    #[test]
    fn memory_search_hybrid_empty() {
        let mem = VerbatimMemory::new(100);
        let results = mem.search_hybrid("anything", 5);
        assert!(results.is_empty());
    }

    #[test]
    fn memory_search_hybrid_basic() {
        let mut mem = VerbatimMemory::new(100);
        mem.add(MemoryEntry::new(1, "user", "hello world"));
        mem.add(MemoryEntry::new(2, "user", "goodbye universe"));
        let results = mem.search_hybrid("hello", 5);
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].content, "hello world");
    }

    #[test]
    fn memory_search_hybrid_top_k() {
        let mut mem = VerbatimMemory::new(100);
        mem.add(MemoryEntry::new(1, "user", "rust programming"));
        mem.add(MemoryEntry::new(2, "user", "rust language"));
        mem.add(MemoryEntry::new(3, "user", "python code"));
        let results = mem.search_hybrid("rust", 2);
        assert_eq!(results.len(), 2);
    }

    #[test]
    fn memory_search_hybrid_fusion_boosts_shared_match() {
        // An entry that matches both BM25 and embedding should outrank an
        // entry that only matches one signal.
        let mut mem = VerbatimMemory::new(100);
        mem.add(MemoryEntry::new(1, "user", "the quick brown fox"));
        mem.add(MemoryEntry::new(2, "user", "foxes are quick animals"));
        mem.add(MemoryEntry::new(3, "user", "slow turtle walks"));
        let results = mem.search_hybrid("quick fox", 3);
        assert_eq!(results.len(), 2); // only entries 1 and 2 are relevant
        // Entry 1 has exact tokens "quick" and "fox" so it should rank first
        assert_eq!(results[0].content, "the quick brown fox");
    }
}
