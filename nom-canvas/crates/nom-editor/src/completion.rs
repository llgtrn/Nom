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
}
