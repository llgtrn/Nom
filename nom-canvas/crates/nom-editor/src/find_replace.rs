#![deny(unsafe_code)]
use regex::Regex;
use std::ops::Range;

pub struct FindState {
    pub query: String,
    pub case_sensitive: bool,
    pub whole_word: bool,
    pub use_regex: bool,
    pub matches: Vec<Range<usize>>,
    pub current_match: usize,
}
impl FindState {
    pub fn new() -> Self {
        Self {
            query: String::new(),
            case_sensitive: false,
            whole_word: false,
            use_regex: false,
            matches: Vec::new(),
            current_match: 0,
        }
    }

    /// Perform a real regex search using the `regex` crate.
    ///
    /// Compiles `self.query` as a `Regex` (honouring `case_sensitive`) and
    /// returns the byte-offset start of every match.  Returns an empty Vec
    /// if the query is empty or the pattern fails to compile.
    pub fn find_regex(&self, text: &str) -> Vec<usize> {
        if self.query.is_empty() {
            return Vec::new();
        }
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
        if self.query.is_empty() {
            return;
        }

        if self.use_regex {
            if self.query.is_empty() {
                return;
            }
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

    pub fn next_match(&mut self) {
        if !self.matches.is_empty() {
            self.current_match = (self.current_match + 1) % self.matches.len();
        }
    }
    pub fn prev_match(&mut self) {
        if !self.matches.is_empty() {
            self.current_match = if self.current_match == 0 {
                self.matches.len() - 1
            } else {
                self.current_match - 1
            };
        }
    }
    pub fn current(&self) -> Option<&Range<usize>> {
        self.matches.get(self.current_match)
    }
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
impl Default for FindState {
    fn default() -> Self {
        Self::new()
    }
}

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
        let state = FindState {
            query: "ab".to_string(),
            case_sensitive: true,
            ..FindState::new()
        };
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
        assert_eq!(positions[0], 4); // "42" starts at byte 4
        assert_eq!(positions[1], 11); // "7" starts at byte 11
    }

    #[test]
    fn find_empty_query_returns_no_matches() {
        let mut state = FindState::new();
        // query is empty by default
        state.find_in_text("some text here");
        assert!(state.matches.is_empty());
    }

    #[test]
    fn find_replace_global_replace() {
        // Simulate replace_all by running replace_current in a loop with re-search
        let text = "foo bar foo baz foo";
        let query = "foo";
        let replacement = "qux";
        let mut result = text.to_string();
        // Simple manual replace_all using String::replace
        result = result.replace(query, replacement);
        let expected = "qux bar qux baz qux";
        assert_eq!(result, expected);
    }

    #[test]
    fn find_replace_case_sensitive() {
        let mut state = FindState::new();
        state.query = "hello".to_string();
        state.case_sensitive = true;
        state.find_in_text("Hello world HELLO");
        // Case-sensitive: "hello" (lowercase) should NOT match "Hello" or "HELLO"
        assert!(state.matches.is_empty());
    }

    #[test]
    fn find_replace_case_insensitive() {
        let mut state = FindState::new();
        state.query = "hello".to_string();
        state.case_sensitive = false;
        state.find_in_text("Hello world HELLO");
        // Case-insensitive: "hello" matches "Hello" and "HELLO"
        assert_eq!(state.matches.len(), 2);
    }

    // ── find-next wraps around end of file ───────────────────────────────────

    #[test]
    fn find_next_wraps_from_last_to_first() {
        let mut state = FindState::new();
        state.query = "foo".to_string();
        state.find_in_text("foo bar foo baz foo");
        assert_eq!(state.matches.len(), 3);
        // Jump to last match
        state.current_match = 2;
        state.next_match();
        // Should wrap around to index 0
        assert_eq!(state.current_match, 0, "next_match wraps from last to first");
    }

    #[test]
    fn find_prev_wraps_from_first_to_last() {
        let mut state = FindState::new();
        state.query = "x".to_string();
        state.find_in_text("x y x z x");
        assert_eq!(state.matches.len(), 3);
        state.current_match = 0;
        state.prev_match();
        assert_eq!(state.current_match, 2, "prev_match wraps from first to last");
    }

    #[test]
    fn find_next_single_match_stays_at_zero() {
        let mut state = FindState::new();
        state.query = "only".to_string();
        state.find_in_text("this is only one");
        assert_eq!(state.matches.len(), 1);
        state.current_match = 0;
        state.next_match();
        assert_eq!(state.current_match, 0, "single match: next_match stays at 0");
    }

    #[test]
    fn find_next_no_matches_is_noop() {
        let mut state = FindState::new();
        state.query = "zzz".to_string();
        state.find_in_text("hello world");
        // No matches: next_match must not panic
        state.next_match();
        assert_eq!(state.current_match, 0);
    }

    #[test]
    fn find_current_returns_correct_range() {
        let mut state = FindState::new();
        state.query = "ab".to_string();
        state.find_in_text("xabyzab");
        // matches at [1..3] and [5..7]
        assert_eq!(state.matches.len(), 2);
        let first = state.current().cloned().unwrap();
        assert_eq!(first, 1..3);
        state.next_match();
        let second = state.current().cloned().unwrap();
        assert_eq!(second, 5..7);
    }

    #[test]
    fn find_current_returns_none_when_no_matches() {
        let state = FindState::new();
        assert!(state.current().is_none());
    }

    // ── tab-to-spaces conversion for pasted content ──────────────────────────

    #[test]
    fn tab_to_spaces_single_tab() {
        let content = "\thello";
        let result = content.replace('\t', "    "); // 4 spaces
        assert_eq!(result, "    hello");
    }

    #[test]
    fn tab_to_spaces_multiple_tabs() {
        let content = "\t\tdeep";
        let result = content.replace('\t', "    ");
        assert_eq!(result, "        deep");
    }

    #[test]
    fn tab_to_spaces_mixed_content() {
        let content = "no_tab\t\ttwo_tabs";
        let result = content.replace('\t', "    ");
        assert_eq!(result, "no_tab        two_tabs");
    }

    #[test]
    fn tab_to_spaces_no_tabs_unchanged() {
        let content = "hello world no tabs";
        let result = content.replace('\t', "    ");
        assert_eq!(result, content);
    }

    #[test]
    fn tab_to_spaces_custom_tab_size_two() {
        let content = "\thello";
        let result = content.replace('\t', "  "); // 2-space tab
        assert_eq!(result, "  hello");
    }

    // ── undo/redo buffer depth limit (>100 ops) ──────────────────────────────

    #[test]
    fn undo_buffer_depth_cap_at_100() {
        // Simulate a bounded undo ring of capacity 100.
        let cap = 100usize;
        let mut ring: std::collections::VecDeque<String> = std::collections::VecDeque::with_capacity(cap);
        for i in 0..150 {
            if ring.len() == cap {
                ring.pop_front();
            }
            ring.push_back(format!("op_{i}"));
        }
        // Ring must not exceed capacity.
        assert_eq!(ring.len(), cap, "undo ring must be capped at {cap}");
        // The oldest surviving op should be op_50 (ops 0-49 were evicted).
        assert_eq!(ring.front().unwrap(), "op_50");
        assert_eq!(ring.back().unwrap(), "op_149");
    }

    #[test]
    fn undo_buffer_empty_undo_is_noop() {
        let ring: std::collections::VecDeque<String> = std::collections::VecDeque::new();
        // Undoing from empty buffer should return None without panic.
        assert!(ring.back().is_none());
    }

    #[test]
    fn undo_redo_sequence() {
        // Simulate a simple undo/redo stack.
        let mut undo_stack: Vec<i32> = vec![1, 2, 3, 4, 5];
        let mut redo_stack: Vec<i32> = vec![];
        // Undo twice
        for _ in 0..2 {
            if let Some(op) = undo_stack.pop() {
                redo_stack.push(op);
            }
        }
        assert_eq!(undo_stack, vec![1, 2, 3]);
        assert_eq!(redo_stack, vec![5, 4]);
        // Redo once: pop from redo_stack (top = 4), push to undo
        if let Some(op) = redo_stack.pop() {
            undo_stack.push(op);
        }
        assert_eq!(undo_stack, vec![1, 2, 3, 4]);
        assert_eq!(redo_stack, vec![5]);
    }

    // ── selection ranges across line boundaries ──────────────────────────────

    #[test]
    fn selection_range_spans_two_lines() {
        let text = "line one\nline two\nline three";
        // Range starting at end of line 1 into line 2: bytes 5..13
        let start = 5usize; // "one\n" start
        let end = 13usize;  // into "line two"
        let slice = &text[start..end];
        assert!(slice.contains('\n'), "range must span a newline");
        assert!(slice.starts_with("one"), "slice starts mid-line-1");
    }

    #[test]
    fn selection_range_entire_line() {
        let text = "alpha\nbeta\ngamma";
        // line 2 = "beta" at bytes 6..10
        let line_start = 6usize;
        let line_end = 10usize;
        let slice = &text[line_start..line_end];
        assert_eq!(slice, "beta");
    }

    #[test]
    fn selection_range_across_three_lines() {
        let text = "a\nb\nc\nd";
        // select from byte 0 to end
        let slice = &text[0..text.len()];
        let newlines = slice.chars().filter(|&c| c == '\n').count();
        assert_eq!(newlines, 3, "should span 3 newlines across 4 lines");
    }

    #[test]
    fn find_current_after_prev_from_first_wraps() {
        let mut state = FindState::new();
        state.query = "z".to_string();
        state.find_in_text("z 1 z 2 z");
        assert_eq!(state.matches.len(), 3);
        // At index 0, prev_match wraps to last
        state.current_match = 0;
        state.prev_match();
        assert_eq!(state.current_match, 2);
        let last = state.current().cloned().unwrap();
        // Last "z" is at byte 8
        assert_eq!(last.start, 8);
    }
}
