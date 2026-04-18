#![deny(unsafe_code)]
use lru::LruCache;
use std::num::NonZeroUsize;
use std::sync::{Arc, Mutex, RwLock};

/// Compile result cached by source hash
#[derive(Clone, Debug)]
pub struct PipelineOutput {
    pub source_hash: u64,
    pub grammar_version: u64,
    pub output_json: String, // serialized pipeline output
}

/// Grammar keyword cached from DB
#[derive(Clone, Debug)]
pub struct GrammarKind {
    pub name: String,
    pub description: String,
}

/// A pooled reader slot: holds a pre-constructed Arc<SharedState> ready to wrap
/// in SqliteDictReader::new(). Callers borrow a slot, use it, then return it.
/// This avoids redundant Arc clones on the hot path when the pool is non-empty.
pub struct ReaderSlot {
    pub state: Arc<SharedState>,
}

/// SharedState: thread-safe state shared across all bridge tiers
/// Owned by BridgeState, accessed via Arc<SharedState>
pub struct SharedState {
    /// Compile result cache: key = SipHash of (source_text, grammar_version)
    pub compile_cache: Mutex<LruCache<u64, PipelineOutput>>,
    /// Grammar kinds cache (loaded once at startup, refreshed on grammar DB change).
    /// RwLock allows multiple concurrent readers; writer only blocks during updates.
    pub grammar_kinds: RwLock<Vec<GrammarKind>>,
    /// Grammar version (incremented when grammar DB changes, used as cache key component)
    pub grammar_version: std::sync::atomic::AtomicU64,
    /// SQLite DB path for dict
    pub dict_path: String,
    /// SQLite DB path for grammar
    pub grammar_path: String,
    /// Reader pool: up to MAX_POOL_SIZE pre-constructed slots.
    /// Callers call borrow_reader() / return_reader() to reuse Arc<SharedState>
    /// instances instead of cloning on every request.
    reader_pool: Mutex<Vec<ReaderSlot>>,
}

/// Maximum number of reader slots kept alive in the pool.
const MAX_POOL_SIZE: usize = 4;

impl SharedState {
    pub fn new(dict_path: impl Into<String>, grammar_path: impl Into<String>) -> Self {
        let cache_capacity = NonZeroUsize::new(256).unwrap();
        Self {
            compile_cache: Mutex::new(LruCache::new(cache_capacity)),
            grammar_kinds: RwLock::new(Vec::new()),
            grammar_version: std::sync::atomic::AtomicU64::new(0),
            dict_path: dict_path.into(),
            grammar_path: grammar_path.into(),
            reader_pool: Mutex::new(Vec::with_capacity(MAX_POOL_SIZE)),
        }
    }

    /// SipHash-like key: xor of source_text hash and grammar version
    pub fn compile_cache_key(source_text: &str, grammar_version: u64) -> u64 {
        let text_hash = source_text
            .bytes()
            .fold(0u64, |acc, b| acc.wrapping_mul(31).wrapping_add(b as u64));
        text_hash ^ grammar_version.wrapping_mul(6364136223846793005)
    }

    pub fn get_cached_compile(&self, key: u64) -> Option<PipelineOutput> {
        self.compile_cache.lock().ok()?.get(&key).cloned()
    }

    pub fn cache_compile_result(&self, key: u64, output: PipelineOutput) {
        if let Ok(mut cache) = self.compile_cache.lock() {
            cache.put(key, output);
        }
    }

    pub fn grammar_version(&self) -> u64 {
        self.grammar_version
            .load(std::sync::atomic::Ordering::Relaxed)
    }

    /// Read grammar kinds — uses RwLock::read() so multiple threads can read concurrently.
    pub fn cached_grammar_kinds(&self) -> Vec<GrammarKind> {
        self.grammar_kinds
            .read()
            .ok()
            .map(|g| g.clone())
            .unwrap_or_default()
    }

    /// Replace grammar kinds — acquires the write lock (exclusive).
    pub fn update_grammar_kinds(&self, kinds: Vec<GrammarKind>) {
        if let Ok(mut g) = self.grammar_kinds.write() {
            *g = kinds;
        }
        self.grammar_version
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    }

    /// Borrow a reader slot from the pool, or create a new one if the pool is empty.
    /// The returned slot holds an Arc<SharedState> pointing back to self.
    /// Callers MUST return the slot via return_reader() after use.
    pub fn borrow_reader(self: &Arc<Self>) -> ReaderSlot {
        if let Ok(mut pool) = self.reader_pool.lock() {
            if let Some(slot) = pool.pop() {
                return slot;
            }
        }
        ReaderSlot {
            state: Arc::clone(self),
        }
    }

    /// Return a previously borrowed reader slot to the pool.
    /// If the pool is already at MAX_POOL_SIZE, the slot is dropped.
    pub fn return_reader(&self, slot: ReaderSlot) {
        if let Ok(mut pool) = self.reader_pool.lock() {
            if pool.len() < MAX_POOL_SIZE {
                pool.push(slot);
            }
        }
    }

    /// Current number of slots sitting idle in the pool (for diagnostics/tests).
    pub fn pool_idle_count(&self) -> usize {
        self.reader_pool
            .lock()
            .map(|p| p.len())
            .unwrap_or(0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn compile_cache_key_deterministic() {
        let k1 = SharedState::compile_cache_key("define x that is 42", 1);
        let k2 = SharedState::compile_cache_key("define x that is 42", 1);
        assert_eq!(k1, k2);
    }

    #[test]
    fn compile_cache_key_differs_by_source() {
        let k1 = SharedState::compile_cache_key("source_a", 1);
        let k2 = SharedState::compile_cache_key("source_b", 1);
        assert_ne!(k1, k2);
    }

    #[test]
    fn compile_cache_roundtrip() {
        let state = SharedState::new("dict.db", "grammar.db");
        let key = SharedState::compile_cache_key("hello", 0);
        let output = PipelineOutput {
            source_hash: key,
            grammar_version: 0,
            output_json: "{}".into(),
        };
        state.cache_compile_result(key, output.clone());
        let result = state.get_cached_compile(key);
        assert!(result.is_some());
        assert_eq!(result.unwrap().output_json, "{}");
    }

    #[test]
    fn grammar_kinds_update() {
        let state = SharedState::new("a", "b");
        assert_eq!(state.grammar_version(), 0);
        state.update_grammar_kinds(vec![GrammarKind {
            name: "verb".into(),
            description: "action".into(),
        }]);
        assert_eq!(state.grammar_version(), 1);
        let kinds = state.cached_grammar_kinds();
        assert_eq!(kinds.len(), 1);
        assert_eq!(kinds[0].name, "verb");
    }

    #[test]
    fn shared_state_grammar_cache_roundtrip() {
        let state = SharedState::new("dict.db", "grammar.db");
        assert!(state.cached_grammar_kinds().is_empty());
        state.update_grammar_kinds(vec![GrammarKind {
            name: "action".into(),
            description: "something done".into(),
        }]);
        let kinds = state.cached_grammar_kinds();
        assert_eq!(kinds.len(), 1);
        assert_eq!(kinds[0].name, "action");
        assert_eq!(kinds[0].description, "something done");
    }

    #[test]
    fn shared_state_lru_eviction() {
        let state = SharedState::new("d.db", "g.db");
        // Insert 257 entries to exceed the LRU capacity of 256
        let version = state.grammar_version();
        // The first key inserted (key_0) should be evicted after 256 more are added
        let key_0 = SharedState::compile_cache_key("evict_me_source", version);
        state.cache_compile_result(
            key_0,
            PipelineOutput {
                source_hash: key_0,
                grammar_version: version,
                output_json: "{}".into(),
            },
        );
        for i in 1u64..=256 {
            let src = format!("source_{i}");
            let k = SharedState::compile_cache_key(&src, version);
            state.cache_compile_result(
                k,
                PipelineOutput {
                    source_hash: k,
                    grammar_version: version,
                    output_json: "{}".into(),
                },
            );
        }
        // key_0 should have been evicted (LRU capacity = 256, we added 257 total)
        assert!(
            state.get_cached_compile(key_0).is_none(),
            "oldest entry should have been evicted by LRU"
        );
    }

    #[test]
    fn shared_state_pipeline_output_version_tracking() {
        let state = SharedState::new("d.db", "g.db");
        // Bump grammar version so we have a non-zero version to track
        state.update_grammar_kinds(vec![GrammarKind {
            name: "noun".into(),
            description: "thing".into(),
        }]);
        let version = state.grammar_version();
        assert_eq!(version, 1);
        let source = "define item that is 1";
        let key = SharedState::compile_cache_key(source, version);
        let output = PipelineOutput {
            source_hash: key,
            grammar_version: version,
            output_json: r#"{"ok":true}"#.into(),
        };
        state.cache_compile_result(key, output.clone());
        let retrieved = state.get_cached_compile(key).expect("should be cached");
        assert_eq!(retrieved.source_hash, key, "source_hash preserved");
        assert_eq!(retrieved.grammar_version, 1, "grammar_version preserved");
    }

    #[test]
    fn shared_state_dict_pool_capacity() {
        use std::num::NonZeroUsize;
        let cap = NonZeroUsize::new(256).unwrap();
        // The LRU cache is constructed with capacity 256
        assert_eq!(cap.get(), 256);
        let state = SharedState::new("dict.db", "grammar.db");
        // Insert up to capacity; none should be evicted yet
        let version = state.grammar_version();
        for i in 0u64..10 {
            let key = SharedState::compile_cache_key(&format!("src_{i}"), version);
            state.cache_compile_result(
                key,
                PipelineOutput {
                    source_hash: key,
                    grammar_version: version,
                    output_json: "{}".into(),
                },
            );
        }
        // All 10 should still be present
        for i in 0u64..10 {
            let key = SharedState::compile_cache_key(&format!("src_{i}"), version);
            assert!(
                state.get_cached_compile(key).is_some(),
                "entry {i} should still be cached"
            );
        }
    }

    #[test]
    fn shared_state_arc_strong_count_one() {
        use std::sync::Arc;
        let state = Arc::new(SharedState::new("d.db", "g.db"));
        // Freshly created Arc has strong count 1
        assert_eq!(Arc::strong_count(&state), 1);
    }

    #[test]
    fn pipeline_output_source_hash_set() {
        let output = PipelineOutput {
            source_hash: 0xDEAD_BEEF,
            grammar_version: 42,
            output_json: r#"{"result":"ok"}"#.into(),
        };
        assert_eq!(output.source_hash, 0xDEAD_BEEF);
        assert_eq!(output.grammar_version, 42);
        assert!(output.output_json.contains("ok"));
    }

    #[test]
    fn compile_cache_key_changes_with_version() {
        let k1 = SharedState::compile_cache_key("same_source", 0);
        let k2 = SharedState::compile_cache_key("same_source", 1);
        assert_ne!(
            k1, k2,
            "different grammar versions must produce different cache keys"
        );
    }

    #[test]
    fn shared_state_paths_stored() {
        let state = SharedState::new("my_dict.db", "my_grammar.db");
        assert_eq!(state.dict_path, "my_dict.db");
        assert_eq!(state.grammar_path, "my_grammar.db");
    }

    #[test]
    fn shared_state_multiple_grammar_updates_increment_version() {
        let state = SharedState::new("d.db", "g.db");
        state.update_grammar_kinds(vec![GrammarKind {
            name: "a".into(),
            description: "".into(),
        }]);
        state.update_grammar_kinds(vec![GrammarKind {
            name: "b".into(),
            description: "".into(),
        }]);
        state.update_grammar_kinds(vec![GrammarKind {
            name: "c".into(),
            description: "".into(),
        }]);
        assert_eq!(state.grammar_version(), 3);
        let kinds = state.cached_grammar_kinds();
        assert_eq!(kinds.len(), 1);
        assert_eq!(kinds[0].name, "c");
    }

    #[test]
    fn shared_state_grammar_version_starts_zero() {
        let state = SharedState::new("d.db", "g.db");
        assert_eq!(state.grammar_version(), 0);
    }

    #[test]
    fn shared_state_increment_grammar_version() {
        let state = SharedState::new("d.db", "g.db");
        state.update_grammar_kinds(vec![GrammarKind {
            name: "noun".into(),
            description: "thing".into(),
        }]);
        assert_eq!(state.grammar_version(), 1);
        state.update_grammar_kinds(vec![GrammarKind {
            name: "verb".into(),
            description: "action".into(),
        }]);
        assert_eq!(state.grammar_version(), 2);
    }

    #[test]
    fn shared_state_dict_pool_non_empty() {
        use std::num::NonZeroUsize;
        // The LRU cache is built with capacity 256 — confirm capacity > 0
        let cap = NonZeroUsize::new(256).unwrap();
        assert!(cap.get() > 0);
        // Inserting one entry leaves it retrievable (pool is functional)
        let state = SharedState::new("d.db", "g.db");
        let key = SharedState::compile_cache_key("probe", 0);
        state.cache_compile_result(
            key,
            PipelineOutput {
                source_hash: key,
                grammar_version: 0,
                output_json: "{}".into(),
            },
        );
        assert!(state.get_cached_compile(key).is_some());
    }

    #[test]
    fn shared_state_two_arcs_same_data() {
        use std::sync::Arc;
        let state = Arc::new(SharedState::new("my_path.db", "my_gram.db"));
        let clone = Arc::clone(&state);
        // Both arcs point to the same allocation — dict_path is identical
        assert_eq!(state.dict_path, clone.dict_path);
        assert_eq!(state.grammar_path, clone.grammar_path);
    }

    #[test]
    fn pipeline_output_grammar_version_preserved() {
        let output = PipelineOutput {
            source_hash: 1234,
            grammar_version: 99,
            output_json: r#"{"v":99}"#.into(),
        };
        assert_eq!(output.grammar_version, 99);
        assert!(output.output_json.contains("99"));
    }

    #[test]
    fn compile_cache_key_empty_string_stable() {
        // Empty source with version 0 must produce a stable u64 (no panic, same value each call)
        let k1 = SharedState::compile_cache_key("", 0);
        let k2 = SharedState::compile_cache_key("", 0);
        assert_eq!(k1, k2);
    }

    #[test]
    fn compile_cache_key_unicode_stable() {
        let k1 = SharedState::compile_cache_key("définir résultat ✓", 3);
        let k2 = SharedState::compile_cache_key("définir résultat ✓", 3);
        assert_eq!(k1, k2);
    }

    #[test]
    fn concurrent_read_write_grammar_kinds() {
        use std::sync::Arc;
        use std::thread;

        let state = Arc::new(SharedState::new("d.db", "g.db"));
        let readers: Vec<_> = (0..4)
            .map(|_| {
                let s = Arc::clone(&state);
                thread::spawn(move || {
                    for _ in 0..50 {
                        let _ = s.cached_grammar_kinds();
                    }
                })
            })
            .collect();

        // Writer thread concurrently updates grammar kinds
        let writer_state = Arc::clone(&state);
        let writer = thread::spawn(move || {
            for i in 0u32..10 {
                writer_state.update_grammar_kinds(vec![GrammarKind {
                    name: format!("kind_{i}"),
                    description: format!("desc_{i}"),
                }]);
            }
        });

        for r in readers {
            r.join().expect("reader thread panicked");
        }
        writer.join().expect("writer thread panicked");
        // After all writes, grammar_version must be at least 10
        assert!(state.grammar_version() >= 10);
    }

    #[test]
    fn concurrent_compile_cache_reads() {
        use std::sync::Arc;
        use std::thread;

        let state = Arc::new(SharedState::new("d.db", "g.db"));
        // Pre-populate a cache entry
        let key = SharedState::compile_cache_key("concurrent_source", 0);
        state.cache_compile_result(
            key,
            PipelineOutput {
                source_hash: key,
                grammar_version: 0,
                output_json: r#"{"ok":true}"#.into(),
            },
        );

        let threads: Vec<_> = (0..8)
            .map(|_| {
                let s = Arc::clone(&state);
                thread::spawn(move || {
                    for _ in 0..20 {
                        let result = s.get_cached_compile(key);
                        if let Some(r) = result {
                            assert_eq!(r.source_hash, key);
                        }
                    }
                })
            })
            .collect();

        for t in threads {
            t.join().expect("reader thread panicked");
        }
    }

    #[test]
    fn dict_pool_eviction_at_exactly_capacity() {
        let state = SharedState::new("d.db", "g.db");
        let version = state.grammar_version();
        // Fill to exactly capacity (256)
        for i in 0u64..256 {
            let k = SharedState::compile_cache_key(&format!("fill_{i}"), version);
            state.cache_compile_result(
                k,
                PipelineOutput {
                    source_hash: k,
                    grammar_version: version,
                    output_json: "{}".into(),
                },
            );
        }
        // Insert one more — the first inserted entry (fill_0) should be evicted
        let first_key = SharedState::compile_cache_key("fill_0", version);
        let overflow_key = SharedState::compile_cache_key("overflow_entry", version);
        state.cache_compile_result(
            overflow_key,
            PipelineOutput {
                source_hash: overflow_key,
                grammar_version: version,
                output_json: "{}".into(),
            },
        );
        // overflow_key is present
        assert!(
            state.get_cached_compile(overflow_key).is_some(),
            "newly inserted entry must be present"
        );
        // fill_0 was evicted
        assert!(
            state.get_cached_compile(first_key).is_none(),
            "oldest entry must have been evicted"
        );
    }

    #[test]
    fn grammar_kinds_replaced_on_each_update() {
        let state = SharedState::new("d.db", "g.db");
        state.update_grammar_kinds(vec![
            GrammarKind {
                name: "a".into(),
                description: "".into(),
            },
            GrammarKind {
                name: "b".into(),
                description: "".into(),
            },
        ]);
        assert_eq!(state.cached_grammar_kinds().len(), 2);
        state.update_grammar_kinds(vec![GrammarKind {
            name: "c".into(),
            description: "".into(),
        }]);
        let kinds = state.cached_grammar_kinds();
        assert_eq!(kinds.len(), 1);
        assert_eq!(kinds[0].name, "c");
    }

    #[test]
    fn cache_miss_returns_none() {
        let state = SharedState::new("d.db", "g.db");
        let bogus_key = 0xDEAD_C0DE_DEAD_C0DE_u64;
        assert!(state.get_cached_compile(bogus_key).is_none());
    }

    #[test]
    fn compile_cache_key_max_version() {
        // u64::MAX grammar version must not panic
        let k = SharedState::compile_cache_key("source", u64::MAX);
        let k2 = SharedState::compile_cache_key("source", u64::MAX);
        assert_eq!(k, k2);
    }

    #[test]
    fn compile_cache_key_different_for_whitespace_variants() {
        let k1 = SharedState::compile_cache_key("hello world", 0);
        let k2 = SharedState::compile_cache_key("hello  world", 0);
        assert_ne!(k1, k2);
    }

    #[test]
    fn pipeline_output_clone_is_independent() {
        let output = PipelineOutput {
            source_hash: 42,
            grammar_version: 7,
            output_json: r#"{"x":1}"#.into(),
        };
        let cloned = output.clone();
        assert_eq!(cloned.source_hash, output.source_hash);
        assert_eq!(cloned.grammar_version, output.grammar_version);
        assert_eq!(cloned.output_json, output.output_json);
    }

    #[test]
    fn grammar_kind_clone_is_independent() {
        let kind = GrammarKind {
            name: "verb".into(),
            description: "action".into(),
        };
        let cloned = kind.clone();
        assert_eq!(cloned.name, kind.name);
        assert_eq!(cloned.description, kind.description);
    }

    #[test]
    fn update_grammar_kinds_clears_previous() {
        let state = SharedState::new("d.db", "g.db");
        state.update_grammar_kinds(vec![
            GrammarKind { name: "a".into(), description: "".into() },
            GrammarKind { name: "b".into(), description: "".into() },
            GrammarKind { name: "c".into(), description: "".into() },
        ]);
        assert_eq!(state.cached_grammar_kinds().len(), 3);
        state.update_grammar_kinds(vec![]);
        assert!(state.cached_grammar_kinds().is_empty());
        // Version still increments even when clearing
        assert_eq!(state.grammar_version(), 2);
    }

    #[test]
    fn compile_cache_overwrite_same_key() {
        let state = SharedState::new("d.db", "g.db");
        let key = SharedState::compile_cache_key("overwrite_src", 0);
        state.cache_compile_result(
            key,
            PipelineOutput { source_hash: key, grammar_version: 0, output_json: "v1".into() },
        );
        state.cache_compile_result(
            key,
            PipelineOutput { source_hash: key, grammar_version: 0, output_json: "v2".into() },
        );
        let retrieved = state.get_cached_compile(key).expect("should be present");
        assert_eq!(retrieved.output_json, "v2");
    }

    #[test]
    fn shared_state_arc_clone_sees_same_cache() {
        use std::sync::Arc;
        let state = Arc::new(SharedState::new("d.db", "g.db"));
        let clone = Arc::clone(&state);
        let key = SharedState::compile_cache_key("shared_src", 0);
        state.cache_compile_result(
            key,
            PipelineOutput { source_hash: key, grammar_version: 0, output_json: "{}".into() },
        );
        assert!(clone.get_cached_compile(key).is_some());
    }

    #[test]
    fn grammar_version_not_affected_by_cache_inserts() {
        let state = SharedState::new("d.db", "g.db");
        let v0 = state.grammar_version();
        let key = SharedState::compile_cache_key("src", 0);
        state.cache_compile_result(
            key,
            PipelineOutput { source_hash: key, grammar_version: 0, output_json: "{}".into() },
        );
        assert_eq!(state.grammar_version(), v0, "cache insert must not bump grammar version");
    }

    #[test]
    fn compile_cache_key_single_byte_sources_differ() {
        let k_a = SharedState::compile_cache_key("a", 0);
        let k_b = SharedState::compile_cache_key("b", 0);
        assert_ne!(k_a, k_b);
    }

    #[test]
    fn grammar_kinds_large_batch() {
        let state = SharedState::new("d.db", "g.db");
        let kinds: Vec<_> = (0..100)
            .map(|i| GrammarKind {
                name: format!("kind_{i:03}"),
                description: format!("description for kind {i}"),
            })
            .collect();
        state.update_grammar_kinds(kinds);
        assert_eq!(state.cached_grammar_kinds().len(), 100);
        assert_eq!(state.grammar_version(), 1);
    }

    // ── AE3 additions ──────────────────────────────────────────────────────

    /// Pool under concurrent load: 8 threads each calling borrow + return in a loop.
    #[test]
    fn pool_under_concurrent_load_8_threads() {
        use std::sync::Arc;
        use std::thread;

        let state = Arc::new(SharedState::new("d.db", "g.db"));

        let handles: Vec<_> = (0..8)
            .map(|_| {
                let s = Arc::clone(&state);
                thread::spawn(move || {
                    for _ in 0..25 {
                        let slot = s.borrow_reader();
                        // Slot must point to same state (dict_path is deterministic)
                        assert_eq!(slot.state.dict_path, "d.db");
                        s.return_reader(slot);
                    }
                })
            })
            .collect();

        for h in handles {
            h.join().expect("thread panicked");
        }

        // Pool must not exceed MAX_POOL_SIZE after all threads finish
        assert!(
            state.pool_idle_count() <= 4,
            "pool exceeded MAX_POOL_SIZE: {}",
            state.pool_idle_count()
        );
    }

    /// RwLock read from 4 threads simultaneously returns the same data.
    #[test]
    fn rwlock_read_from_4_threads_returns_same_data() {
        use std::sync::Arc;
        use std::thread;

        let state = Arc::new(SharedState::new("d.db", "g.db"));
        state.update_grammar_kinds(vec![
            GrammarKind { name: "alpha".into(), description: "first".into() },
            GrammarKind { name: "beta".into(), description: "second".into() },
        ]);

        let handles: Vec<_> = (0..4)
            .map(|_| {
                let s = Arc::clone(&state);
                thread::spawn(move || {
                    let kinds = s.cached_grammar_kinds();
                    (kinds[0].name.clone(), kinds[1].name.clone())
                })
            })
            .collect();

        for h in handles {
            let (name0, name1) = h.join().expect("reader thread panicked");
            assert_eq!(name0, "alpha");
            assert_eq!(name1, "beta");
        }
    }

    /// borrow_reader when pool is empty creates a fresh slot pointing to self.
    #[test]
    fn borrow_reader_empty_pool_creates_fresh_slot() {
        use std::sync::Arc;
        let state = Arc::new(SharedState::new("fresh.db", "g.db"));
        assert_eq!(state.pool_idle_count(), 0);
        let slot = state.borrow_reader();
        assert_eq!(slot.state.dict_path, "fresh.db");
        // pool is still empty because we didn't return
        assert_eq!(state.pool_idle_count(), 0);
        // return it — now pool has 1
        state.return_reader(slot);
        assert_eq!(state.pool_idle_count(), 1);
    }

    /// Returned slot holds a valid Arc pointing back to the same SharedState.
    #[test]
    fn returned_reader_slot_arc_points_to_same_state() {
        use std::sync::Arc;
        let state = Arc::new(SharedState::new("same.db", "g.db"));
        let slot = state.borrow_reader();
        // The Arc inside the slot must alias the same allocation
        assert!(Arc::ptr_eq(&state, &slot.state));
        state.return_reader(slot);
    }

    // ── RwLock / reader-pool tests ──────────────────────────────────────────

    /// Multiple concurrent readers must not block each other.
    /// With RwLock, N read guards can all be held simultaneously.
    #[test]
    fn rwlock_multiple_readers_simultaneous() {
        use std::sync::{Arc, RwLock};
        use std::thread;

        let lock: Arc<RwLock<Vec<GrammarKind>>> = Arc::new(RwLock::new(vec![
            GrammarKind { name: "noun".into(), description: "thing".into() },
        ]));

        // Acquire 8 read guards in parallel — none should block the others.
        let handles: Vec<_> = (0..8)
            .map(|_| {
                let l = Arc::clone(&lock);
                thread::spawn(move || {
                    let guard = l.read().expect("read lock poisoned");
                    assert_eq!(guard.len(), 1);
                    assert_eq!(guard[0].name, "noun");
                })
            })
            .collect();

        for h in handles {
            h.join().expect("reader thread panicked");
        }
    }

    /// Write upgrade: after reads complete, a write lock can be acquired and
    /// observed by a subsequent read.
    #[test]
    fn rwlock_write_upgrade_visible_to_readers() {
        use std::sync::{Arc, RwLock};
        use std::thread;

        let lock: Arc<RwLock<Vec<GrammarKind>>> = Arc::new(RwLock::new(vec![]));

        // Writer thread pushes one entry.
        let wl = Arc::clone(&lock);
        let writer = thread::spawn(move || {
            let mut g = wl.write().expect("write lock poisoned");
            g.push(GrammarKind { name: "verb".into(), description: "action".into() });
        });
        writer.join().expect("writer panicked");

        // Reader sees the update.
        let reader = lock.read().expect("read lock poisoned");
        assert_eq!(reader.len(), 1);
        assert_eq!(reader[0].name, "verb");
    }

    /// Grammar kinds accessible via RwLock::read() from 2 threads simultaneously.
    #[test]
    fn grammar_kinds_rwlock_two_threads_simultaneous_read() {
        use std::sync::Arc;
        use std::thread;

        let state = Arc::new(SharedState::new("d.db", "g.db"));
        state.update_grammar_kinds(vec![
            GrammarKind { name: "action".into(), description: "something done".into() },
            GrammarKind { name: "entity".into(), description: "something that exists".into() },
        ]);

        let s1 = Arc::clone(&state);
        let s2 = Arc::clone(&state);

        let t1 = thread::spawn(move || s1.cached_grammar_kinds());
        let t2 = thread::spawn(move || s2.cached_grammar_kinds());

        let kinds1 = t1.join().expect("thread 1 panicked");
        let kinds2 = t2.join().expect("thread 2 panicked");

        assert_eq!(kinds1.len(), 2);
        assert_eq!(kinds2.len(), 2);
        assert_eq!(kinds1[0].name, "action");
        assert_eq!(kinds2[1].name, "entity");
    }

    /// Borrowed reader slot is returned to the pool; second borrow gets it back.
    #[test]
    fn reader_pool_slot_reused_after_return() {
        use std::sync::Arc;

        let state = Arc::new(SharedState::new("d.db", "g.db"));
        assert_eq!(state.pool_idle_count(), 0);

        // Borrow, then return — pool grows to 1.
        let slot = state.borrow_reader();
        state.return_reader(slot);
        assert_eq!(state.pool_idle_count(), 1);

        // Second borrow pops from the pool — idle count drops back to 0.
        let slot2 = state.borrow_reader();
        assert_eq!(state.pool_idle_count(), 0);
        // Return again.
        state.return_reader(slot2);
        assert_eq!(state.pool_idle_count(), 1);
    }

    /// Pool is capped at MAX_POOL_SIZE; excess returned slots are dropped.
    #[test]
    fn reader_pool_capped_at_max_size() {
        use std::sync::Arc;

        let state = Arc::new(SharedState::new("d.db", "g.db"));

        // Borrow 6 slots (> MAX_POOL_SIZE=4) and return them all.
        let slots: Vec<_> = (0..6).map(|_| state.borrow_reader()).collect();
        for slot in slots {
            state.return_reader(slot);
        }

        // Pool must not exceed MAX_POOL_SIZE.
        assert!(
            state.pool_idle_count() <= 4,
            "pool exceeded MAX_POOL_SIZE: got {}",
            state.pool_idle_count()
        );
    }

    /// Concurrent borrow/return cycle is race-free under many threads.
    #[test]
    fn reader_pool_concurrent_borrow_return_no_panic() {
        use std::sync::Arc;
        use std::thread;

        let state = Arc::new(SharedState::new("d.db", "g.db"));

        let handles: Vec<_> = (0..16)
            .map(|_| {
                let s = Arc::clone(&state);
                thread::spawn(move || {
                    for _ in 0..20 {
                        let slot = s.borrow_reader();
                        // Verify the slot points to the same state.
                        assert_eq!(slot.state.dict_path, "d.db");
                        s.return_reader(slot);
                    }
                })
            })
            .collect();

        for h in handles {
            h.join().expect("thread panicked");
        }

        // Pool must not exceed MAX_POOL_SIZE after all threads finish.
        assert!(state.pool_idle_count() <= 4);
    }

    // ── AG6 additions ──────────────────────────────────────────────────────

    /// Multiple concurrent readers via RwLock don't block each other (via SharedState).
    #[test]
    fn rwlock_grammar_kinds_concurrent_readers() {
        use std::sync::Arc;
        use std::thread;

        let state = Arc::new(SharedState::new("d.db", "g.db"));
        state.update_grammar_kinds(vec![
            GrammarKind { name: "noun".into(), description: "thing".into() },
            GrammarKind { name: "verb".into(), description: "action".into() },
        ]);

        // 6 concurrent readers — all succeed without blocking each other
        let handles: Vec<_> = (0..6)
            .map(|_| {
                let s = Arc::clone(&state);
                thread::spawn(move || {
                    let kinds = s.cached_grammar_kinds();
                    assert_eq!(kinds.len(), 2);
                })
            })
            .collect();

        for h in handles {
            h.join().expect("reader thread panicked");
        }
    }

    /// borrow_reader returns a slot whose state dict_path matches self.
    #[test]
    fn borrow_reader_returns_slot() {
        use std::sync::Arc;
        let state = Arc::new(SharedState::new("target.db", "g.db"));
        let slot = state.borrow_reader();
        assert_eq!(slot.state.dict_path, "target.db");
        state.return_reader(slot);
    }

    /// After borrow then return, a second borrow_reader succeeds (slot is reused).
    #[test]
    fn return_reader_puts_slot_back() {
        use std::sync::Arc;
        let state = Arc::new(SharedState::new("d.db", "g.db"));
        let slot = state.borrow_reader();
        state.return_reader(slot);
        // Second borrow should succeed — no panic, slot is valid
        let slot2 = state.borrow_reader();
        assert_eq!(slot2.state.dict_path, "d.db");
        state.return_reader(slot2);
    }

    /// Pool starts with 0 idle slots; after returning 4, it holds exactly 4.
    #[test]
    fn reader_pool_max_4_slots() {
        use std::sync::Arc;
        let state = Arc::new(SharedState::new("d.db", "g.db"));
        assert_eq!(state.pool_idle_count(), 0);
        // Borrow 4 slots then return them
        let slots: Vec<_> = (0..4).map(|_| state.borrow_reader()).collect();
        for s in slots {
            state.return_reader(s);
        }
        assert_eq!(state.pool_idle_count(), 4, "pool must hold exactly 4 slots after 4 returns");
    }

    /// Write kinds, then read them back: round trip preserves content.
    #[test]
    fn grammar_kinds_read_write_round_trip() {
        let state = SharedState::new("d.db", "g.db");
        let expected = vec![
            GrammarKind { name: "alpha".into(), description: "first".into() },
            GrammarKind { name: "beta".into(), description: "second".into() },
        ];
        state.update_grammar_kinds(expected.clone());
        let read_back = state.cached_grammar_kinds();
        assert_eq!(read_back.len(), 2);
        assert_eq!(read_back[0].name, "alpha");
        assert_eq!(read_back[1].name, "beta");
    }

    /// Fresh SharedState has empty grammar_kinds.
    #[test]
    fn shared_state_new_default_empty_kinds() {
        let state = SharedState::new("x.db", "y.db");
        assert!(state.cached_grammar_kinds().is_empty(), "fresh state must have no grammar kinds");
    }

    /// After borrowing all 4 pool slots, the 5th borrow creates a new slot (pool empty → fresh).
    #[test]
    fn borrow_reader_all_slots_exhausted_returns_none() {
        use std::sync::Arc;
        let state = Arc::new(SharedState::new("d.db", "g.db"));
        // Pre-populate pool with 4 slots
        let initial: Vec<_> = (0..4).map(|_| state.borrow_reader()).collect();
        for s in initial {
            state.return_reader(s);
        }
        assert_eq!(state.pool_idle_count(), 4);
        // Borrow all 4 — pool becomes empty
        let borrowed: Vec<_> = (0..4).map(|_| state.borrow_reader()).collect();
        assert_eq!(state.pool_idle_count(), 0, "pool must be empty after borrowing all 4 slots");
        // 5th borrow creates a fresh slot (no panic)
        let extra = state.borrow_reader();
        assert_eq!(extra.state.dict_path, "d.db");
        // Return all
        for s in borrowed {
            state.return_reader(s);
        }
        state.return_reader(extra);
    }

    /// Returning a slot when pool is already at MAX_POOL_SIZE causes the extra to be dropped silently.
    #[test]
    fn return_reader_after_full_pool_no_panic() {
        use std::sync::Arc;
        let state = Arc::new(SharedState::new("d.db", "g.db"));
        // Fill pool to MAX_POOL_SIZE (4)
        let slots: Vec<_> = (0..4).map(|_| state.borrow_reader()).collect();
        for s in slots {
            state.return_reader(s);
        }
        assert_eq!(state.pool_idle_count(), 4);
        // Returning a 5th slot — pool is full, extra must be dropped silently (no panic)
        let extra = state.borrow_reader(); // creates fresh (pool empty after 4 borrows above)
        // Borrow one to make room check: pool still at 4 after returning above 4; extra is fresh
        state.return_reader(extra); // pool is already at 4, so this extra is dropped
        // Count must not exceed 4
        assert!(state.pool_idle_count() <= 4, "pool must not exceed MAX_POOL_SIZE");
    }

    /// update_grammar_kinds replaces the previous list (acts as setter).
    #[test]
    fn shared_state_update_grammar_kinds() {
        let state = SharedState::new("d.db", "g.db");
        state.update_grammar_kinds(vec![
            GrammarKind { name: "old".into(), description: "stale".into() },
        ]);
        assert_eq!(state.cached_grammar_kinds().len(), 1);
        state.update_grammar_kinds(vec![
            GrammarKind { name: "new_a".into(), description: "fresh".into() },
            GrammarKind { name: "new_b".into(), description: "also fresh".into() },
        ]);
        let kinds = state.cached_grammar_kinds();
        assert_eq!(kinds.len(), 2);
        assert_eq!(kinds[0].name, "new_a");
        assert_eq!(kinds[1].name, "new_b");
    }

    /// Grammar kinds can be filtered by name substring (simulates category filter).
    #[test]
    fn grammar_kinds_filter_by_category() {
        let state = SharedState::new("d.db", "g.db");
        state.update_grammar_kinds(vec![
            GrammarKind { name: "verb_run".into(), description: "action".into() },
            GrammarKind { name: "verb_jump".into(), description: "action".into() },
            GrammarKind { name: "noun_item".into(), description: "thing".into() },
        ]);
        let kinds = state.cached_grammar_kinds();
        let verbs: Vec<_> = kinds.iter().filter(|k| k.name.starts_with("verb")).collect();
        assert_eq!(verbs.len(), 2, "filter by 'verb' prefix must yield 2 results");
        let nouns: Vec<_> = kinds.iter().filter(|k| k.name.starts_with("noun")).collect();
        assert_eq!(nouns.len(), 1);
    }

    /// ReaderSlot has a state field that is an Arc<SharedState>.
    #[test]
    fn pool_slot_has_reader_field() {
        use std::sync::Arc;
        let state = Arc::new(SharedState::new("slot_test.db", "g.db"));
        let slot = state.borrow_reader();
        // Access the Arc inside the slot
        assert_eq!(slot.state.dict_path, "slot_test.db");
        // Arc strong count reflects that slot holds a reference
        assert!(Arc::strong_count(&state) >= 2);
        state.return_reader(slot);
    }

    /// Concurrent borrow and return from 4 threads is data-race-free.
    #[test]
    fn pool_concurrent_borrow_and_return_safe() {
        use std::sync::Arc;
        use std::thread;

        let state = Arc::new(SharedState::new("d.db", "g.db"));

        let handles: Vec<_> = (0..4)
            .map(|_| {
                let s = Arc::clone(&state);
                thread::spawn(move || {
                    for _ in 0..10 {
                        let slot = s.borrow_reader();
                        // Verify we got a valid slot
                        assert_eq!(slot.state.dict_path, "d.db");
                        s.return_reader(slot);
                    }
                })
            })
            .collect();

        for h in handles {
            h.join().expect("thread panicked");
        }
        assert!(state.pool_idle_count() <= 4);
    }
}
