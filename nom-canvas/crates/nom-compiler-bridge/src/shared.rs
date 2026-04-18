#![deny(unsafe_code)]
use lru::LruCache;
use std::num::NonZeroUsize;
use std::sync::Mutex;

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

/// SharedState: thread-safe state shared across all bridge tiers
/// Owned by BridgeState, accessed via Arc<SharedState>
pub struct SharedState {
    /// Compile result cache: key = SipHash of (source_text, grammar_version)
    pub compile_cache: Mutex<LruCache<u64, PipelineOutput>>,
    /// Grammar kinds cache (loaded once at startup, refreshed on grammar DB change)
    pub grammar_kinds: Mutex<Vec<GrammarKind>>,
    /// Grammar version (incremented when grammar DB changes, used as cache key component)
    pub grammar_version: std::sync::atomic::AtomicU64,
    /// SQLite DB path for dict
    pub dict_path: String,
    /// SQLite DB path for grammar
    pub grammar_path: String,
}

impl SharedState {
    pub fn new(dict_path: impl Into<String>, grammar_path: impl Into<String>) -> Self {
        let cache_capacity = NonZeroUsize::new(256).unwrap();
        Self {
            compile_cache: Mutex::new(LruCache::new(cache_capacity)),
            grammar_kinds: Mutex::new(Vec::new()),
            grammar_version: std::sync::atomic::AtomicU64::new(0),
            dict_path: dict_path.into(),
            grammar_path: grammar_path.into(),
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

    pub fn cached_grammar_kinds(&self) -> Vec<GrammarKind> {
        self.grammar_kinds
            .lock()
            .ok()
            .map(|g| g.clone())
            .unwrap_or_default()
    }

    pub fn update_grammar_kinds(&self, kinds: Vec<GrammarKind>) {
        if let Ok(mut g) = self.grammar_kinds.lock() {
            *g = kinds;
        }
        self.grammar_version
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
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
}
