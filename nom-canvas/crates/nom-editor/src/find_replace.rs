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
}
impl Default for FindState { fn default() -> Self { Self::new() } }
