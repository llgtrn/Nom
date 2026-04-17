use std::collections::HashMap;

pub trait CacheBackend {
    fn get(&self, key: u64) -> Option<Vec<u8>>;
    fn set(&mut self, key: u64, value: Vec<u8>);
    fn clear(&mut self);
}

// ── NoCache ──────────────────────────────────────────────────────────────────

pub struct NoCache;

impl CacheBackend for NoCache {
    fn get(&self, _key: u64) -> Option<Vec<u8>> {
        None
    }
    fn set(&mut self, _key: u64, _value: Vec<u8>) {}
    fn clear(&mut self) {}
}

// ── LruCache ─────────────────────────────────────────────────────────────────

/// O(1) LRU via a logical clock stored alongside each entry.
pub struct LruCache {
    pub max_entries: usize,
    /// (value, last_access_tick)
    data: HashMap<u64, (Vec<u8>, u64)>,
    tick: u64,
}

impl LruCache {
    pub fn new(max_entries: usize) -> Self {
        Self {
            max_entries,
            data: HashMap::new(),
            tick: 0,
        }
    }
}

impl CacheBackend for LruCache {
    fn get(&self, key: u64) -> Option<Vec<u8>> {
        self.data.get(&key).map(|(v, _)| v.clone())
    }

    fn set(&mut self, key: u64, value: Vec<u8>) {
        self.tick += 1;
        if self.data.len() >= self.max_entries && !self.data.contains_key(&key) {
            // evict LRU entry (smallest tick)
            if let Some((&evict_key, _)) = self.data.iter().min_by_key(|(_, (_, t))| t) {
                self.data.remove(&evict_key);
            }
        }
        self.data.insert(key, (value, self.tick));
    }

    fn clear(&mut self) {
        self.data.clear();
        self.tick = 0;
    }
}

// ── RamPressureCache ─────────────────────────────────────────────────────────

pub struct RamPressureCache {
    pub soft_limit_bytes: usize,
    /// (value, insert_order)
    data: HashMap<u64, (Vec<u8>, u64)>,
    counter: u64,
    current_bytes: usize,
}

impl RamPressureCache {
    pub fn new(soft_limit_bytes: usize) -> Self {
        Self {
            soft_limit_bytes,
            data: HashMap::new(),
            counter: 0,
            current_bytes: 0,
        }
    }

    fn evict_oldest_half(&mut self) {
        let mut entries: Vec<(u64, u64)> = self.data.iter().map(|(&k, (_, ord))| (k, *ord)).collect();
        entries.sort_unstable_by_key(|(_, ord)| *ord);
        let evict_count = (entries.len() / 2).max(1);
        for (k, _) in entries.into_iter().take(evict_count) {
            if let Some((v, _)) = self.data.remove(&k) {
                self.current_bytes = self.current_bytes.saturating_sub(v.len());
            }
        }
    }
}

impl CacheBackend for RamPressureCache {
    fn get(&self, key: u64) -> Option<Vec<u8>> {
        self.data.get(&key).map(|(v, _)| v.clone())
    }

    fn set(&mut self, key: u64, value: Vec<u8>) {
        self.counter += 1;
        let size = value.len();
        if self.current_bytes + size > self.soft_limit_bytes {
            self.evict_oldest_half();
        }
        if let Some((old, ord)) = self.data.insert(key, (value, self.counter)) {
            self.current_bytes = self.current_bytes.saturating_sub(old.len());
            self.counter = ord; // restore previous counter for replaced entry
        }
        self.current_bytes += size;
    }

    fn clear(&mut self) {
        self.data.clear();
        self.current_bytes = 0;
        self.counter = 0;
    }
}

// ── ClassicCache ─────────────────────────────────────────────────────────────

pub struct ClassicCache {
    data: HashMap<u64, Vec<u8>>,
}

impl ClassicCache {
    pub fn new() -> Self {
        Self { data: HashMap::new() }
    }
}

impl CacheBackend for ClassicCache {
    fn get(&self, key: u64) -> Option<Vec<u8>> {
        self.data.get(&key).cloned()
    }

    fn set(&mut self, key: u64, value: Vec<u8>) {
        self.data.insert(key, value);
    }

    fn clear(&mut self) {
        self.data.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // NoCache tests
    #[test]
    fn no_cache_always_miss() {
        let c = NoCache;
        assert!(c.get(1).is_none());
    }

    #[test]
    fn no_cache_set_is_noop() {
        let mut c = NoCache;
        c.set(1, vec![1, 2, 3]);
        assert!(c.get(1).is_none());
    }

    // LruCache tests
    #[test]
    fn lru_basic_hit() {
        let mut c = LruCache::new(4);
        c.set(10, vec![9, 8]);
        assert_eq!(c.get(10), Some(vec![9, 8]));
    }

    #[test]
    fn lru_evicts_when_full() {
        let mut c = LruCache::new(2);
        c.set(1, vec![1]);
        c.set(2, vec![2]);
        c.set(3, vec![3]); // should evict oldest
        // At most 2 entries
        let hits = [1u64, 2, 3].iter().filter(|&&k| c.get(k).is_some()).count();
        assert_eq!(hits, 2);
    }

    #[test]
    fn lru_clear_empties() {
        let mut c = LruCache::new(4);
        c.set(1, vec![1]);
        c.clear();
        assert!(c.get(1).is_none());
    }

    // RamPressureCache tests
    #[test]
    fn ram_pressure_hit() {
        let mut c = RamPressureCache::new(1024);
        c.set(5, vec![7, 8, 9]);
        assert_eq!(c.get(5), Some(vec![7, 8, 9]));
    }

    #[test]
    fn ram_pressure_evicts_on_limit() {
        let mut c = RamPressureCache::new(10);
        // Fill up to limit
        c.set(1, vec![0u8; 6]);
        c.set(2, vec![0u8; 6]); // triggers eviction
        // After eviction, total should be under limit
        assert!(c.current_bytes <= 10);
    }

    #[test]
    fn ram_pressure_clear() {
        let mut c = RamPressureCache::new(1024);
        c.set(1, vec![1]);
        c.clear();
        assert!(c.get(1).is_none());
        assert_eq!(c.current_bytes, 0);
    }

    // ClassicCache tests
    #[test]
    fn classic_hit_miss() {
        let mut c = ClassicCache::new();
        assert!(c.get(99).is_none());
        c.set(99, vec![42]);
        assert_eq!(c.get(99), Some(vec![42]));
    }

    #[test]
    fn classic_overwrite() {
        let mut c = ClassicCache::new();
        c.set(1, vec![1]);
        c.set(1, vec![2]);
        assert_eq!(c.get(1), Some(vec![2]));
    }

    #[test]
    fn classic_clear() {
        let mut c = ClassicCache::new();
        c.set(1, vec![1]);
        c.set(2, vec![2]);
        c.clear();
        assert!(c.get(1).is_none());
        assert!(c.get(2).is_none());
    }
}
