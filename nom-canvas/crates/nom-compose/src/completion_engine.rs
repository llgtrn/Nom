/// AI/compiler-driven completion suggestions fed into the editor.

#[derive(Debug, Clone, PartialEq)]
pub enum CompletionKind {
    Keyword,
    Variable,
    Function,
    Type,
    Snippet,
}

impl CompletionKind {
    /// Lower value = higher priority in completion lists.
    pub fn sort_weight(&self) -> u8 {
        match self {
            CompletionKind::Snippet => 0,
            CompletionKind::Keyword => 1,
            CompletionKind::Type => 2,
            CompletionKind::Function => 3,
            CompletionKind::Variable => 4,
        }
    }

    pub fn is_insertable(&self) -> bool {
        true
    }
}

pub struct CompletionItem {
    pub label: String,
    pub kind: CompletionKind,
    pub insert_text: String,
    pub detail: Option<String>,
    pub score: f32,
}

impl CompletionItem {
    pub fn has_detail(&self) -> bool {
        self.detail.is_some()
    }

    pub fn display_label(&self) -> String {
        match &self.detail {
            Some(d) => format!("{} \u{2014} {}", self.label, d),
            None => self.label.clone(),
        }
    }
}

pub struct CompletionList {
    pub items: Vec<CompletionItem>,
    pub is_complete: bool,
}

impl CompletionList {
    pub fn new(is_complete: bool) -> Self {
        Self {
            items: Vec::new(),
            is_complete,
        }
    }

    pub fn add(&mut self, item: CompletionItem) {
        self.items.push(item);
    }

    /// Return top `n` items sorted by score descending.
    pub fn top_n(&self, n: usize) -> Vec<&CompletionItem> {
        let mut refs: Vec<&CompletionItem> = self.items.iter().collect();
        refs.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));
        refs.into_iter().take(n).collect()
    }

    /// Return all items matching the given kind.
    pub fn filter_kind(&self, k: CompletionKind) -> Vec<&CompletionItem> {
        self.items.iter().filter(|i| i.kind == k).collect()
    }
}

pub struct CompletionQuery {
    pub prefix: String,
    pub cursor_byte: usize,
    pub max_results: usize,
}

impl CompletionQuery {
    /// Returns true if `label` starts with `self.prefix` (case-insensitive).
    pub fn matches_label(&self, label: &str) -> bool {
        label
            .to_lowercase()
            .starts_with(&self.prefix.to_lowercase())
    }
}

pub struct CompletionEngine {
    pub items: Vec<CompletionItem>,
}

impl CompletionEngine {
    pub fn new() -> Self {
        Self { items: Vec::new() }
    }

    pub fn register(&mut self, item: CompletionItem) {
        self.items.push(item);
    }

    /// Filter by prefix match, sort by score desc, limit to max_results.
    pub fn complete(&self, query: &CompletionQuery) -> CompletionList {
        let mut matched: Vec<&CompletionItem> = self
            .items
            .iter()
            .filter(|i| query.matches_label(&i.label))
            .collect();
        matched.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));

        let mut list = CompletionList::new(true);
        for item in matched.into_iter().take(query.max_results) {
            list.add(CompletionItem {
                label: item.label.clone(),
                kind: item.kind.clone(),
                insert_text: item.insert_text.clone(),
                detail: item.detail.clone(),
                score: item.score,
            });
        }
        list
    }
}

impl Default for CompletionEngine {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod completion_engine_tests {
    use super::*;

    // Test 1: kind sort_weight snippet=0
    #[test]
    fn kind_sort_weight_snippet_is_zero() {
        assert_eq!(CompletionKind::Snippet.sort_weight(), 0);
        assert_eq!(CompletionKind::Keyword.sort_weight(), 1);
        assert_eq!(CompletionKind::Type.sort_weight(), 2);
        assert_eq!(CompletionKind::Function.sort_weight(), 3);
        assert_eq!(CompletionKind::Variable.sort_weight(), 4);
    }

    // Test 2: item has_detail true
    #[test]
    fn item_has_detail_true() {
        let item = CompletionItem {
            label: "foo".to_string(),
            kind: CompletionKind::Function,
            insert_text: "foo()".to_string(),
            detail: Some("Returns a foo".to_string()),
            score: 1.0,
        };
        assert!(item.has_detail());
    }

    // Test 3: item has_detail false
    #[test]
    fn item_has_detail_false() {
        let item = CompletionItem {
            label: "bar".to_string(),
            kind: CompletionKind::Variable,
            insert_text: "bar".to_string(),
            detail: None,
            score: 0.5,
        };
        assert!(!item.has_detail());
    }

    // Test 4: item display_label with detail
    #[test]
    fn item_display_label_with_detail() {
        let item = CompletionItem {
            label: "open".to_string(),
            kind: CompletionKind::Keyword,
            insert_text: "open".to_string(),
            detail: Some("open a file".to_string()),
            score: 0.8,
        };
        assert_eq!(item.display_label(), "open \u{2014} open a file");
    }

    // Test 5: item display_label without detail
    #[test]
    fn item_display_label_without_detail() {
        let item = CompletionItem {
            label: "close".to_string(),
            kind: CompletionKind::Keyword,
            insert_text: "close".to_string(),
            detail: None,
            score: 0.7,
        };
        assert_eq!(item.display_label(), "close");
    }

    // Test 6: list top_n sorted by score
    #[test]
    fn list_top_n_sorted_by_score() {
        let mut list = CompletionList::new(true);
        list.add(CompletionItem {
            label: "a".to_string(),
            kind: CompletionKind::Variable,
            insert_text: "a".to_string(),
            detail: None,
            score: 0.3,
        });
        list.add(CompletionItem {
            label: "b".to_string(),
            kind: CompletionKind::Variable,
            insert_text: "b".to_string(),
            detail: None,
            score: 0.9,
        });
        list.add(CompletionItem {
            label: "c".to_string(),
            kind: CompletionKind::Variable,
            insert_text: "c".to_string(),
            detail: None,
            score: 0.6,
        });
        let top2 = list.top_n(2);
        assert_eq!(top2.len(), 2);
        assert_eq!(top2[0].label, "b");
        assert_eq!(top2[1].label, "c");
    }

    // Test 7: list filter_kind
    #[test]
    fn list_filter_kind() {
        let mut list = CompletionList::new(true);
        list.add(CompletionItem {
            label: "fn1".to_string(),
            kind: CompletionKind::Function,
            insert_text: "fn1()".to_string(),
            detail: None,
            score: 1.0,
        });
        list.add(CompletionItem {
            label: "kw1".to_string(),
            kind: CompletionKind::Keyword,
            insert_text: "kw1".to_string(),
            detail: None,
            score: 0.8,
        });
        list.add(CompletionItem {
            label: "fn2".to_string(),
            kind: CompletionKind::Function,
            insert_text: "fn2()".to_string(),
            detail: None,
            score: 0.9,
        });
        let fns = list.filter_kind(CompletionKind::Function);
        assert_eq!(fns.len(), 2);
        let kws = list.filter_kind(CompletionKind::Keyword);
        assert_eq!(kws.len(), 1);
        assert_eq!(kws[0].label, "kw1");
    }

    // Test 8: query matches_label case-insensitive
    #[test]
    fn query_matches_label_case_insensitive() {
        let query = CompletionQuery {
            prefix: "FoO".to_string(),
            cursor_byte: 3,
            max_results: 10,
        };
        assert!(query.matches_label("fooBar"));
        assert!(query.matches_label("FOObar"));
        assert!(!query.matches_label("barFoo"));
    }

    // Test 9a: engine complete filters by prefix
    #[test]
    fn engine_complete_filters_by_prefix() {
        let mut engine = CompletionEngine::new();
        engine.register(CompletionItem {
            label: "println".to_string(),
            kind: CompletionKind::Function,
            insert_text: "println!()".to_string(),
            detail: None,
            score: 0.9,
        });
        engine.register(CompletionItem {
            label: "eprintln".to_string(),
            kind: CompletionKind::Function,
            insert_text: "eprintln!()".to_string(),
            detail: None,
            score: 0.8,
        });
        engine.register(CompletionItem {
            label: "format".to_string(),
            kind: CompletionKind::Function,
            insert_text: "format!()".to_string(),
            detail: None,
            score: 0.7,
        });
        let query = CompletionQuery {
            prefix: "print".to_string(),
            cursor_byte: 5,
            max_results: 10,
        };
        let list = engine.complete(&query);
        assert_eq!(list.items.len(), 1);
        assert_eq!(list.items[0].label, "println");
    }

    // Test 9b: engine complete limits to max_results
    #[test]
    fn engine_complete_limits_to_max_results() {
        let mut engine = CompletionEngine::new();
        for i in 0..5 {
            engine.register(CompletionItem {
                label: format!("item{}", i),
                kind: CompletionKind::Variable,
                insert_text: format!("item{}", i),
                detail: None,
                score: i as f32 * 0.1,
            });
        }
        let query = CompletionQuery {
            prefix: "item".to_string(),
            cursor_byte: 4,
            max_results: 3,
        };
        let list = engine.complete(&query);
        assert_eq!(list.items.len(), 3);
    }

    // Test 9c: engine complete is_complete=true
    #[test]
    fn engine_complete_is_complete_true() {
        let engine = CompletionEngine::new();
        let query = CompletionQuery {
            prefix: "any".to_string(),
            cursor_byte: 3,
            max_results: 10,
        };
        let list = engine.complete(&query);
        assert!(list.is_complete);
    }
}
