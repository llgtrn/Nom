#![deny(unsafe_code)]

use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::Instant;

/// Lifecycle status of a cached glue blueprint.
#[derive(Debug, Clone, PartialEq)]
pub enum GlueStatus {
    /// Inserted but not yet used enough to promote.
    Transient,
    /// Used at least `promotion_threshold` times; ready for DB promotion.
    Partial,
    /// Confirmed complete; promoted to persistent storage.
    Complete,
}

/// A cached AI-generated glue blueprint entry.
#[derive(Debug, Clone)]
pub struct CachedGlue {
    /// sha256-like hash of the .nomx source.
    pub hash: String,
    pub kind: String,
    pub nomx_source: String,
    pub status: GlueStatus,
    pub use_count: u32,
    pub created_at: Instant,
}

/// In-memory cache tracking AI-generated glue blueprints with a
/// Transient → Partial → Complete lifecycle.
pub struct GlueCache {
    entries: Arc<Mutex<HashMap<String, CachedGlue>>>,
    /// Number of uses before a Transient entry is promoted to Partial.
    promotion_threshold: u32,
}

impl GlueCache {
    /// Create a new cache with the given promotion threshold.
    pub fn new(promotion_threshold: u32) -> Self {
        Self {
            entries: Arc::new(Mutex::new(HashMap::new())),
            promotion_threshold,
        }
    }

    /// Insert a new entry with Transient status and use_count = 0.
    pub fn insert(&self, hash: String, kind: String, source: String) {
        let entry = CachedGlue {
            hash: hash.clone(),
            kind,
            nomx_source: source,
            status: GlueStatus::Transient,
            use_count: 0,
            created_at: Instant::now(),
        };
        self.entries.lock().unwrap().insert(hash, entry);
    }

    /// Increment use_count. If use_count reaches the promotion threshold
    /// and the entry is still Transient, promote it to Partial.
    pub fn record_use(&self, hash: &str) {
        let mut guard = self.entries.lock().unwrap();
        if let Some(entry) = guard.get_mut(hash) {
            entry.use_count += 1;
            if entry.use_count >= self.promotion_threshold
                && entry.status == GlueStatus::Transient
            {
                entry.status = GlueStatus::Partial;
            }
        }
    }

    /// Unconditionally promote the entry to Complete status.
    pub fn promote_to_complete(&self, hash: &str) {
        let mut guard = self.entries.lock().unwrap();
        if let Some(entry) = guard.get_mut(hash) {
            entry.status = GlueStatus::Complete;
        }
    }

    /// Return a snapshot of the entry for the given hash, or None.
    pub fn get(&self, hash: &str) -> Option<CachedGlue> {
        self.entries.lock().unwrap().get(hash).cloned()
    }

    /// Return the current status for the given hash, or None.
    pub fn status(&self, hash: &str) -> Option<GlueStatus> {
        self.entries.lock().unwrap().get(hash).map(|e| e.status.clone())
    }

    /// Return all entries with Partial status (ready for DB promotion).
    pub fn pending_promotion(&self) -> Vec<CachedGlue> {
        self.entries
            .lock()
            .unwrap()
            .values()
            .filter(|e| e.status == GlueStatus::Partial)
            .cloned()
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_glue_cache_insert_and_get() {
        let cache = GlueCache::new(3);
        cache.insert("hash_a".to_string(), "video_compose".to_string(), "source_a".to_string());

        let entry = cache.get("hash_a").expect("entry must exist after insert");
        assert_eq!(entry.hash, "hash_a");
        assert_eq!(entry.kind, "video_compose");
        assert_eq!(entry.nomx_source, "source_a");
        assert_eq!(entry.status, GlueStatus::Transient);
        assert_eq!(entry.use_count, 0);

        assert!(cache.get("missing").is_none(), "unknown hash must return None");
    }

    #[test]
    fn test_glue_cache_transient_to_partial_on_threshold() {
        let cache = GlueCache::new(3);
        cache.insert("hash_b".to_string(), "audio_compose".to_string(), "src_b".to_string());

        // Two uses — still Transient
        cache.record_use("hash_b");
        cache.record_use("hash_b");
        assert_eq!(
            cache.status("hash_b"),
            Some(GlueStatus::Transient),
            "status must remain Transient before threshold"
        );

        // Third use — reaches threshold, must promote to Partial
        cache.record_use("hash_b");
        assert_eq!(
            cache.status("hash_b"),
            Some(GlueStatus::Partial),
            "status must become Partial at threshold"
        );

        // Additional uses must not regress status
        cache.record_use("hash_b");
        assert_eq!(
            cache.status("hash_b"),
            Some(GlueStatus::Partial),
            "status must remain Partial after further uses"
        );
    }

    #[test]
    fn test_glue_cache_promote_to_complete() {
        let cache = GlueCache::new(2);
        cache.insert("hash_c".to_string(), "picture_compose".to_string(), "src_c".to_string());

        cache.record_use("hash_c");
        cache.record_use("hash_c");
        assert_eq!(cache.status("hash_c"), Some(GlueStatus::Partial));

        cache.promote_to_complete("hash_c");
        assert_eq!(
            cache.status("hash_c"),
            Some(GlueStatus::Complete),
            "status must be Complete after promote_to_complete"
        );
    }

    #[test]
    fn test_glue_cache_pending_promotion_returns_partial() {
        let cache = GlueCache::new(2);
        cache.insert("hash_d".to_string(), "workflow_compose".to_string(), "src_d".to_string());
        cache.insert("hash_e".to_string(), "document_compose".to_string(), "src_e".to_string());
        cache.insert("hash_f".to_string(), "web_app_compose".to_string(), "src_f".to_string());

        // Promote hash_d and hash_e to Partial
        cache.record_use("hash_d");
        cache.record_use("hash_d");
        cache.record_use("hash_e");
        cache.record_use("hash_e");

        // Promote hash_f all the way to Complete
        cache.record_use("hash_f");
        cache.record_use("hash_f");
        cache.promote_to_complete("hash_f");

        let pending = cache.pending_promotion();
        assert_eq!(
            pending.len(),
            2,
            "pending_promotion must return only Partial entries, got {}",
            pending.len()
        );
        let hashes: Vec<&str> = pending.iter().map(|e| e.hash.as_str()).collect();
        assert!(hashes.contains(&"hash_d"), "hash_d must be in pending");
        assert!(hashes.contains(&"hash_e"), "hash_e must be in pending");
    }
}
