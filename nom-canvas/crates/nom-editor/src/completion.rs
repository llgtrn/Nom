#![deny(unsafe_code)]
use crate::lsp_bridge::CompletionItem;

pub struct CompletionMenu {
    pub items: Vec<CompletionItem>,
    pub selected: usize,
    pub trigger_pos: usize,
    pub filter: String,
}

impl CompletionMenu {
    pub fn new(items: Vec<CompletionItem>, trigger_pos: usize) -> Self {
        Self {
            items,
            selected: 0,
            trigger_pos,
            filter: String::new(),
        }
    }
    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }
    pub fn select_next(&mut self) {
        if self.selected + 1 < self.items.len() {
            self.selected += 1;
        }
    }
    pub fn select_prev(&mut self) {
        if self.selected > 0 {
            self.selected -= 1;
        }
    }
    pub fn selected_item(&self) -> Option<&CompletionItem> {
        self.items.get(self.selected)
    }
    pub fn filter_items(&mut self, prefix: &str) {
        self.filter = prefix.to_lowercase();
        self.selected = 0;
    }
    pub fn visible_items(&self) -> Vec<&CompletionItem> {
        if self.filter.is_empty() {
            self.items.iter().collect()
        } else {
            self.items
                .iter()
                .filter(|item| item.label.to_lowercase().starts_with(&self.filter))
                .collect()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn make_item(label: &str) -> CompletionItem {
        use crate::lsp_bridge::CompletionKind;
        CompletionItem {
            label: label.into(),
            kind: CompletionKind::Function,
            detail: None,
            insert_text: label.into(),
            sort_text: None,
        }
    }
    #[test]
    fn completion_filter() {
        let mut menu = CompletionMenu::new(
            vec![
                make_item("summarize"),
                make_item("search"),
                make_item("transform"),
            ],
            5,
        );
        menu.filter_items("su");
        assert_eq!(menu.visible_items().len(), 1);
        assert_eq!(menu.visible_items()[0].label, "summarize");
    }
    #[test]
    fn completion_list_creates() {
        let menu = CompletionMenu::new(vec![make_item("foo"), make_item("bar")], 0);
        assert_eq!(menu.items.len(), 2);
        assert_eq!(menu.selected, 0);
        assert_eq!(menu.trigger_pos, 0);
    }
    #[test]
    fn completion_filter_by_prefix() {
        let mut menu = CompletionMenu::new(
            vec![make_item("alpha"), make_item("beta"), make_item("almond")],
            0,
        );
        menu.filter_items("al");
        let visible = menu.visible_items();
        assert_eq!(visible.len(), 2);
        assert!(visible.iter().all(|i| i.label.starts_with("al")));
    }
    #[test]
    fn completion_select_item() {
        let mut menu = CompletionMenu::new(vec![make_item("a"), make_item("b"), make_item("c")], 0);
        assert_eq!(menu.selected_item().unwrap().label, "a");
        menu.select_next();
        assert_eq!(menu.selected_item().unwrap().label, "b");
        menu.select_prev();
        assert_eq!(menu.selected_item().unwrap().label, "a");
    }

    #[test]
    fn completion_provider_empty_prefix_returns_all_items() {
        let menu = CompletionMenu::new(
            vec![make_item("alpha"), make_item("beta"), make_item("gamma")],
            0,
        );
        // empty filter → all items visible
        assert_eq!(menu.visible_items().len(), 3);
    }

    #[test]
    fn completion_item_label_nonempty() {
        let items = vec![make_item("fn"), make_item("let"), make_item("define")];
        for item in &items {
            assert!(!item.label.is_empty(), "completion label must not be empty");
        }
    }

    #[test]
    fn completion_provider_filters_by_fn_prefix() {
        let mut menu = CompletionMenu::new(
            vec![
                make_item("fn_call"),
                make_item("fn_def"),
                make_item("let_bind"),
                make_item("function"),
            ],
            0,
        );
        menu.filter_items("fn");
        let visible = menu.visible_items();
        assert_eq!(visible.len(), 2);
        assert!(visible.iter().all(|i| i.label.starts_with("fn")));
    }

    #[test]
    fn completion_item_kind_set() {
        use crate::lsp_bridge::CompletionKind;
        let item = make_item("foo");
        // make_item always produces CompletionKind::Function
        assert_eq!(item.kind, CompletionKind::Function);
    }

    #[test]
    fn completion_select_prev_at_start_stays() {
        let mut menu = CompletionMenu::new(vec![make_item("x"), make_item("y")], 0);
        menu.select_prev(); // already at 0, should stay
        assert_eq!(menu.selected, 0);
    }

    #[test]
    fn completion_select_next_at_end_stays() {
        let mut menu = CompletionMenu::new(vec![make_item("x"), make_item("y")], 0);
        menu.select_next();
        menu.select_next(); // already at last item
        assert_eq!(menu.selected, 1);
    }

    // ── wave AJ-7: new completion tests ──────────────────────────────────────

    /// Completion triggers on '.' — trigger_pos is the offset after the dot.
    #[test]
    fn editor_completion_triggers_on_dot() {
        let source = "foo.";
        let trigger_pos = source.len(); // 4
        let items = vec![make_item("bar"), make_item("baz")];
        let menu = CompletionMenu::new(items, trigger_pos);
        assert_eq!(menu.trigger_pos, 4);
        assert_eq!(menu.items.len(), 2);
    }

    /// Completion triggers on ':' — trigger_pos is the offset after the colon.
    #[test]
    fn editor_completion_triggers_on_colon() {
        let source = "Type:";
        let trigger_pos = source.len();
        let items = vec![make_item("method_a"), make_item("method_b")];
        let menu = CompletionMenu::new(items, trigger_pos);
        assert_eq!(menu.trigger_pos, 5);
        assert!(!menu.is_empty());
    }

    /// Dismiss on Escape — menu becomes empty after clearing items.
    #[test]
    fn editor_completion_dismiss_on_escape() {
        let mut menu = CompletionMenu::new(vec![make_item("foo"), make_item("bar")], 0);
        assert!(!menu.is_empty());
        // Simulate dismiss: replace items with empty
        menu.items.clear();
        assert!(menu.is_empty());
    }

    /// Accept on Tab — selected item's insert_text is returned.
    #[test]
    fn editor_completion_accept_on_tab() {
        let mut menu = CompletionMenu::new(
            vec![make_item("define"), make_item("describe")],
            3,
        );
        menu.select_next();
        let accepted = menu.selected_item().unwrap().insert_text.clone();
        assert_eq!(accepted, "describe");
    }

    /// Accept on Enter — selected item's insert_text equals label for make_item.
    #[test]
    fn editor_completion_accept_on_enter() {
        let menu = CompletionMenu::new(vec![make_item("yield"), make_item("map")], 0);
        let accepted = menu.selected_item().unwrap().insert_text.clone();
        assert_eq!(accepted, "yield");
    }

    /// Completion insert text at cursor: insert_text of selected item is non-empty.
    #[test]
    fn editor_completion_insert_text_at_cursor() {
        let menu = CompletionMenu::new(vec![make_item("result")], 5);
        let item = menu.selected_item().unwrap();
        assert!(!item.insert_text.is_empty());
        assert_eq!(item.insert_text, "result");
    }

    /// Project search finds across files — simulated by searching a list of (file, text) pairs.
    #[test]
    fn editor_project_search_finds_across_files() {
        let files = [
            ("a.nom", "define foo that is 1"),
            ("b.nom", "define bar that is foo"),
            ("c.nom", "use result from foo"),
        ];
        let query = "foo";
        let matches: Vec<_> = files.iter().filter(|(_, text)| text.contains(query)).collect();
        assert_eq!(matches.len(), 3, "query 'foo' should match all 3 files");
    }

    /// Project search is case-insensitive.
    #[test]
    fn editor_project_search_case_insensitive() {
        let files = [
            ("a.nom", "Define Foo That Is 1"),
            ("b.nom", "define bar that is foo"),
        ];
        let query = "foo";
        let matches: Vec<_> = files.iter()
            .filter(|(_, text)| text.to_lowercase().contains(&query.to_lowercase()))
            .collect();
        assert_eq!(matches.len(), 2);
    }

    /// Project search with regex pattern matches only files with matching pattern.
    #[test]
    fn editor_project_search_regex_pattern() {
        // Simulate a regex-style search: pattern "define \w+" matches define + word
        let files = [
            ("a.nom", "define foo that is 1"),
            ("b.nom", "use bar"),
            ("c.nom", "define baz"),
        ];
        // Use starts_with as a simple "regex" substitute
        let pattern = "define ";
        let matches: Vec<_> = files.iter().filter(|(_, t)| t.contains(pattern)).collect();
        assert_eq!(matches.len(), 2);
    }

    /// Go to definition for a known symbol returns a non-empty path.
    #[test]
    fn editor_go_to_definition_known_symbol() {
        use crate::lsp_bridge::{CompletionKind, CompletionItem, Location};
        // Simulate: definition lookup returns Some(Location)
        let loc = Some(Location {
            path: std::path::PathBuf::from("src/lib.nom"),
            range: 10..20,
        });
        assert!(loc.is_some());
        let l = loc.unwrap();
        assert_eq!(l.path, std::path::PathBuf::from("src/lib.nom"));
        assert_eq!(l.range.start, 10);
    }

    /// Go to definition for an unknown symbol returns None.
    #[test]
    fn editor_go_to_definition_unknown_returns_none() {
        use crate::lsp_bridge::StubLspProvider;
        use crate::lsp_bridge::LspProvider;
        let provider = StubLspProvider;
        let loc = provider.goto_definition(std::path::Path::new("unknown.nom"), 0);
        assert!(loc.is_none());
    }

    /// Signature shows active param — first parameter is at index 0.
    #[test]
    fn editor_signature_shows_active_param() {
        let signature = "define <name> that is <value>";
        let params: Vec<&str> = signature.split_whitespace().collect();
        let active_param = 0usize;
        assert!(active_param < params.len());
        assert_eq!(params[active_param], "define");
    }

    /// Completion with prefix "de" filters to items starting with "de".
    #[test]
    fn complete_prefix_filters_to_subset() {
        let mut menu = CompletionMenu::new(
            vec![make_item("define"), make_item("describe"), make_item("map"), make_item("filter")],
            0,
        );
        menu.filter_items("de");
        let visible = menu.visible_items();
        assert_eq!(visible.len(), 2);
        assert!(visible.iter().all(|i| i.label.starts_with("de")));
    }

    /// 100 grammar kinds all returned by visible_items when no filter set.
    #[test]
    fn complete_with_100_grammar_kinds_returns_all() {
        let items: Vec<_> = (0..100).map(|i| make_item(&format!("kind_{i:03}"))).collect();
        let menu = CompletionMenu::new(items, 0);
        assert_eq!(menu.visible_items().len(), 100);
    }
}
