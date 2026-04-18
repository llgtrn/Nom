use std::collections::HashMap;

/// Classifies a memoization key by its purity and caching policy.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MemoKeyKind {
    /// Deterministic: same inputs always produce same output. Long TTL.
    Pure,
    /// Has side-effects or external dependencies: never cache.
    Effectful,
    /// Computed from other cached values: medium TTL.
    Derived,
}

impl MemoKeyKind {
    /// Returns true when entries of this kind may be stored in the cache.
    pub fn is_cacheable(&self) -> bool {
        matches!(self, MemoKeyKind::Pure | MemoKeyKind::Derived)
    }

    /// Time-to-live in seconds for entries of this kind.
    /// Effectful entries have TTL 0 (never cache).
    pub fn ttl_secs(&self) -> u64 {
        match self {
            MemoKeyKind::Pure => 3600,
            MemoKeyKind::Effectful => 0,
            MemoKeyKind::Derived => 300,
        }
    }
}

/// A typed key for graph-aware memoization.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MemoKey {
    pub kind: MemoKeyKind,
    pub hash: u64,
    pub label: String,
}

impl MemoKey {
    /// Returns the string used as the HashMap key: `"<label>:<hash>"`.
    pub fn cache_key(&self) -> String {
        format!("{}:{}", self.label, self.hash)
    }

    /// Returns true when this key can produce a valid cache entry.
    pub fn is_valid(&self) -> bool {
        self.kind.is_cacheable() && self.hash != 0
    }
}

/// A single cached computation result with dependency tracking.
#[derive(Debug, Clone)]
pub struct MemoEntry {
    pub key: MemoKey,
    pub value_hash: u64,
    pub computed_at_ms: u64,
    pub dep_hashes: Vec<u64>,
}

impl MemoEntry {
    /// Returns true when the entry has aged past its TTL relative to `now_ms`.
    pub fn is_stale(&self, now_ms: u64) -> bool {
        let ttl_ms = self.key.kind.ttl_secs() * 1000;
        now_ms > self.computed_at_ms + ttl_ms
    }

    /// Returns the number of dependency hashes recorded for this entry.
    pub fn dep_count(&self) -> usize {
        self.dep_hashes.len()
    }
}

/// A graph-aware store of `MemoEntry` values, keyed by `MemoKey::cache_key()`.
#[derive(Debug, Default)]
pub struct MemoGraph {
    pub entries: HashMap<String, MemoEntry>,
}

impl MemoGraph {
    /// Inserts an entry using its `cache_key()` as the map key.
    pub fn insert(&mut self, e: MemoEntry) {
        let k = e.key.cache_key();
        self.entries.insert(k, e);
    }

    /// Returns a reference to the entry for `key`, or `None` if absent.
    pub fn get(&self, key: &MemoKey) -> Option<&MemoEntry> {
        self.entries.get(&key.cache_key())
    }

    /// Removes the entry for `key`. Returns `true` when an entry was present.
    pub fn invalidate(&mut self, key: &MemoKey) -> bool {
        self.entries.remove(&key.cache_key()).is_some()
    }

    /// Returns all entries that are stale at the given wall-clock time.
    pub fn stale_entries(&self, now_ms: u64) -> Vec<&MemoEntry> {
        self.entries
            .values()
            .filter(|e| e.is_stale(now_ms))
            .collect()
    }
}

/// Wraps a `MemoGraph` and provides bulk stale-entry invalidation.
pub struct MemoInvalidator {
    pub graph: MemoGraph,
}

impl MemoInvalidator {
    pub fn new() -> Self {
        Self {
            graph: MemoGraph::default(),
        }
    }

    /// Inserts an entry into the underlying graph.
    pub fn add(&mut self, e: MemoEntry) {
        self.graph.insert(e);
    }

    /// Removes all stale entries and returns the count removed.
    pub fn invalidate_stale(&mut self, now_ms: u64) -> usize {
        let stale_keys: Vec<String> = self
            .graph
            .entries
            .iter()
            .filter(|(_, e)| e.is_stale(now_ms))
            .map(|(k, _)| k.clone())
            .collect();
        let count = stale_keys.len();
        for k in stale_keys {
            self.graph.entries.remove(&k);
        }
        count
    }
}

impl Default for MemoInvalidator {
    fn default() -> Self {
        Self::new()
    }
}

// ---------------------------------------------------------------------------
// Unit tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn pure_key(label: &str, hash: u64) -> MemoKey {
        MemoKey {
            kind: MemoKeyKind::Pure,
            hash,
            label: label.to_string(),
        }
    }

    fn entry(key: MemoKey, computed_at_ms: u64) -> MemoEntry {
        MemoEntry {
            key,
            value_hash: 0xdeadbeef,
            computed_at_ms,
            dep_hashes: vec![1, 2, 3],
        }
    }

    // 1. Effectful kind is not cacheable.
    #[test]
    fn memo_graph_key_kind_effectful_not_cacheable() {
        assert!(!MemoKeyKind::Effectful.is_cacheable());
    }

    // 2. TTL values per kind.
    #[test]
    fn memo_graph_key_kind_ttl_secs() {
        assert_eq!(MemoKeyKind::Pure.ttl_secs(), 3600);
        assert_eq!(MemoKeyKind::Effectful.ttl_secs(), 0);
        assert_eq!(MemoKeyKind::Derived.ttl_secs(), 300);
    }

    // 3. cache_key format is "label:hash".
    #[test]
    fn memo_graph_memo_key_cache_key_format() {
        let k = MemoKey {
            kind: MemoKeyKind::Pure,
            hash: 42,
            label: "render".to_string(),
        };
        assert_eq!(k.cache_key(), "render:42");
    }

    // 4. is_valid returns false when hash == 0.
    #[test]
    fn memo_graph_memo_key_is_valid_hash_zero_false() {
        let k = MemoKey {
            kind: MemoKeyKind::Pure,
            hash: 0,
            label: "layout".to_string(),
        };
        assert!(!k.is_valid());
    }

    // 5. is_stale returns true when now_ms is past computed_at_ms + ttl_ms.
    #[test]
    fn memo_graph_entry_is_stale_true() {
        let k = pure_key("fn", 99);
        let e = MemoEntry {
            key: k,
            value_hash: 1,
            computed_at_ms: 0,
            dep_hashes: vec![],
        };
        // Pure TTL = 3600s = 3_600_000 ms. now = 3_600_001 ms → stale.
        assert!(e.is_stale(3_600_001));
    }

    // 6. dep_count returns the length of dep_hashes.
    #[test]
    fn memo_graph_entry_dep_count() {
        let k = pure_key("node", 7);
        let e = MemoEntry {
            key: k,
            value_hash: 0,
            computed_at_ms: 0,
            dep_hashes: vec![10, 20, 30, 40],
        };
        assert_eq!(e.dep_count(), 4);
    }

    // 7. graph insert then get returns the entry.
    #[test]
    fn memo_graph_insert_and_get() {
        let mut g = MemoGraph::default();
        let k = pure_key("compute", 1234);
        let e = entry(k.clone(), 0);
        g.insert(e);
        let found = g.get(&k);
        assert!(found.is_some());
        assert_eq!(found.unwrap().key.hash, 1234);
    }

    // 8. invalidate returns true when entry was present.
    #[test]
    fn memo_graph_invalidate_returns_true() {
        let mut g = MemoGraph::default();
        let k = pure_key("render", 55);
        g.insert(entry(k.clone(), 0));
        assert!(g.invalidate(&k));
        // Second call returns false (already removed).
        assert!(!g.invalidate(&k));
    }

    // 9. MemoInvalidator::invalidate_stale removes stale entries and returns count.
    #[test]
    fn memo_invalidator_invalidate_stale_count() {
        let mut inv = MemoInvalidator::new();
        // Two stale entries (computed_at = 0, now = 3_600_001 ms).
        inv.add(entry(pure_key("a", 1), 0));
        inv.add(entry(pure_key("b", 2), 0));
        // One fresh entry (computed_at = now, so now > 0 + ttl is false for now = ttl/2).
        let fresh_key = pure_key("c", 3);
        inv.add(MemoEntry {
            key: fresh_key,
            value_hash: 0,
            computed_at_ms: 3_600_001,
            dep_hashes: vec![],
        });
        let removed = inv.invalidate_stale(3_600_001);
        assert_eq!(removed, 2);
        assert_eq!(inv.graph.entries.len(), 1);
    }
}
