#![deny(unsafe_code)]
use crate::dock::{DockPosition, Panel};
use crate::right::chat_sidebar::RenderPrimitive;

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
    /// Render the search box and result rows into primitives.
    ///
    /// Layout: input rect (h=32) at top; each result row is 18px tall below it.
    /// Matched text receives a highlight rect (color 0x45475a) behind it.
    pub fn render_bounds(&self, width: f32, _height: f32) -> Vec<RenderPrimitive> {
        let mut out = Vec::new();

        // Input field background.
        out.push(RenderPrimitive::Rect {
            x: 0.0,
            y: 0.0,
            w: width,
            h: 32.0,
            color: 0x313244,
        });

        // Query text inside the input box.
        if !self.query.is_empty() {
            out.push(RenderPrimitive::Text {
                x: 8.0,
                y: 8.0,
                text: self.query.clone(),
                size: 13.0,
                color: 0xcdd6f4,
            });
        }

        // Result rows.
        for (i, result) in self.results.iter().enumerate() {
            let y = 32.0 + i as f32 * 18.0;

            // Highlight rect for the matched portion.
            out.push(RenderPrimitive::Rect {
                x: 0.0,
                y,
                w: width,
                h: 18.0,
                color: 0x45475a,
            });

            out.push(RenderPrimitive::Text {
                x: 8.0,
                y: y + 2.0,
                text: result.label.clone(),
                size: 13.0,
                color: 0xcdd6f4,
            });
        }

        out
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
    fn quick_search_render_has_input_rect() {
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

        let prims = qs.render_bounds(248.0, 400.0);

        // First primitive must be the input rect (h=32, color 0x313244).
        match &prims[0] {
            RenderPrimitive::Rect { y, h, color, .. } => {
                assert_eq!(*y, 0.0);
                assert_eq!(*h, 32.0);
                assert_eq!(*color, 0x313244);
            }
            other => panic!("expected input Rect, got {:?}", other),
        }

        // Must have highlight rects for both results (color 0x45475a).
        let highlight_count = prims
            .iter()
            .filter(|p| matches!(p, RenderPrimitive::Rect { color: 0x45475a, .. }))
            .count();
        assert_eq!(highlight_count, 2, "expected 2 result highlight rects");

        // Must have text primitives for query + 2 results.
        let text_count = prims.iter().filter(|p| matches!(p, RenderPrimitive::Text { .. })).count();
        assert!(text_count >= 3, "expected >=3 text primitives, got {}", text_count);
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
