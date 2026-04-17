use std::collections::HashMap;
use crate::diagnostic::Diagnostic;

/// A cache keyed by (file_hash, rule_set_hash, ast_node_hash).
pub struct LintCache {
    entries: HashMap<(u64, u64, u64), Vec<Diagnostic>>,
}

impl LintCache {
    pub fn new() -> Self {
        Self { entries: HashMap::new() }
    }

    pub fn get(&self, file_hash: u64, rule_set_hash: u64, ast_node_hash: u64) -> Option<&[Diagnostic]> {
        self.entries.get(&(file_hash, rule_set_hash, ast_node_hash)).map(|v| v.as_slice())
    }

    pub fn insert(&mut self, file_hash: u64, rule_set_hash: u64, ast_node_hash: u64, diags: Vec<Diagnostic>) {
        self.entries.insert((file_hash, rule_set_hash, ast_node_hash), diags);
    }

    /// Remove all entries whose file_hash matches.
    pub fn invalidate_file(&mut self, file_hash: u64) {
        self.entries.retain(|k, _| k.0 != file_hash);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::diagnostic::{Diagnostic, Severity};
    use crate::span::Span;

    fn make_diag(code: &'static str) -> Diagnostic {
        Diagnostic {
            span: Span::new(0, 1),
            severity: Severity::Info,
            code,
            message: "test".to_string(),
            fix: None,
        }
    }

    #[test]
    fn miss_on_empty_cache() {
        let cache = LintCache::new();
        assert!(cache.get(1, 2, 3).is_none());
    }

    #[test]
    fn insert_then_hit() {
        let mut cache = LintCache::new();
        cache.insert(1, 2, 3, vec![make_diag("C001")]);
        let hit = cache.get(1, 2, 3).unwrap();
        assert_eq!(hit.len(), 1);
        assert_eq!(hit[0].code, "C001");
    }

    #[test]
    fn different_keys_do_not_collide() {
        let mut cache = LintCache::new();
        cache.insert(1, 2, 3, vec![make_diag("A")]);
        cache.insert(1, 2, 4, vec![make_diag("B")]);
        assert_eq!(cache.get(1, 2, 3).unwrap()[0].code, "A");
        assert_eq!(cache.get(1, 2, 4).unwrap()[0].code, "B");
    }

    #[test]
    fn invalidate_file_removes_entries() {
        let mut cache = LintCache::new();
        cache.insert(10, 1, 1, vec![make_diag("X")]);
        cache.insert(10, 1, 2, vec![make_diag("Y")]);
        cache.insert(20, 1, 1, vec![make_diag("Z")]);
        cache.invalidate_file(10);
        assert!(cache.get(10, 1, 1).is_none());
        assert!(cache.get(10, 1, 2).is_none());
        assert!(cache.get(20, 1, 1).is_some());
    }

    #[test]
    fn invalidate_nonexistent_file_is_noop() {
        let mut cache = LintCache::new();
        cache.insert(1, 1, 1, vec![make_diag("D")]);
        cache.invalidate_file(99); // should not panic
        assert!(cache.get(1, 1, 1).is_some());
    }
}
