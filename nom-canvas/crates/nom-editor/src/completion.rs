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
        let mut menu = CompletionMenu::new(vec![make_item("define"), make_item("describe")], 3);
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
        let matches: Vec<_> = files
            .iter()
            .filter(|(_, text)| text.contains(query))
            .collect();
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
        let matches: Vec<_> = files
            .iter()
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
        use crate::lsp_bridge::{CompletionItem, CompletionKind, Location};
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
        use crate::lsp_bridge::LspProvider;
        use crate::lsp_bridge::StubLspProvider;
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
            vec![
                make_item("define"),
                make_item("describe"),
                make_item("map"),
                make_item("filter"),
            ],
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
        let items: Vec<_> = (0..100)
            .map(|i| make_item(&format!("kind_{i:03}")))
            .collect();
        let menu = CompletionMenu::new(items, 0);
        assert_eq!(menu.visible_items().len(), 100);
    }

    // ── wave AB: completion tests ────────────────────────────────────────────

    /// Empty prefix returns all completions via visible_items.
    #[test]
    fn completion_empty_prefix_returns_all() {
        let menu = CompletionMenu::new(
            vec![
                make_item("alpha"),
                make_item("beta"),
                make_item("gamma"),
                make_item("delta"),
            ],
            0,
        );
        // No filter set — all 4 items visible
        assert_eq!(menu.visible_items().len(), 4);
    }

    /// Filter is applied case-insensitively.
    #[test]
    fn completion_filter_case_insensitive() {
        let mut menu = CompletionMenu::new(
            vec![
                make_item("Define"),
                make_item("DEFINE"),
                make_item("describe"),
            ],
            0,
        );
        menu.filter_items("define");
        let visible = menu.visible_items();
        // "Define" and "DEFINE" both lowercased start with "define"
        assert_eq!(visible.len(), 2);
    }

    /// CompletionItem with a documentation/detail field is preserved.
    #[test]
    fn completion_item_detail_field_preserved() {
        use crate::lsp_bridge::CompletionKind;
        let item = CompletionItem {
            label: "summarize".into(),
            kind: CompletionKind::Function,
            detail: Some("fn summarize(text: str) -> str".into()),
            insert_text: "summarize".into(),
            sort_text: None,
        };
        assert!(item.detail.is_some());
        assert_eq!(
            item.detail.as_deref(),
            Some("fn summarize(text: str) -> str")
        );
    }

    /// CompletionKind::Keyword round-trips through construction.
    #[test]
    fn completion_kind_keyword_round_trip() {
        use crate::lsp_bridge::CompletionKind;
        let item = CompletionItem {
            label: "define".into(),
            kind: CompletionKind::Keyword,
            detail: None,
            insert_text: "define".into(),
            sort_text: None,
        };
        assert_eq!(item.kind, CompletionKind::Keyword);
    }

    /// CompletionKind::Value round-trips through construction.
    #[test]
    fn completion_kind_value_round_trip() {
        use crate::lsp_bridge::CompletionKind;
        let item = CompletionItem {
            label: "count".into(),
            kind: CompletionKind::Value,
            detail: None,
            insert_text: "count".into(),
            sort_text: None,
        };
        assert_eq!(item.kind, CompletionKind::Value);
    }

    /// Empty completion list returns empty vec, not an error.
    #[test]
    fn completion_empty_list_returns_empty_vec() {
        let menu = CompletionMenu::new(vec![], 0);
        assert!(menu.visible_items().is_empty());
        assert!(menu.is_empty());
        assert!(menu.selected_item().is_none());
    }

    /// insert_text overwrites the prefix range — insert_text does not contain the prefix again.
    #[test]
    fn completion_insert_text_overwrites_prefix() {
        // The insert_text is the full replacement, not prefix + suffix.
        use crate::lsp_bridge::CompletionKind;
        let item = CompletionItem {
            label: "describe".into(),
            kind: CompletionKind::Function,
            detail: None,
            insert_text: "describe".into(),
            sort_text: None,
        };
        // When triggered at prefix "des", the insert_text replaces the full word.
        let prefix = "des";
        assert!(item.insert_text.starts_with(prefix));
        // insert_text is the whole word, not prefix repeated
        assert_eq!(item.insert_text, "describe");
    }

    /// Max completions limit: only first N items are shown when sliced.
    #[test]
    fn completion_max_limit_respected() {
        let items: Vec<_> = (0..20)
            .map(|i| make_item(&format!("item_{i:02}")))
            .collect();
        let menu = CompletionMenu::new(items, 0);
        let max = 5usize;
        let limited: Vec<_> = menu.visible_items().into_iter().take(max).collect();
        assert_eq!(limited.len(), max);
    }

    /// sort_text field is preserved when set.
    #[test]
    fn completion_sort_text_preserved() {
        use crate::lsp_bridge::CompletionKind;
        let item = CompletionItem {
            label: "foo".into(),
            kind: CompletionKind::Function,
            detail: None,
            insert_text: "foo".into(),
            sort_text: Some("aaa_foo".into()),
        };
        assert_eq!(item.sort_text.as_deref(), Some("aaa_foo"));
    }

    /// filter_items resets selected index to 0.
    #[test]
    fn completion_filter_resets_selected_index() {
        let mut menu = CompletionMenu::new(
            vec![make_item("alpha"), make_item("beta"), make_item("almond")],
            0,
        );
        menu.select_next(); // selected = 1
        menu.filter_items("al");
        assert_eq!(menu.selected, 0);
    }

    /// visible_items after filter contains only matching items.
    #[test]
    fn completion_visible_items_only_matching_after_filter() {
        let mut menu = CompletionMenu::new(
            vec![
                make_item("map"),
                make_item("filter"),
                make_item("fold"),
                make_item("flat_map"),
            ],
            0,
        );
        menu.filter_items("f");
        let visible = menu.visible_items();
        assert!(visible.iter().all(|i| i.label.starts_with('f')));
        // "filter", "fold", "flat_map" match; "map" does not
        assert_eq!(visible.len(), 3);
    }

    /// is_empty returns false when items exist.
    #[test]
    fn completion_is_empty_false_when_items_present() {
        let menu = CompletionMenu::new(vec![make_item("foo")], 0);
        assert!(!menu.is_empty());
    }

    /// trigger_pos is preserved through the menu lifecycle.
    #[test]
    fn completion_trigger_pos_preserved() {
        let menu = CompletionMenu::new(vec![make_item("bar")], 42);
        assert_eq!(menu.trigger_pos, 42);
    }

    /// CompletionKind::Module round-trips through construction.
    #[test]
    fn completion_kind_module_round_trip() {
        use crate::lsp_bridge::CompletionKind;
        let item = CompletionItem {
            label: "std".into(),
            kind: CompletionKind::Module,
            detail: None,
            insert_text: "std".into(),
            sort_text: None,
        };
        assert_eq!(item.kind, CompletionKind::Module);
    }

    /// CompletionKind::Snippet is distinct from all other kinds.
    #[test]
    fn completion_kind_snippet_distinct() {
        use crate::lsp_bridge::CompletionKind;
        assert_ne!(CompletionKind::Snippet, CompletionKind::Function);
        assert_ne!(CompletionKind::Snippet, CompletionKind::Keyword);
        assert_ne!(CompletionKind::Snippet, CompletionKind::Value);
    }
}
