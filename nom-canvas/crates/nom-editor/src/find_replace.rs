#![deny(unsafe_code)]
use std::ops::Range;

pub struct FindState { pub query: String, pub case_sensitive: bool, pub whole_word: bool, pub use_regex: bool, pub matches: Vec<Range<usize>>, pub current_match: usize }
impl FindState {
    pub fn new() -> Self { Self { query: String::new(), case_sensitive: false, whole_word: false, use_regex: false, matches: Vec::new(), current_match: 0 } }
    pub fn find_in_text(&mut self, text: &str) {
        self.matches.clear();
        if self.query.is_empty() { return; }
        let (search_text, search_query) = if self.case_sensitive {
            (text.to_string(), self.query.clone())
        } else {
            (text.to_lowercase(), self.query.to_lowercase())
        };
        let mut start = 0;
        while let Some(pos) = search_text[start..].find(&search_query) {
            let abs_pos = start + pos;
            self.matches.push(abs_pos..abs_pos + self.query.len());
            start = abs_pos + 1;
        }
    }
    pub fn next_match(&mut self) { if !self.matches.is_empty() { self.current_match = (self.current_match + 1) % self.matches.len(); } }
    pub fn prev_match(&mut self) { if !self.matches.is_empty() { self.current_match = if self.current_match == 0 { self.matches.len() - 1 } else { self.current_match - 1 }; } }
    pub fn current(&self) -> Option<&Range<usize>> { self.matches.get(self.current_match) }
    pub fn replace_current(&self, text: &mut String, replacement: &str) -> bool {
        if let Some(range) = self.current() {
            let range = range.clone();
            text.replace_range(range, replacement);
            true
        } else {
            false
        }
    }
}
impl Default for FindState { fn default() -> Self { Self::new() } }

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn find_replace_case_insensitive_match() {
        let mut state = FindState::new();
        state.query = "hello".to_string();
        state.case_sensitive = false;
        state.find_in_text("Say HELLO world hello");
        assert_eq!(state.matches.len(), 2);
    }

    #[test]
    fn find_replace_no_match_empty_results() {
        let mut state = FindState::new();
        state.query = "xyz".to_string();
        state.find_in_text("hello world");
        assert!(state.matches.is_empty());
    }

    #[test]
    fn find_replace_wraps_next_at_end() {
        let mut state = FindState::new();
        state.query = "a".to_string();
        state.find_in_text("a b a");
        assert_eq!(state.matches.len(), 2);
        state.current_match = 1;
        state.next_match();
        assert_eq!(state.current_match, 0);
    }

    #[test]
    fn find_replace_prev_match() {
        let mut state = FindState::new();
        state.query = "a".to_string();
        state.find_in_text("a b a c a");
        assert_eq!(state.matches.len(), 3);
        state.current_match = 0;
        state.prev_match();
        assert_eq!(state.current_match, 2);
    }

    #[test]
    fn find_replace_replace_current() {
        let mut state = FindState::new();
        state.query = "world".to_string();
        state.find_in_text("hello world");
        let mut text = "hello world".to_string();
        let replaced = state.replace_current(&mut text, "rust");
        assert!(replaced);
        assert_eq!(text, "hello rust");
    }
}
