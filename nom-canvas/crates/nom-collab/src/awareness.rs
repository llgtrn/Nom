//! Awareness map — tracks per-client presence state with TTL garbage collection.

use std::collections::HashMap;

/// Maps client_id → (state_bytes, last_seen_ms).
pub struct Awareness {
    entries: HashMap<u64, (Vec<u8>, u64)>,
}

impl Awareness {
    pub fn new() -> Self {
        Self { entries: HashMap::new() }
    }

    /// Upsert presence state for a client.  `last_seen_ms` is set to the
    /// current time supplied by the caller.
    pub fn set(&mut self, client_id: u64, state: Vec<u8>) {
        // Callers supply their own clock; store raw state + 0 sentinel until
        // a timestamped variant is needed.
        let ts = self.entries.get(&client_id).map(|e| e.1).unwrap_or(0);
        self.entries.insert(client_id, (state, ts));
    }

    /// Remove a client's presence entry.
    pub fn remove(&mut self, client_id: u64) {
        self.entries.remove(&client_id);
    }

    /// Remove all entries whose last-seen timestamp is older than `ttl_ms`
    /// relative to `now_ms`.
    pub fn gc(&mut self, ttl_ms: u64, now_ms: u64) {
        self.entries.retain(|_, (_, last_seen)| {
            now_ms.saturating_sub(*last_seen) <= ttl_ms
        });
    }

    /// Update last-seen timestamp for a client (called on message receipt).
    pub fn touch(&mut self, client_id: u64, now_ms: u64) {
        if let Some(entry) = self.entries.get_mut(&client_id) {
            entry.1 = now_ms;
        }
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn set_and_remove() {
        let mut aw = Awareness::new();
        aw.set(1, vec![1, 2, 3]);
        assert_eq!(aw.len(), 1);
        aw.remove(1);
        assert!(aw.is_empty());
    }

    #[test]
    fn gc_removes_stale_entries() {
        let mut aw = Awareness::new();
        aw.set(1, vec![]);
        aw.touch(1, 100);
        aw.set(2, vec![]);
        aw.touch(2, 900);
        // ttl = 200ms, now = 1000ms → client 1 last seen at 100, age = 900 > 200 → removed
        aw.gc(200, 1000);
        assert_eq!(aw.len(), 1);
        assert!(!aw.entries.contains_key(&1));
    }

    #[test]
    fn gc_keeps_fresh_entries() {
        let mut aw = Awareness::new();
        aw.set(1, vec![]);
        aw.touch(1, 950);
        aw.gc(200, 1000); // age = 50 ≤ 200 → keep
        assert_eq!(aw.len(), 1);
    }

    #[test]
    fn set_overwrites_state() {
        let mut aw = Awareness::new();
        aw.set(1, vec![1]);
        aw.set(1, vec![2, 3]);
        assert_eq!(aw.entries[&1].0, vec![2, 3]);
    }
}
