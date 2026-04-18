/// A single ancestor entry pairing a node id with its depth in the ancestry chain.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AncestorEntry {
    /// Node identifier.
    pub id: u32,
    /// Depth from the root (0 = root).
    pub depth: u32,
}

/// Cache of ancestor entries up to a configurable maximum depth.
#[derive(Debug, Default)]
pub struct AncestryCache {
    entries: Vec<AncestorEntry>,
    max_depth: u32,
}

impl AncestryCache {
    /// Create a new cache that discards entries deeper than `max_depth`.
    pub fn new(max_depth: u32) -> Self {
        Self {
            entries: Vec::new(),
            max_depth,
        }
    }

    /// Insert `id` at `depth`.  If `depth > max_depth` the entry is silently
    /// dropped.  If the `id` already exists its depth is updated in place.
    pub fn insert(&mut self, id: u32, depth: u32) {
        if depth > self.max_depth {
            return;
        }
        if let Some(e) = self.entries.iter_mut().find(|e| e.id == id) {
            e.depth = depth;
        } else {
            self.entries.push(AncestorEntry { id, depth });
        }
    }

    /// Return the recorded depth of `id`, or `None` if not present.
    pub fn get(&self, id: u32) -> Option<u32> {
        self.entries.iter().find(|e| e.id == id).map(|e| e.depth)
    }

    /// Return all ids whose depth equals `d`.
    pub fn at_depth(&self, d: u32) -> Vec<u32> {
        self.entries
            .iter()
            .filter(|e| e.depth == d)
            .map(|e| e.id)
            .collect()
    }

    /// Number of entries currently cached.
    pub fn count(&self) -> usize {
        self.entries.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn insert_and_get() {
        let mut c = AncestryCache::new(5);
        c.insert(1, 2);
        assert_eq!(c.get(1), Some(2));
    }

    #[test]
    fn depth_filter() {
        let mut c = AncestryCache::new(3);
        c.insert(10, 1);
        c.insert(20, 2);
        assert_eq!(c.get(10), Some(1));
        assert_eq!(c.get(20), Some(2));
    }

    #[test]
    fn at_depth() {
        let mut c = AncestryCache::new(10);
        c.insert(1, 3);
        c.insert(2, 3);
        c.insert(3, 5);
        let mut ids = c.at_depth(3);
        ids.sort();
        assert_eq!(ids, vec![1, 2]);
    }

    #[test]
    fn max_depth_guard() {
        let mut c = AncestryCache::new(2);
        c.insert(99, 3); // exceeds max_depth → dropped
        assert_eq!(c.get(99), None);
        assert_eq!(c.count(), 0);
    }

    #[test]
    fn count() {
        let mut c = AncestryCache::new(10);
        c.insert(1, 0);
        c.insert(2, 1);
        c.insert(3, 2);
        assert_eq!(c.count(), 3);
    }

    #[test]
    fn reinsert_updates() {
        let mut c = AncestryCache::new(10);
        c.insert(7, 1);
        c.insert(7, 4); // update in place
        assert_eq!(c.get(7), Some(4));
        assert_eq!(c.count(), 1); // still only one entry
    }
}
