use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq)]
pub enum CacheStrategy {
    None,
    LruBounded { max_size: usize },
    ContentAddressed,
}

impl CacheStrategy {
    pub fn strategy_name(&self) -> &str {
        match self {
            CacheStrategy::None => "none",
            CacheStrategy::LruBounded { .. } => "lru_bounded",
            CacheStrategy::ContentAddressed => "content_addressed",
        }
    }

    pub fn is_enabled(&self) -> bool {
        !matches!(self, CacheStrategy::None)
    }
}

#[derive(Debug, Clone)]
pub struct PlanCacheEntry {
    pub plan_hash: u64,
    pub result_hash: u64,
    pub hit_count: u32,
}

impl PlanCacheEntry {
    pub fn new(plan_hash: u64, result_hash: u64) -> Self {
        Self {
            plan_hash,
            result_hash,
            hit_count: 0,
        }
    }

    pub fn record_hit(&mut self) {
        self.hit_count += 1;
    }
}

#[derive(Debug)]
pub struct ExecutionCache {
    pub strategy: CacheStrategy,
    pub entries: HashMap<u64, PlanCacheEntry>,
}

impl ExecutionCache {
    pub fn new(strategy: CacheStrategy) -> Self {
        Self {
            strategy,
            entries: HashMap::new(),
        }
    }

    pub fn insert(&mut self, plan_hash: u64, result_hash: u64) {
        self.entries
            .insert(plan_hash, PlanCacheEntry::new(plan_hash, result_hash));
    }

    pub fn get(&self, plan_hash: u64) -> Option<&PlanCacheEntry> {
        self.entries.get(&plan_hash)
    }

    pub fn hit_rate(&self) -> f32 {
        if self.entries.is_empty() {
            return 0.0;
        }
        let total_hits: u32 = self.entries.values().map(|e| e.hit_count).sum();
        total_hits as f32 / self.entries.len() as f32
    }

    pub fn evict_lru(&mut self, keep: usize) {
        if self.entries.len() <= keep {
            return;
        }
        let mut keys_by_hits: Vec<(u64, u32)> = self
            .entries
            .iter()
            .map(|(&k, v)| (k, v.hit_count))
            .collect();
        // Sort descending by hit_count
        keys_by_hits.sort_by(|a, b| b.1.cmp(&a.1));
        let keep_keys: std::collections::HashSet<u64> =
            keys_by_hits.iter().take(keep).map(|(k, _)| *k).collect();
        self.entries.retain(|k, _| keep_keys.contains(k));
    }
}

pub struct DependencyOrder;

impl DependencyOrder {
    /// Kahn's algorithm topological sort.
    /// Input: slice of (node_id, dependencies).
    /// Returns sorted order, or empty vec if a cycle is detected.
    pub fn topological_sort(nodes: &[(u64, Vec<u64>)]) -> Vec<u64> {
        let mut in_degree: HashMap<u64, usize> = HashMap::new();
        let mut adj: HashMap<u64, Vec<u64>> = HashMap::new();

        // Initialize all nodes
        for (id, _) in nodes {
            in_degree.entry(*id).or_insert(0);
            adj.entry(*id).or_insert_with(Vec::new);
        }

        // Build adjacency (dep -> id) and in-degrees
        for (id, deps) in nodes {
            for dep in deps {
                in_degree.entry(*dep).or_insert(0);
                adj.entry(*dep).or_insert_with(Vec::new).push(*id);
                *in_degree.entry(*id).or_insert(0) += 1;
            }
        }

        let mut queue: std::collections::VecDeque<u64> = in_degree
            .iter()
            .filter(|(_, &d)| d == 0)
            .map(|(&k, _)| k)
            .collect();

        // Stable sort for determinism
        let mut queue_vec: Vec<u64> = queue.drain(..).collect();
        queue_vec.sort_unstable();
        queue = queue_vec.into_iter().collect();

        let mut result = Vec::new();

        while let Some(node) = queue.pop_front() {
            result.push(node);
            if let Some(neighbors) = adj.get(&node) {
                let mut next: Vec<u64> = Vec::new();
                for &neighbor in neighbors {
                    let deg = in_degree.entry(neighbor).or_insert(0);
                    *deg -= 1;
                    if *deg == 0 {
                        next.push(neighbor);
                    }
                }
                next.sort_unstable();
                for n in next {
                    queue.push_back(n);
                }
            }
        }

        if result.len() != in_degree.len() {
            // Cycle detected
            return vec![];
        }

        result
    }

    pub fn has_cycle(nodes: &[(u64, Vec<u64>)]) -> bool {
        let sorted = Self::topological_sort(nodes);
        sorted.len() < nodes.len()
    }
}

#[cfg(test)]
mod plan_cache_tests {
    use super::*;

    // Test 1: CacheStrategy::is_enabled()
    #[test]
    fn test_cache_strategy_is_enabled() {
        assert!(!CacheStrategy::None.is_enabled());
        assert!(CacheStrategy::LruBounded { max_size: 10 }.is_enabled());
        assert!(CacheStrategy::ContentAddressed.is_enabled());
    }

    // Test 2: PlanCacheEntry::record_hit()
    #[test]
    fn test_plan_cache_entry_record_hit() {
        let mut entry = PlanCacheEntry::new(1, 2);
        assert_eq!(entry.hit_count, 0);
        entry.record_hit();
        assert_eq!(entry.hit_count, 1);
        entry.record_hit();
        entry.record_hit();
        assert_eq!(entry.hit_count, 3);
    }

    // Test 3: ExecutionCache::insert() + get()
    #[test]
    fn test_execution_cache_insert_and_get() {
        let mut cache = ExecutionCache::new(CacheStrategy::ContentAddressed);
        cache.insert(42, 99);
        let entry = cache.get(42).expect("entry should exist");
        assert_eq!(entry.plan_hash, 42);
        assert_eq!(entry.result_hash, 99);
    }

    // Test 4: get() returns None for missing key
    #[test]
    fn test_execution_cache_get_missing() {
        let cache = ExecutionCache::new(CacheStrategy::ContentAddressed);
        assert!(cache.get(999).is_none());
    }

    // Test 5: hit_rate() with entries
    #[test]
    fn test_hit_rate_with_entries() {
        let mut cache = ExecutionCache::new(CacheStrategy::ContentAddressed);
        cache.insert(1, 10);
        cache.insert(2, 20);
        // Manually bump hit counts
        cache.entries.get_mut(&1).unwrap().record_hit();
        cache.entries.get_mut(&1).unwrap().record_hit();
        cache.entries.get_mut(&2).unwrap().record_hit();
        // total hits = 3, entries = 2, hit_rate = 1.5
        let rate = cache.hit_rate();
        assert!((rate - 1.5).abs() < 1e-6, "expected 1.5 got {rate}");
    }

    // Test 6: evict_lru() keeps most-hit
    #[test]
    fn test_evict_lru_keeps_most_hit() {
        let mut cache = ExecutionCache::new(CacheStrategy::LruBounded { max_size: 2 });
        cache.insert(1, 10);
        cache.insert(2, 20);
        cache.insert(3, 30);
        // Give entry 3 the most hits, entry 2 second
        for _ in 0..5 {
            cache.entries.get_mut(&3).unwrap().record_hit();
        }
        for _ in 0..2 {
            cache.entries.get_mut(&2).unwrap().record_hit();
        }
        // entry 1 has 0 hits

        cache.evict_lru(2);
        assert_eq!(cache.entries.len(), 2);
        assert!(cache.entries.contains_key(&3), "key 3 (most hits) should survive");
        assert!(cache.entries.contains_key(&2), "key 2 (second most) should survive");
        assert!(!cache.entries.contains_key(&1), "key 1 (no hits) should be evicted");
    }

    // Test 7: topological_sort() linear chain
    #[test]
    fn test_topological_sort_linear_chain() {
        // 1 -> 2 -> 3  (1 depends on nothing, 2 depends on 1, 3 depends on 2)
        let nodes: Vec<(u64, Vec<u64>)> = vec![
            (1, vec![]),
            (2, vec![1]),
            (3, vec![2]),
        ];
        let order = DependencyOrder::topological_sort(&nodes);
        assert_eq!(order, vec![1, 2, 3]);
    }

    // Test 8: topological_sort() with no deps
    #[test]
    fn test_topological_sort_no_deps() {
        let nodes: Vec<(u64, Vec<u64>)> = vec![
            (10, vec![]),
            (20, vec![]),
            (30, vec![]),
        ];
        let order = DependencyOrder::topological_sort(&nodes);
        assert_eq!(order.len(), 3);
        // All nodes present (order among independents is deterministic but we just check membership)
        assert!(order.contains(&10));
        assert!(order.contains(&20));
        assert!(order.contains(&30));
    }

    // Test 9: has_cycle() false for DAG
    #[test]
    fn test_has_cycle_false_for_dag() {
        let nodes: Vec<(u64, Vec<u64>)> = vec![
            (1, vec![]),
            (2, vec![1]),
            (3, vec![1]),
            (4, vec![2, 3]),
        ];
        assert!(!DependencyOrder::has_cycle(&nodes));
    }

    // Test 10: has_cycle() true for cycle
    #[test]
    fn test_has_cycle_true_for_cycle() {
        // 1 -> 2 -> 3 -> 1  (circular)
        let nodes: Vec<(u64, Vec<u64>)> = vec![
            (1, vec![3]),
            (2, vec![1]),
            (3, vec![2]),
        ];
        assert!(DependencyOrder::has_cycle(&nodes));
    }
}
