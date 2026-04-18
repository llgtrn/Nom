/// Accumulated statistics for a single cache instance.
#[derive(Debug, Clone, PartialEq)]
pub struct CacheStats {
    pub hits: u64,
    pub misses: u64,
    pub evictions: u64,
    pub total_entries: usize,
}

impl CacheStats {
    /// Create a fresh, zeroed-out stats object.
    pub fn new() -> Self {
        Self {
            hits: 0,
            misses: 0,
            evictions: 0,
            total_entries: 0,
        }
    }

    /// Record a cache hit.
    pub fn record_hit(&mut self) {
        self.hits += 1;
    }

    /// Record a cache miss.
    pub fn record_miss(&mut self) {
        self.misses += 1;
    }

    /// Record an eviction.
    pub fn record_eviction(&mut self) {
        self.evictions += 1;
    }

    /// Hit rate: `hits / (hits + misses)`. Returns `0.0` when both are zero.
    pub fn hit_rate(&self) -> f32 {
        let total = self.hits + self.misses;
        if total == 0 {
            0.0
        } else {
            self.hits as f32 / total as f32
        }
    }

    /// Total number of lookups (hits + misses).
    pub fn total_lookups(&self) -> u64 {
        self.hits + self.misses
    }
}

impl Default for CacheStats {
    fn default() -> Self {
        Self::new()
    }
}

/// A point-in-time snapshot of a cache's stats plus its configuration.
#[derive(Debug, Clone, PartialEq)]
pub struct CacheSnapshot {
    pub stats: CacheStats,
    pub policy_name: String,
    pub capacity: usize,
}

impl CacheSnapshot {
    /// Construct a snapshot.
    pub fn new(stats: CacheStats, policy_name: &str, capacity: usize) -> Self {
        Self {
            stats,
            policy_name: policy_name.to_owned(),
            capacity,
        }
    }

    /// A cache is considered healthy when either:
    ///  - no lookups have been performed yet (`total_lookups == 0`), or
    ///  - the hit rate exceeds 50 %.
    pub fn is_healthy(&self) -> bool {
        self.stats.total_lookups() == 0 || self.stats.hit_rate() > 0.5
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stats_new() {
        let s = CacheStats::new();
        assert_eq!(s.hits, 0);
        assert_eq!(s.misses, 0);
        assert_eq!(s.evictions, 0);
        assert_eq!(s.total_entries, 0);
    }

    #[test]
    fn record_hit_miss() {
        let mut s = CacheStats::new();
        s.record_hit();
        s.record_hit();
        s.record_miss();
        s.record_eviction();
        assert_eq!(s.hits, 2);
        assert_eq!(s.misses, 1);
        assert_eq!(s.evictions, 1);
    }

    #[test]
    fn hit_rate_empty() {
        let s = CacheStats::new();
        assert_eq!(s.hit_rate(), 0.0);
        assert_eq!(s.total_lookups(), 0);
    }

    #[test]
    fn hit_rate_with_data() {
        let mut s = CacheStats::new();
        s.record_hit();
        s.record_hit();
        s.record_miss();
        s.record_miss();
        // 2 hits / 4 total = 0.5
        assert!((s.hit_rate() - 0.5).abs() < 1e-6);
        assert_eq!(s.total_lookups(), 4);
    }

    #[test]
    fn snapshot_healthy() {
        // No lookups yet → healthy by definition.
        let snap = CacheSnapshot::new(CacheStats::new(), "lru", 128);
        assert!(snap.is_healthy());
    }

    #[test]
    fn snapshot_unhealthy() {
        // hit_rate = 1/4 = 0.25, which is <= 0.5 → unhealthy.
        let mut s = CacheStats::new();
        s.record_hit();
        s.record_miss();
        s.record_miss();
        s.record_miss();
        let snap = CacheSnapshot::new(s, "lru", 64);
        assert!(!snap.is_healthy());
    }
}
