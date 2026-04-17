#![deny(unsafe_code)]
use std::ops::Range;
use regex::Regex;

pub struct FindState { pub query: String, pub case_sensitive: bool, pub whole_word: bool, pub use_regex: bool, pub matches: Vec<Range<usize>>, pub current_match: usize }
impl FindState {
    pub fn new() -> Self { Self { query: String::new(), case_sensitive: false, whole_word: false, use_regex: false, matches: Vec::new(), current_match: 0 } }

    /// Perform a real regex search using the `regex` crate.
    ///
    /// Compiles `self.query` as a `Regex` (honouring `case_sensitive`) and
    /// returns the byte-offset start of every match.  Returns an empty Vec
    /// if the query is empty or the pattern fails to compile.
    pub fn find_regex(&self, text: &str) -> Vec<usize> {
        if self.query.is_empty() { return Vec::new(); }
        let pattern = if self.case_sensitive {
            self.query.clone()
        } else {
            format!("(?i){}", self.query)
        };
        let re = match Regex::new(&pattern) {
            Ok(r) => r,
            Err(_) => return Vec::new(),
        };
        re.find_iter(text).map(|m| m.start()).collect()
    }

    fn is_word_boundary(text: &str, pos: usize, len: usize) -> bool {
        let before_ok = if pos == 0 {
            true
        } else {
            let ch = text[..pos].chars().next_back().unwrap_or(' ');
            !ch.is_alphanumeric() && ch != '_'
        };
        let after_pos = pos + len;
        let after_ok = if after_pos >= text.len() {
            true
        } else {
            let ch = text[after_pos..].chars().next().unwrap_or(' ');
            !ch.is_alphanumeric() && ch != '_'
        };
        before_ok && after_ok
    }

    pub fn find_in_text(&mut self, text: &str) {
        self.matches.clear();
        if self.query.is_empty() { return; }

        if self.use_regex {
            if self.query.is_empty() { return; }
            let pattern = if self.case_sensitive {
                self.query.clone()
            } else {
                format!("(?i){}", self.query)
            };
            if let Ok(re) = Regex::new(&pattern) {
                for m in re.find_iter(text) {
                    self.matches.push(m.start()..m.end());
                }
            }
            return;
        }

        let (search_text, search_query) = if self.case_sensitive {
            (text.to_string(), self.query.clone())
        } else {
            (text.to_lowercase(), self.query.to_lowercase())
        };
        let mut start = 0;
        while let Some(pos) = search_text[start..].find(&search_query) {
            let abs_pos = start + pos;
            if !self.whole_word || Self::is_word_boundary(text, abs_pos, self.query.len()) {
                self.matches.push(abs_pos..abs_pos + self.query.len());
            }
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

    #[test]
    fn find_whole_word_skips_partial_matches() {
        let mut state = FindState::new();
        state.query = "cat".to_string();
        state.whole_word = true;
        state.find_in_text("concatenate cat scatter");
        // Only the standalone "cat" matches.
        assert_eq!(state.matches.len(), 1);
        assert_eq!(state.matches[0].start, 12);
    }

    #[test]
    fn find_whole_word_at_boundaries() {
        let mut state = FindState::new();
        state.query = "hello".to_string();
        state.whole_word = true;
        state.find_in_text("hello world hello_there");
        // "hello" at 0 matches (word boundary); "hello_there" at 12 does NOT.
        assert_eq!(state.matches.len(), 1);
        assert_eq!(state.matches[0].start, 0);
    }

    #[test]
    fn find_use_regex_finds_substring() {
        let mut state = FindState::new();
        state.query = "foo".to_string();
        state.use_regex = true;
        state.find_in_text("foobar foo baz foo");
        // Three occurrences: "foo" at 0, 7, 15.
        assert_eq!(state.matches.len(), 3);
    }

    #[test]
    fn find_regex_method_returns_positions() {
        let state = FindState { query: "ab".to_string(), case_sensitive: true, ..FindState::new() };
        let positions = state.find_regex("xabyzab");
        assert_eq!(positions, vec![1, 5]);
    }

    #[test]
    fn find_replace_regex_matches_pattern() {
        let mut state = FindState::new();
        // Pattern matches one or more digits.
        state.query = r"\d+".to_string();
        state.use_regex = true;
        state.find_in_text("abc 123 def 45 ghi");
        // "abc " = bytes 0-3, "123" = bytes 4-6, " def " = bytes 7-11, "45" = bytes 12-13
        assert_eq!(state.matches.len(), 2);
        assert_eq!(state.matches[0], 4..7);
        assert_eq!(state.matches[1], 12..14);
    }

    #[test]
    fn find_replace_regex_invalid_pattern_returns_empty() {
        let mut state = FindState::new();
        // Unclosed bracket is an invalid regex pattern.
        state.query = "[unclosed".to_string();
        state.use_regex = true;
        state.find_in_text("some text [unclosed here");
        // Must return empty matches rather than panicking or erroring.
        assert!(state.matches.is_empty());
    }

    #[test]
    fn find_match_in_multiline() {
        let mut state = FindState::new();
        state.query = "world".to_string();
        state.find_in_text("hello\nworld\nend");
        assert_eq!(state.matches.len(), 1);
        assert_eq!(state.matches[0].start, 6);
    }

    #[test]
    fn find_replace_replaces_first_match_rest_unchanged() {
        let mut state = FindState::new();
        state.query = "x".to_string();
        state.find_in_text("x y x z x");
        // current_match starts at 0
        let mut text = "x y x z x".to_string();
        let replaced = state.replace_current(&mut text, "Q");
        assert!(replaced);
        // Only the first "x" was replaced
        assert!(text.starts_with('Q'));
        // The rest of the original string still has 'x' characters
        assert!(text.contains('x'));
    }

    #[test]
    fn find_replace_no_match_returns_zero() {
        let mut state = FindState::new();
        state.query = "zzz".to_string();
        state.find_in_text("hello world");
        assert_eq!(state.matches.len(), 0);
    }

    #[test]
    fn regex_find_captures_group_position() {
        let mut state = FindState::new();
        // Match a digit sequence; we check the start positions via find_regex
        state.query = r"(\d+)".to_string();
        state.case_sensitive = true;
        let positions = state.find_regex("abc 42 def 7");
        assert_eq!(positions.len(), 2);
        assert_eq!(positions[0], 4);  // "42" starts at byte 4
        assert_eq!(positions[1], 11); // "7" starts at byte 11
    }

    #[test]
    fn find_empty_query_returns_no_matches() {
        let mut state = FindState::new();
        // query is empty by default
        state.find_in_text("some text here");
        assert!(state.matches.is_empty());
    }
}
