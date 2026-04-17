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
        Self { items, selected: 0, trigger_pos, filter: String::new() }
    }
    pub fn is_empty(&self) -> bool { self.items.is_empty() }
    pub fn select_next(&mut self) { if self.selected + 1 < self.items.len() { self.selected += 1; } }
    pub fn select_prev(&mut self) { if self.selected > 0 { self.selected -= 1; } }
    pub fn selected_item(&self) -> Option<&CompletionItem> { self.items.get(self.selected) }
    pub fn filter_items(&mut self, prefix: &str) {
        self.filter = prefix.to_lowercase();
        self.selected = 0;
    }
    pub fn visible_items(&self) -> Vec<&CompletionItem> {
        if self.filter.is_empty() {
            self.items.iter().collect()
        } else {
            self.items.iter().filter(|item| item.label.to_lowercase().starts_with(&self.filter)).collect()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn make_item(label: &str) -> CompletionItem {
        use crate::lsp_bridge::CompletionKind;
        CompletionItem { label: label.into(), kind: CompletionKind::Function, detail: None, insert_text: label.into(), sort_text: None }
    }
    #[test]
    fn completion_filter() {
        let mut menu = CompletionMenu::new(vec![make_item("summarize"), make_item("search"), make_item("transform")], 5);
        menu.filter_items("su");
        assert_eq!(menu.visible_items().len(), 1);
        assert_eq!(menu.visible_items()[0].label, "summarize");
    }
}
