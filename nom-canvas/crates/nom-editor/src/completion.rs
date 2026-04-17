//! Completion candidates from nom-resolver (entities by prefix) and
//! nom-grammar (keywords / patterns). MVP stub; real wiring lands with
//! compiler integration.

#![deny(unsafe_code)]

/// A single completion candidate shown in the popup list.
#[derive(Clone, Debug)]
pub struct CompletionItem {
    /// Text shown in the list.
    pub label: String,
    /// Optional detail line (e.g. type signature).
    pub detail: Option<String>,
    pub kind: CompletionKind,
    /// Text actually inserted when the item is accepted.
    pub insert_text: String,
    /// Lower value = shown first. Ties broken by `label` lexicographic order.
    pub sort_priority: i32,
}

/// Origin of a completion candidate; callers may use this for icon rendering.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CompletionKind {
    /// Reserved word from the fixed keyword set.
    Keyword,
    /// Entity (function, kind, field) from nom-resolver.
    Entity,
    /// Structural pattern from nom-grammar.
    Pattern,
    /// Multi-token expansion snippet.
    Snippet,
}

/// Sort `items` in-place: ascending by `sort_priority`, then by `label`.
pub fn rank(items: &mut Vec<CompletionItem>) {
    items.sort_by(|a, b| {
        a.sort_priority
            .cmp(&b.sort_priority)
            .then_with(|| a.label.cmp(&b.label))
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    fn item(label: &str, priority: i32) -> CompletionItem {
        CompletionItem {
            label: label.into(),
            detail: None,
            kind: CompletionKind::Keyword,
            insert_text: label.into(),
            sort_priority: priority,
        }
    }

    #[test]
    fn rank_sorts_by_priority_then_label() {
        let mut items = vec![item("z", 1), item("a", 2), item("b", 1)];
        rank(&mut items);
        assert_eq!(items[0].label, "b"); // priority 1, "b" < "z"
        assert_eq!(items[1].label, "z"); // priority 1, "z" > "b"
        assert_eq!(items[2].label, "a"); // priority 2 (last)
    }

    #[test]
    fn rank_stable_on_equal_priority_and_label() {
        let mut items = vec![item("x", 0), item("x", 0)];
        rank(&mut items);
        assert_eq!(items.len(), 2);
    }

    #[test]
    fn rank_single_item() {
        let mut items = vec![item("only", 5)];
        rank(&mut items);
        assert_eq!(items[0].label, "only");
    }

    #[test]
    fn rank_empty_no_panic() {
        let mut items: Vec<CompletionItem> = vec![];
        rank(&mut items);
        assert!(items.is_empty());
    }

    #[test]
    fn completion_kinds_are_distinct() {
        assert_ne!(CompletionKind::Keyword, CompletionKind::Entity);
        assert_ne!(CompletionKind::Pattern, CompletionKind::Snippet);
    }
}
