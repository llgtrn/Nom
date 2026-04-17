#![deny(unsafe_code)]
use std::collections::HashMap;
use crate::constraint::Constraint;
use crate::hash::Hash128;
use crate::tracked::TrackedSnapshot;

/// A single cached computation result
pub struct CachedResult<T: Clone> {
    pub value: T,
    pub constraint: Constraint,
    pub hash: Hash128,
}

/// Memoization cache: key (Hash128) → (T, Constraint)
/// Validates constraint before returning cached value
pub struct MemoCache<T: Clone> {
    entries: HashMap<u64, CachedResult<T>>,
    hit_count: u64,
    miss_count: u64,
}

impl<T: Clone> MemoCache<T> {
    pub fn new() -> Self {
        Self { entries: HashMap::new(), hit_count: 0, miss_count: 0 }
    }

    /// Try to retrieve a cached result. Validates constraint before returning.
    /// `current_snapshots` holds fresh (method_id, return_hash) snapshots for validation.
    pub fn get(&mut self, key: &Hash128, current_input_hash: u64, current_snapshots: &[TrackedSnapshot]) -> Option<T> {
        let entry = self.entries.get(&key.as_u64())?;
        if entry.constraint.validate(current_input_hash, current_snapshots) {
            self.hit_count += 1;
            Some(entry.value.clone())
        } else {
            self.miss_count += 1;
            None
        }
    }

    pub fn put(&mut self, key: Hash128, value: T, constraint: Constraint) {
        self.entries.insert(key.as_u64(), CachedResult { value, constraint, hash: key });
    }

    pub fn invalidate(&mut self, key: &Hash128) {
        self.entries.remove(&key.as_u64());
    }

    pub fn clear(&mut self) { self.entries.clear(); }
    pub fn len(&self) -> usize { self.entries.len() }
    pub fn hit_count(&self) -> u64 { self.hit_count }
    pub fn miss_count(&self) -> u64 { self.miss_count }
    pub fn hit_rate(&self) -> f64 {
        let total = self.hit_count + self.miss_count;
        if total == 0 { 0.0 } else { self.hit_count as f64 / total as f64 }
    }
}

impl<T: Clone> Default for MemoCache<T> { fn default() -> Self { Self::new() } }

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn memo_cache_hit() {
        let mut cache: MemoCache<String> = MemoCache::new();
        let key = Hash128::of_str("f(x)");
        let constraint = Constraint::new(42);
        cache.put(key, "result".into(), constraint);
        let result = cache.get(&key, 42, &[]);
        assert_eq!(result, Some("result".into()));
        assert_eq!(cache.hit_count(), 1);
    }

    #[test]
    fn memo_cache_miss_on_stale_input() {
        let mut cache: MemoCache<u64> = MemoCache::new();
        let key = Hash128::of_str("g(x)");
        let constraint = Constraint::new(100);
        cache.put(key, 999, constraint);
        // input hash changed: 100 → 200
        let result = cache.get(&key, 200, &[]);
        assert_eq!(result, None);
        assert_eq!(cache.miss_count(), 1);
    }
}
