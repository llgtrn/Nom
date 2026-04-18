#[derive(Debug, Clone, PartialEq)]
pub enum SearchTokenKind {
    Word,
    Symbol,
    Number,
    Operator,
}

impl SearchTokenKind {
    pub fn is_indexable(&self) -> bool {
        matches!(self, SearchTokenKind::Word | SearchTokenKind::Symbol)
    }

    pub fn token_code(&self) -> u8 {
        match self {
            SearchTokenKind::Word => 0,
            SearchTokenKind::Symbol => 1,
            SearchTokenKind::Number => 2,
            SearchTokenKind::Operator => 3,
        }
    }
}

#[derive(Debug, Clone)]
pub struct SearchToken {
    pub kind: SearchTokenKind,
    pub text: String,
    pub byte_offset: usize,
}

impl SearchToken {
    pub fn matches_query(&self, q: &str) -> bool {
        self.text.to_lowercase().contains(&q.to_lowercase())
    }

    pub fn token_key(&self) -> String {
        format!("{}@{}", self.text, self.byte_offset)
    }
}

pub struct SearchIndex {
    pub tokens: Vec<SearchToken>,
}

impl SearchIndex {
    pub fn new() -> Self {
        Self { tokens: Vec::new() }
    }

    pub fn add(&mut self, t: SearchToken) {
        self.tokens.push(t);
    }

    pub fn search(&self, query: &str) -> Vec<&SearchToken> {
        self.tokens
            .iter()
            .filter(|t| t.kind.is_indexable() && t.matches_query(query))
            .collect()
    }

    pub fn token_count(&self) -> usize {
        self.tokens.len()
    }

    pub fn clear(&mut self) {
        self.tokens.clear();
    }
}

pub struct SearchQuery {
    pub text: String,
    pub case_sensitive: bool,
    pub max_results: Option<usize>,
}

impl SearchQuery {
    pub fn effective_text(&self) -> String {
        if !self.case_sensitive {
            self.text.to_lowercase()
        } else {
            self.text.clone()
        }
    }

    pub fn is_empty(&self) -> bool {
        self.text.is_empty()
    }
}

pub struct SearchResult {
    pub token: SearchToken,
    pub rank: f32,
}

impl SearchResult {
    pub fn is_relevant(&self, threshold: f32) -> bool {
        self.rank >= threshold
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn token_kind_is_indexable_operator_false() {
        assert!(!SearchTokenKind::Operator.is_indexable());
        assert!(!SearchTokenKind::Number.is_indexable());
        assert!(SearchTokenKind::Word.is_indexable());
        assert!(SearchTokenKind::Symbol.is_indexable());
    }

    #[test]
    fn token_kind_token_code() {
        assert_eq!(SearchTokenKind::Word.token_code(), 0);
        assert_eq!(SearchTokenKind::Symbol.token_code(), 1);
        assert_eq!(SearchTokenKind::Number.token_code(), 2);
        assert_eq!(SearchTokenKind::Operator.token_code(), 3);
    }

    #[test]
    fn token_matches_query_case_insensitive() {
        let t = SearchToken {
            kind: SearchTokenKind::Word,
            text: "Hello".to_string(),
            byte_offset: 0,
        };
        assert!(t.matches_query("hello"));
        assert!(t.matches_query("HELLO"));
        assert!(t.matches_query("ell"));
        assert!(!t.matches_query("world"));
    }

    #[test]
    fn token_token_key() {
        let t = SearchToken {
            kind: SearchTokenKind::Word,
            text: "foo".to_string(),
            byte_offset: 42,
        };
        assert_eq!(t.token_key(), "foo@42");
    }

    #[test]
    fn index_search_filters_non_indexable() {
        let mut idx = SearchIndex::new();
        idx.add(SearchToken { kind: SearchTokenKind::Word, text: "alpha".to_string(), byte_offset: 0 });
        idx.add(SearchToken { kind: SearchTokenKind::Number, text: "alpha123".to_string(), byte_offset: 5 });
        idx.add(SearchToken { kind: SearchTokenKind::Symbol, text: "alphaSymbol".to_string(), byte_offset: 10 });
        let results = idx.search("alpha");
        assert_eq!(results.len(), 2);
        assert!(results.iter().all(|t| t.kind.is_indexable()));
    }

    #[test]
    fn index_token_count() {
        let mut idx = SearchIndex::new();
        assert_eq!(idx.token_count(), 0);
        idx.add(SearchToken { kind: SearchTokenKind::Word, text: "a".to_string(), byte_offset: 0 });
        idx.add(SearchToken { kind: SearchTokenKind::Word, text: "b".to_string(), byte_offset: 1 });
        assert_eq!(idx.token_count(), 2);
    }

    #[test]
    fn index_clear() {
        let mut idx = SearchIndex::new();
        idx.add(SearchToken { kind: SearchTokenKind::Word, text: "x".to_string(), byte_offset: 0 });
        idx.clear();
        assert_eq!(idx.token_count(), 0);
    }

    #[test]
    fn query_effective_text_lowercase() {
        let q = SearchQuery {
            text: "FooBar".to_string(),
            case_sensitive: false,
            max_results: None,
        };
        assert_eq!(q.effective_text(), "foobar");

        let q2 = SearchQuery {
            text: "FooBar".to_string(),
            case_sensitive: true,
            max_results: None,
        };
        assert_eq!(q2.effective_text(), "FooBar");
    }

    #[test]
    fn result_is_relevant() {
        let r = SearchResult {
            token: SearchToken { kind: SearchTokenKind::Word, text: "t".to_string(), byte_offset: 0 },
            rank: 0.75,
        };
        assert!(r.is_relevant(0.5));
        assert!(r.is_relevant(0.75));
        assert!(!r.is_relevant(0.76));
    }
}
