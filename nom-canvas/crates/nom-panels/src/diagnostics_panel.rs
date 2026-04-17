//! Diagnostics panel view-model.
//!
//! Collects `DiagnosticEntry` items from multiple sources (compiler, linter,
//! LSP), groups them by severity + source file, supports filtering +
//! "jump to" navigation.
#![deny(unsafe_code)]

use std::collections::HashMap;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Ord, PartialOrd)]
pub enum DiagSeverity { Error, Warning, Info, Hint }

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum DiagSource { Compiler, Linter, Lsp, Typechecker, Security }

#[derive(Clone, Debug, PartialEq)]
pub struct DiagnosticEntry {
    pub id: u64,
    pub severity: DiagSeverity,
    pub source: DiagSource,
    pub file_path: String,
    pub line: u32,
    pub column: u32,
    pub code: Option<String>,
    pub message: String,
}

#[derive(Default)]
pub struct DiagnosticsPanel {
    entries: Vec<DiagnosticEntry>,
    next_id: u64,
    pub severity_filter: Option<DiagSeverity>,
    pub source_filter: Option<DiagSource>,
    pub file_filter: Option<String>,
}

impl DiagnosticsPanel {
    pub fn new() -> Self { Self::default() }

    pub fn push(&mut self, entry: DiagnosticEntry) -> u64 {
        let id = self.next_id;
        self.next_id += 1;
        let mut entry = entry;
        entry.id = id;
        self.entries.push(entry);
        id
    }

    pub fn clear(&mut self) { self.entries.clear(); }
    pub fn clear_source(&mut self, source: DiagSource) { self.entries.retain(|e| e.source != source); }
    pub fn len(&self) -> usize { self.entries.len() }
    pub fn is_empty(&self) -> bool { self.entries.is_empty() }

    /// Filtered view with panel's current filter settings.
    pub fn filtered(&self) -> Vec<&DiagnosticEntry> {
        self.entries.iter()
            .filter(|e| self.severity_filter.map(|s| s == e.severity).unwrap_or(true))
            .filter(|e| self.source_filter.map(|s| s == e.source).unwrap_or(true))
            .filter(|e| self.file_filter.as_deref().map(|f| e.file_path == f).unwrap_or(true))
            .collect()
    }

    /// Counts by severity over all entries (ignores filters).
    pub fn counts_by_severity(&self) -> HashMap<DiagSeverity, usize> {
        let mut out: HashMap<DiagSeverity, usize> = HashMap::new();
        for e in &self.entries {
            *out.entry(e.severity).or_insert(0) += 1;
        }
        out
    }

    /// Group filtered entries by file path (preserving insertion order
    /// within each group).
    pub fn grouped_by_file(&self) -> Vec<(String, Vec<&DiagnosticEntry>)> {
        let mut order: Vec<String> = Vec::new();
        let mut map: HashMap<String, Vec<&DiagnosticEntry>> = HashMap::new();
        for e in self.filtered() {
            if !map.contains_key(&e.file_path) {
                order.push(e.file_path.clone());
            }
            map.entry(e.file_path.clone()).or_default().push(e);
        }
        order.into_iter().map(|k| { let v = map.remove(&k).unwrap(); (k, v) }).collect()
    }

    /// Next diagnostic after the current cursor position (same file if any,
    /// else any file).  Used for F8-style navigation.
    pub fn next_after(&self, file: &str, line: u32) -> Option<&DiagnosticEntry> {
        // First try same file after the line.
        let same_file: Option<&DiagnosticEntry> = self.entries.iter()
            .filter(|e| e.file_path == file && e.line > line)
            .min_by_key(|e| e.line);
        if let Some(e) = same_file { return Some(e); }
        // Otherwise return the first entry in any other file (sorted).
        self.entries.iter()
            .filter(|e| e.file_path != file)
            .min_by(|a, b| a.file_path.cmp(&b.file_path).then(a.line.cmp(&b.line)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_entry(severity: DiagSeverity, source: DiagSource, file: &str, line: u32) -> DiagnosticEntry {
        DiagnosticEntry {
            id: 0,
            severity,
            source,
            file_path: file.to_string(),
            line,
            column: 1,
            code: None,
            message: "test message".to_string(),
        }
    }

    #[test]
    fn new_is_empty() {
        let panel = DiagnosticsPanel::new();
        assert!(panel.is_empty());
        assert_eq!(panel.len(), 0);
    }

    #[test]
    fn push_assigns_incrementing_ids() {
        let mut panel = DiagnosticsPanel::new();
        let id0 = panel.push(make_entry(DiagSeverity::Error, DiagSource::Compiler, "a.nom", 1));
        let id1 = panel.push(make_entry(DiagSeverity::Warning, DiagSource::Linter, "b.nom", 2));
        let id2 = panel.push(make_entry(DiagSeverity::Info, DiagSource::Lsp, "c.nom", 3));
        assert_eq!(id0, 0);
        assert_eq!(id1, 1);
        assert_eq!(id2, 2);
        assert_eq!(panel.len(), 3);
    }

    #[test]
    fn clear_wipes_all_entries() {
        let mut panel = DiagnosticsPanel::new();
        panel.push(make_entry(DiagSeverity::Error, DiagSource::Compiler, "a.nom", 1));
        panel.push(make_entry(DiagSeverity::Warning, DiagSource::Linter, "b.nom", 2));
        panel.clear();
        assert!(panel.is_empty());
    }

    #[test]
    fn clear_source_removes_only_that_source() {
        let mut panel = DiagnosticsPanel::new();
        panel.push(make_entry(DiagSeverity::Error, DiagSource::Compiler, "a.nom", 1));
        panel.push(make_entry(DiagSeverity::Warning, DiagSource::Linter, "b.nom", 2));
        panel.push(make_entry(DiagSeverity::Error, DiagSource::Compiler, "c.nom", 3));
        panel.clear_source(DiagSource::Compiler);
        assert_eq!(panel.len(), 1);
        assert_eq!(panel.entries[0].source, DiagSource::Linter);
    }

    #[test]
    fn filtered_no_filters_returns_all() {
        let mut panel = DiagnosticsPanel::new();
        panel.push(make_entry(DiagSeverity::Error, DiagSource::Compiler, "a.nom", 1));
        panel.push(make_entry(DiagSeverity::Warning, DiagSource::Linter, "b.nom", 2));
        assert_eq!(panel.filtered().len(), 2);
    }

    #[test]
    fn filtered_by_severity() {
        let mut panel = DiagnosticsPanel::new();
        panel.push(make_entry(DiagSeverity::Error, DiagSource::Compiler, "a.nom", 1));
        panel.push(make_entry(DiagSeverity::Warning, DiagSource::Linter, "b.nom", 2));
        panel.push(make_entry(DiagSeverity::Error, DiagSource::Lsp, "c.nom", 3));
        panel.severity_filter = Some(DiagSeverity::Error);
        let results = panel.filtered();
        assert_eq!(results.len(), 2);
        assert!(results.iter().all(|e| e.severity == DiagSeverity::Error));
    }

    #[test]
    fn filtered_by_source() {
        let mut panel = DiagnosticsPanel::new();
        panel.push(make_entry(DiagSeverity::Error, DiagSource::Compiler, "a.nom", 1));
        panel.push(make_entry(DiagSeverity::Warning, DiagSource::Linter, "b.nom", 2));
        panel.push(make_entry(DiagSeverity::Info, DiagSource::Compiler, "c.nom", 3));
        panel.source_filter = Some(DiagSource::Compiler);
        let results = panel.filtered();
        assert_eq!(results.len(), 2);
        assert!(results.iter().all(|e| e.source == DiagSource::Compiler));
    }

    #[test]
    fn filtered_by_file() {
        let mut panel = DiagnosticsPanel::new();
        panel.push(make_entry(DiagSeverity::Error, DiagSource::Compiler, "a.nom", 1));
        panel.push(make_entry(DiagSeverity::Warning, DiagSource::Linter, "b.nom", 2));
        panel.push(make_entry(DiagSeverity::Info, DiagSource::Lsp, "a.nom", 5));
        panel.file_filter = Some("a.nom".to_string());
        let results = panel.filtered();
        assert_eq!(results.len(), 2);
        assert!(results.iter().all(|e| e.file_path == "a.nom"));
    }

    #[test]
    fn counts_by_severity_aggregates_correctly() {
        let mut panel = DiagnosticsPanel::new();
        panel.push(make_entry(DiagSeverity::Error, DiagSource::Compiler, "a.nom", 1));
        panel.push(make_entry(DiagSeverity::Error, DiagSource::Lsp, "b.nom", 2));
        panel.push(make_entry(DiagSeverity::Warning, DiagSource::Linter, "c.nom", 3));
        let counts = panel.counts_by_severity();
        assert_eq!(counts[&DiagSeverity::Error], 2);
        assert_eq!(counts[&DiagSeverity::Warning], 1);
        assert!(!counts.contains_key(&DiagSeverity::Info));
    }

    #[test]
    fn grouped_by_file_preserves_insertion_order() {
        let mut panel = DiagnosticsPanel::new();
        panel.push(make_entry(DiagSeverity::Error, DiagSource::Compiler, "z.nom", 1));
        panel.push(make_entry(DiagSeverity::Warning, DiagSource::Linter, "a.nom", 2));
        panel.push(make_entry(DiagSeverity::Error, DiagSource::Compiler, "z.nom", 5));
        let groups = panel.grouped_by_file();
        assert_eq!(groups.len(), 2);
        // z.nom was inserted first
        assert_eq!(groups[0].0, "z.nom");
        assert_eq!(groups[0].1.len(), 2);
        assert_eq!(groups[1].0, "a.nom");
        assert_eq!(groups[1].1.len(), 1);
    }

    #[test]
    fn next_after_same_file_picks_closest_line() {
        let mut panel = DiagnosticsPanel::new();
        panel.push(make_entry(DiagSeverity::Error, DiagSource::Compiler, "a.nom", 10));
        panel.push(make_entry(DiagSeverity::Warning, DiagSource::Compiler, "a.nom", 20));
        panel.push(make_entry(DiagSeverity::Info, DiagSource::Compiler, "a.nom", 30));
        let next = panel.next_after("a.nom", 15).unwrap();
        assert_eq!(next.line, 20);
    }

    #[test]
    fn next_after_falls_back_to_other_file() {
        let mut panel = DiagnosticsPanel::new();
        panel.push(make_entry(DiagSeverity::Error, DiagSource::Compiler, "a.nom", 5));
        panel.push(make_entry(DiagSeverity::Warning, DiagSource::Linter, "b.nom", 3));
        // cursor at line 10 of a.nom — nothing after in a.nom, falls back to b.nom
        let next = panel.next_after("a.nom", 10).unwrap();
        assert_eq!(next.file_path, "b.nom");
    }

    #[test]
    fn next_after_returns_none_when_empty() {
        let panel = DiagnosticsPanel::new();
        assert!(panel.next_after("a.nom", 0).is_none());
    }
}
