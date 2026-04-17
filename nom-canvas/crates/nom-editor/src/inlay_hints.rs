//! Inlay-hint cache keyed on byte ranges.  Debounces LSP fetches separately
//! for edits (fast invalidate) and scrolls (slow refresh).
#![deny(unsafe_code)]

use std::collections::HashMap;
use std::ops::Range;
use std::time::{Duration, Instant};

use crate::lsp_bridge::InlayHint;

#[derive(Default)]
pub struct HintCache {
    /// Stored hints, keyed by (uri, start, end) for the range fetched.
    entries: HashMap<(String, usize, usize), (Vec<InlayHint>, Instant)>,
    ttl: Duration,
}

impl HintCache {
    pub fn new(ttl: Duration) -> Self {
        Self {
            entries: HashMap::new(),
            ttl,
        }
    }

    pub fn insert(&mut self, uri: impl Into<String>, range: Range<usize>, hints: Vec<InlayHint>) {
        let key = (uri.into(), range.start, range.end);
        self.entries.insert(key, (hints, Instant::now()));
    }

    pub fn get(&self, uri: &str, range: Range<usize>) -> Option<&[InlayHint]> {
        let key = (uri.to_owned(), range.start, range.end);
        self.entries.get(&key).and_then(|(hints, stored_at)| {
            if stored_at.elapsed() < self.ttl {
                Some(hints.as_slice())
            } else {
                None
            }
        })
    }

    pub fn invalidate_overlapping(&mut self, uri: &str, range: Range<usize>) {
        self.entries.retain(|(u, start, end), _| {
            if u != uri {
                return true;
            }
            // Keep entries that don't overlap with the given range.
            // Two ranges overlap unless one ends before the other starts.
            let no_overlap = *end <= range.start || *start >= range.end;
            no_overlap
        });
    }

    pub fn clear(&mut self, uri: &str) {
        self.entries.retain(|(u, _, _), _| u != uri);
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}

pub struct DebouncedRequest {
    last_edit: Option<Instant>,
    edit_debounce: Duration,
    scroll_debounce: Duration,
}

impl DebouncedRequest {
    pub fn new(edit_debounce_ms: u64, scroll_debounce_ms: u64) -> Self {
        Self {
            last_edit: None,
            edit_debounce: Duration::from_millis(edit_debounce_ms),
            scroll_debounce: Duration::from_millis(scroll_debounce_ms),
        }
    }

    pub fn record_edit(&mut self) {
        self.last_edit = Some(Instant::now());
    }

    /// Returns true when enough time has passed since the last edit to safely
    /// fetch inlay hints (edit debounce window elapsed).
    pub fn should_fetch_after_edit(&self) -> bool {
        match self.last_edit {
            Some(t) => t.elapsed() >= self.edit_debounce,
            None => true,
        }
    }

    /// Returns true when no edit occurred within the scroll debounce window,
    /// making it safe to refresh hints on scroll.
    pub fn should_fetch_after_scroll(&self) -> bool {
        match self.last_edit {
            Some(t) => t.elapsed() >= self.scroll_debounce,
            None => true,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lsp_bridge::{InlayHint, InlayHintKind};

    fn make_hint(offset: usize) -> InlayHint {
        InlayHint { offset, label: format!(": T{}", offset), kind: InlayHintKind::Type }
    }

    #[test]
    fn hint_cache_new_ttl_and_empty() {
        let cache = HintCache::new(Duration::from_secs(5));
        assert_eq!(cache.len(), 0);
        assert!(cache.is_empty());
        assert_eq!(cache.ttl, Duration::from_secs(5));
    }

    #[test]
    fn insert_and_get_roundtrip() {
        let mut cache = HintCache::new(Duration::from_secs(60));
        let hints = vec![make_hint(10), make_hint(20)];
        cache.insert("file://doc", 0..100, hints.clone());
        let got = cache.get("file://doc", 0..100).unwrap();
        assert_eq!(got.len(), 2);
        assert_eq!(got[0].offset, 10);
    }

    #[test]
    fn get_miss_on_different_range() {
        let mut cache = HintCache::new(Duration::from_secs(60));
        cache.insert("file://doc", 0..100, vec![make_hint(5)]);
        assert!(cache.get("file://doc", 0..50).is_none());
        assert!(cache.get("file://doc", 50..100).is_none());
        assert!(cache.get("file://other", 0..100).is_none());
    }

    #[test]
    #[ignore = "sleeps 5ms to age out TTL"]
    fn get_returns_none_past_ttl() {
        let mut cache = HintCache::new(Duration::from_millis(1));
        cache.insert("file://doc", 0..100, vec![make_hint(5)]);
        std::thread::sleep(Duration::from_millis(5));
        assert!(cache.get("file://doc", 0..100).is_none());
    }

    #[test]
    fn get_returns_none_past_ttl_direct_mutation() {
        // Age the entry by using a very short TTL cache and directly overwriting
        // the stored Instant with one from the past via a helper entry.
        let mut cache = HintCache::new(Duration::from_millis(1));
        cache.insert("file://doc", 0..100, vec![make_hint(5)]);
        // Overwrite the entry with an already-elapsed instant.
        let key = ("file://doc".to_owned(), 0usize, 100usize);
        let past = Instant::now() - Duration::from_secs(10);
        cache.entries.insert(key, (vec![make_hint(5)], past));
        assert!(cache.get("file://doc", 0..100).is_none());
    }

    #[test]
    fn invalidate_overlapping_removes_matching() {
        let mut cache = HintCache::new(Duration::from_secs(60));
        cache.insert("file://doc", 0..50, vec![make_hint(10)]);
        cache.insert("file://doc", 40..90, vec![make_hint(45)]);
        cache.insert("file://doc", 200..300, vec![make_hint(250)]);
        cache.invalidate_overlapping("file://doc", 30..60);
        // 0..50 overlaps 30..60, 40..90 overlaps 30..60, 200..300 does not.
        assert_eq!(cache.len(), 1);
        assert!(cache.get("file://doc", 200..300).is_some());
    }

    #[test]
    fn invalidate_overlapping_keeps_non_overlapping() {
        let mut cache = HintCache::new(Duration::from_secs(60));
        cache.insert("file://doc", 0..20, vec![make_hint(5)]);
        cache.insert("file://doc", 80..100, vec![make_hint(90)]);
        cache.invalidate_overlapping("file://doc", 30..60);
        assert_eq!(cache.len(), 2);
    }

    #[test]
    fn clear_wipes_one_uri() {
        let mut cache = HintCache::new(Duration::from_secs(60));
        cache.insert("file://a", 0..100, vec![make_hint(1)]);
        cache.insert("file://b", 0..100, vec![make_hint(2)]);
        cache.clear("file://a");
        assert_eq!(cache.len(), 1);
        assert!(cache.get("file://a", 0..100).is_none());
        assert!(cache.get("file://b", 0..100).is_some());
    }

    #[test]
    fn debounced_initial_should_fetch_true() {
        let req = DebouncedRequest::new(300, 1000);
        assert!(req.should_fetch_after_edit());
        assert!(req.should_fetch_after_scroll());
    }

    #[test]
    fn record_edit_makes_immediate_fetch_false() {
        let mut req = DebouncedRequest::new(300, 1000);
        req.record_edit();
        // Immediately after edit, debounce window not elapsed.
        assert!(!req.should_fetch_after_edit());
        assert!(!req.should_fetch_after_scroll());
    }

    #[test]
    fn zero_ms_debounce_always_true_after_record_edit() {
        let mut req = DebouncedRequest::new(0, 0);
        req.record_edit();
        // 0ms debounce: elapsed >= 0 is always true.
        assert!(req.should_fetch_after_edit());
        assert!(req.should_fetch_after_scroll());
    }
}
