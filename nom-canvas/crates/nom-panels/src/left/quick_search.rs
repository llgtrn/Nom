#![deny(unsafe_code)]
use crate::dock::{fill_quad, DockPosition, Panel};
use nom_gpui::scene::Scene;
use nom_theme::tokens;

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

impl QuickSearchPanel {
    /// Paint the search box + result rows into the GPU scene.
    pub fn paint_scene(&self, width: f32, _height: f32, scene: &mut Scene) {
        // Input field background (32 px tall).
        scene.push_quad(fill_quad(0.0, 0.0, width, 32.0, tokens::BG2));

        // Result row backgrounds (18 px each).
        for (i, _result) in self.results.iter().enumerate() {
            let y = 32.0 + i as f32 * 18.0;
            let row_bg = if self.selected == Some(i) { tokens::FOCUS } else { tokens::BG };
            scene.push_quad(fill_quad(0.0, y, width, 18.0, row_bg));
        }
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
    fn quick_search_paint_has_input_and_rows() {
        let mut qs = QuickSearchPanel::new();
        qs.open();
        qs.set_query("main");
        qs.load_results(vec![
            SearchResult {
                id: "a".into(),
                label: "main.nom".into(),
                detail: None,
                kind: SearchResultKind::File,
                score: 10,
            },
            SearchResult {
                id: "b".into(),
                label: "main_loop".into(),
                detail: None,
                kind: SearchResultKind::Command,
                score: 8,
            },
        ]);

        let mut scene = Scene::new();
        qs.paint_scene(248.0, 400.0, &mut scene);

        // input quad + 2 result-row quads.
        assert_eq!(scene.quads.len(), 3);
        let input = &scene.quads[0];
        assert_eq!(input.bounds.size.height, nom_gpui::types::Pixels(32.0));
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
