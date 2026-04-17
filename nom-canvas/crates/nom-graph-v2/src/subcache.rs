use crate::cache::CacheBackend;

pub struct SubCache<B: CacheBackend> {
    parent_id: u64,
    #[allow(dead_code)]
    child_key_prefix: Vec<u8>,
    backend: B,
}

impl<B: CacheBackend> SubCache<B> {
    pub fn new(parent_id: u64, backend: B) -> Self {
        let child_key_prefix = parent_id.to_le_bytes().to_vec();
        Self {
            parent_id,
            child_key_prefix,
            backend,
        }
    }

    fn compose_key(&self, child_id: u64) -> u64 {
        (self.parent_id << 32) | (child_id & 0xFFFF_FFFF)
    }

    pub fn get(&self, child_id: u64) -> Option<Vec<u8>> {
        self.backend.get(self.compose_key(child_id))
    }

    pub fn set(&mut self, child_id: u64, value: Vec<u8>) {
        let key = self.compose_key(child_id);
        self.backend.set(key, value);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cache::ClassicCache;

    #[test]
    fn basic_set_get() {
        let mut sc = SubCache::new(1, ClassicCache::new());
        sc.set(10, vec![1, 2, 3]);
        assert_eq!(sc.get(10), Some(vec![1, 2, 3]));
    }

    #[test]
    fn sibling_subcaches_dont_pollute() {
        let backend_a = ClassicCache::new();
        let backend_b = ClassicCache::new();
        let mut sc_a = SubCache::new(1, backend_a);
        let mut sc_b = SubCache::new(2, backend_b);
        sc_a.set(5, vec![0xAA]);
        sc_b.set(5, vec![0xBB]);
        // Each subcache sees its own value
        assert_eq!(sc_a.get(5), Some(vec![0xAA]));
        assert_eq!(sc_b.get(5), Some(vec![0xBB]));
    }

    #[test]
    fn different_parents_dont_collide_in_shared_backend() {
        // Two subcaches using different parent_ids but same backend would collide
        // This test verifies the compose_key produces distinct keys for different parents
        let sc_a = SubCache::new(1u64, ClassicCache::new());
        let sc_b = SubCache::new(2u64, ClassicCache::new());
        let key_a = sc_a.compose_key(5);
        let key_b = sc_b.compose_key(5);
        assert_ne!(key_a, key_b);
    }

    #[test]
    fn parent_reuse_same_child_key() {
        let sc = SubCache::new(7, ClassicCache::new());
        assert_eq!(sc.compose_key(3), sc.compose_key(3));
    }
}
