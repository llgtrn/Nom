// Find + replace primitives over source text.
//
// No regex dependency — only std str operations.  Patterns are plain
// text with optional case-insensitive / whole-word modes.

use std::ops::Range;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FindOptions {
    pub case_sensitive: bool,
    pub whole_word: bool,
}

impl Default for FindOptions {
    fn default() -> Self { Self { case_sensitive: true, whole_word: false } }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Match {
    pub range: Range<usize>,
}

/// Find ALL non-overlapping matches of `pattern` in `haystack`.
pub fn find_all(haystack: &str, pattern: &str, options: FindOptions) -> Vec<Match> {
    if pattern.is_empty() { return vec![]; }
    let mut out = Vec::new();
    if options.case_sensitive {
        scan_all(haystack, pattern, options.whole_word, &mut out);
    } else {
        let lh = haystack.to_lowercase();
        let lp = pattern.to_lowercase();
        // lowercased search — but emit ranges on the ORIGINAL haystack.  Works
        // for ASCII text; for non-ASCII we conservatively approximate by using
        // the same ranges, which is correct when uppercase/lowercase are
        // same byte-length (true for Latin-1).
        if haystack.len() == lh.len() && pattern.len() == lp.len() {
            scan_all(&lh, &lp, options.whole_word, &mut out);
        } else {
            // Fallback: naive case-insensitive chars scan.
            naive_ci_scan(haystack, pattern, options.whole_word, &mut out);
        }
    }
    out
}

fn scan_all(haystack: &str, pattern: &str, whole_word: bool, out: &mut Vec<Match>) {
    let mut start = 0usize;
    while let Some(i) = haystack[start..].find(pattern) {
        let match_start = start + i;
        let match_end = match_start + pattern.len();
        if !whole_word || is_whole_word(haystack, match_start, match_end) {
            out.push(Match { range: match_start..match_end });
        }
        start = match_end;
        if start > haystack.len() { break; }
    }
}

fn naive_ci_scan(haystack: &str, pattern: &str, whole_word: bool, out: &mut Vec<Match>) {
    let hay_chars: Vec<(usize, char)> = haystack.char_indices().collect();
    let pat_chars: Vec<char> = pattern.chars().flat_map(|c| c.to_lowercase()).collect();
    if pat_chars.is_empty() { return; }
    let mut i = 0usize;
    while i + pat_chars.len() <= hay_chars.len() {
        let window: Vec<char> = hay_chars[i..i + pat_chars.len()]
            .iter()
            .flat_map(|(_, c)| c.to_lowercase())
            .collect();
        if window == pat_chars {
            let start = hay_chars[i].0;
            let end = if i + pat_chars.len() < hay_chars.len() {
                hay_chars[i + pat_chars.len()].0
            } else {
                haystack.len()
            };
            if !whole_word || is_whole_word(haystack, start, end) {
                out.push(Match { range: start..end });
            }
            i += pat_chars.len();
        } else {
            i += 1;
        }
    }
}

fn is_whole_word(haystack: &str, start: usize, end: usize) -> bool {
    let prev_ok = start == 0
        || haystack[..start].chars().last().map(|c| !is_word_char(c)).unwrap_or(true);
    let next_ok = end == haystack.len()
        || haystack[end..].chars().next().map(|c| !is_word_char(c)).unwrap_or(true);
    prev_ok && next_ok
}

fn is_word_char(c: char) -> bool { c.is_alphanumeric() || c == '_' }

/// Replace ALL non-overlapping matches of `pattern` with `replacement`.
pub fn replace_all(haystack: &str, pattern: &str, replacement: &str, options: FindOptions) -> String {
    let matches = find_all(haystack, pattern, options);
    if matches.is_empty() { return haystack.to_string(); }
    let mut out = String::with_capacity(haystack.len());
    let mut cursor = 0usize;
    for m in matches {
        out.push_str(&haystack[cursor..m.range.start]);
        out.push_str(replacement);
        cursor = m.range.end;
    }
    out.push_str(&haystack[cursor..]);
    out
}

/// Find next match at or after `from` (inclusive start).
pub fn find_next(haystack: &str, pattern: &str, from: usize, options: FindOptions) -> Option<Match> {
    let from = from.min(haystack.len());
    find_all(&haystack[from..], pattern, options)
        .into_iter()
        .next()
        .map(|m| Match { range: (m.range.start + from)..(m.range.end + from) })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cs() -> FindOptions { FindOptions { case_sensitive: true, whole_word: false } }
    fn ci() -> FindOptions { FindOptions { case_sensitive: false, whole_word: false } }
    fn cs_ww() -> FindOptions { FindOptions { case_sensitive: true, whole_word: true } }
    fn ci_ww() -> FindOptions { FindOptions { case_sensitive: false, whole_word: true } }

    #[test]
    fn empty_pattern_returns_no_matches() {
        assert!(find_all("hello world", "", cs()).is_empty());
    }

    #[test]
    fn basic_case_sensitive_two_matches() {
        let m = find_all("foo bar foo", "foo", cs());
        assert_eq!(m.len(), 2);
        assert_eq!(m[0].range, 0..3);
        assert_eq!(m[1].range, 8..11);
    }

    #[test]
    fn no_match_returns_empty() {
        assert!(find_all("hello world", "xyz", cs()).is_empty());
    }

    #[test]
    fn case_insensitive_three_matches() {
        let m = find_all("foo Foo FOO", "FOO", ci());
        assert_eq!(m.len(), 3);
        assert_eq!(m[0].range, 0..3);
        assert_eq!(m[1].range, 4..7);
        assert_eq!(m[2].range, 8..11);
    }

    #[test]
    fn whole_word_skips_prefix_match() {
        // "cat" in "cat category" — only first "cat" is a whole word
        let m = find_all("cat category", "cat", cs_ww());
        assert_eq!(m.len(), 1);
        assert_eq!(m[0].range, 0..3);
    }

    #[test]
    fn whole_word_two_standalone_matches() {
        let m = find_all("cat cat", "cat", cs_ww());
        assert_eq!(m.len(), 2);
        assert_eq!(m[0].range, 0..3);
        assert_eq!(m[1].range, 4..7);
    }

    #[test]
    fn whole_word_false_matches_anywhere() {
        let m = find_all("category", "cat", cs());
        assert_eq!(m.len(), 1);
        assert_eq!(m[0].range, 0..3);
    }

    #[test]
    fn replace_all_basic() {
        let result = replace_all("foo bar foo", "foo", "baz", cs());
        assert_eq!(result, "baz bar baz");
    }

    #[test]
    fn replace_all_no_match_returns_unchanged() {
        let result = replace_all("hello world", "xyz", "zzz", cs());
        assert_eq!(result, "hello world");
    }

    #[test]
    fn replace_all_case_insensitive_keeps_surrounding_context() {
        // replacements happen but surrounding text is untouched
        let result = replace_all("say Hello and hello", "hello", "hi", ci());
        assert_eq!(result, "say hi and hi");
    }

    #[test]
    fn find_next_from_offset_skips_earlier_matches() {
        // "foo" at 0 and 8; start from 1 → should get only 8..11
        let m = find_next("foo bar foo", "foo", 1, cs());
        assert_eq!(m, Some(Match { range: 8..11 }));
    }

    #[test]
    fn find_next_past_end_returns_none() {
        let m = find_next("foo", "foo", 100, cs());
        assert_eq!(m, None);
    }

    #[test]
    fn match_at_very_start_and_very_end() {
        let m = find_all("abcabc", "abc", cs());
        assert_eq!(m.len(), 2);
        assert_eq!(m[0].range, 0..3);
        assert_eq!(m[1].range, 3..6);
    }

    #[test]
    fn non_overlapping_aaa() {
        // "aa" in "aaa" → only 1 non-overlapping match (0..2)
        let m = find_all("aaa", "aa", cs());
        assert_eq!(m.len(), 1);
        assert_eq!(m[0].range, 0..2);
    }

    #[test]
    fn whole_word_case_insensitive() {
        // "CAT" case-insensitively as whole word in "cat category"
        let m = find_all("cat category", "CAT", ci_ww());
        assert_eq!(m.len(), 1);
        assert_eq!(m[0].range, 0..3);
    }
}
