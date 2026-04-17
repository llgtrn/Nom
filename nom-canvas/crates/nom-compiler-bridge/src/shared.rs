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
        let text_hash = source_text.bytes().fold(0u64, |acc, b| {
            acc.wrapping_mul(31).wrapping_add(b as u64)
        });
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
        self.grammar_version.load(std::sync::atomic::Ordering::Relaxed)
    }

    pub fn cached_grammar_kinds(&self) -> Vec<GrammarKind> {
        self.grammar_kinds.lock().ok().map(|g| g.clone()).unwrap_or_default()
    }

    pub fn update_grammar_kinds(&self, kinds: Vec<GrammarKind>) {
        if let Ok(mut g) = self.grammar_kinds.lock() {
            *g = kinds;
        }
        self.grammar_version.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
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
        let output = PipelineOutput { source_hash: key, grammar_version: 0, output_json: "{}".into() };
        state.cache_compile_result(key, output.clone());
        let result = state.get_cached_compile(key);
        assert!(result.is_some());
        assert_eq!(result.unwrap().output_json, "{}");
    }

    #[test]
    fn grammar_kinds_update() {
        let state = SharedState::new("a", "b");
        assert_eq!(state.grammar_version(), 0);
        state.update_grammar_kinds(vec![
            GrammarKind { name: "verb".into(), description: "action".into() },
        ]);
        assert_eq!(state.grammar_version(), 1);
        let kinds = state.cached_grammar_kinds();
        assert_eq!(kinds.len(), 1);
        assert_eq!(kinds[0].name, "verb");
    }
}
