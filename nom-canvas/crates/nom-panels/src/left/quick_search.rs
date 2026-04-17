#![deny(unsafe_code)]
use crate::dock::{DockPosition, Panel};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SearchResultKind { NomtuEntry, Command, File, RecentDoc }

#[derive(Debug, Clone)]
pub struct SearchResult {
    pub id: String,
    pub label: String,
    pub detail: Option<String>,
    pub kind: SearchResultKind,
    pub score: u32,  // higher = better match
}

pub struct QuickSearchPanel {
    pub query: String,
    pub results: Vec<SearchResult>,
    pub selected: Option<usize>,
    pub is_open: bool,
}

impl QuickSearchPanel {
    pub fn new() -> Self {
        Self { query: String::new(), results: vec![], selected: None, is_open: false }
    }

    pub fn open(&mut self) { self.is_open = true; self.query.clear(); self.results.clear(); self.selected = None; }
    pub fn close(&mut self) { self.is_open = false; }

    pub fn set_query(&mut self, q: impl Into<String>) {
        self.query = q.into();
        self.selected = if self.results.is_empty() { None } else { Some(0) };
    }

    pub fn load_results(&mut self, results: Vec<SearchResult>) {
        self.results = results;
        self.selected = if self.results.is_empty() { None } else { Some(0) };
    }

    pub fn move_selection(&mut self, delta: i32) {
        if self.results.is_empty() { return; }
        let n = self.results.len() as i32;
        let current = self.selected.unwrap_or(0) as i32;
        self.selected = Some(((current + delta).rem_euclid(n)) as usize);
    }

    pub fn selected_result(&self) -> Option<&SearchResult> {
        self.selected.and_then(|i| self.results.get(i))
    }
}

impl Default for QuickSearchPanel { fn default() -> Self { Self::new() } }

impl Panel for QuickSearchPanel {
    fn id(&self) -> &str { "quick-search" }
    fn title(&self) -> &str { "Search" }
    fn default_size(&self) -> f32 { 248.0 }
    fn position(&self) -> DockPosition { DockPosition::Left }
    fn activation_priority(&self) -> u32 { 20 }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn quick_search_open_close() {
        let mut qs = QuickSearchPanel::new();
        assert!(!qs.is_open);
        qs.open();
        assert!(qs.is_open);
        qs.close();
        assert!(!qs.is_open);
    }

    #[test]
    fn quick_search_navigation() {
        let mut qs = QuickSearchPanel::new();
        qs.load_results(vec![
            SearchResult { id: "a".into(), label: "alpha".into(), detail: None, kind: SearchResultKind::Command, score: 10 },
            SearchResult { id: "b".into(), label: "beta".into(), detail: None, kind: SearchResultKind::NomtuEntry, score: 8 },
        ]);
        assert_eq!(qs.selected_result().unwrap().id, "a");
        qs.move_selection(1);
        assert_eq!(qs.selected_result().unwrap().id, "b");
        qs.move_selection(1);  // wraps
        assert_eq!(qs.selected_result().unwrap().id, "a");
    }
}
