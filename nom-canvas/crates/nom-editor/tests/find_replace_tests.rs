// Integration tests for find_replace — compiled as a standalone test crate
// so they don't require find_replace to be declared in lib.rs.

// Include the module source directly into this test binary.
include!("../src/find_replace.rs");

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
    let result = replace_all("say Hello and hello", "hello", "hi", ci());
    assert_eq!(result, "say hi and hi");
}

#[test]
fn find_next_from_offset_skips_earlier_matches() {
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
    let m = find_all("aaa", "aa", cs());
    assert_eq!(m.len(), 1);
    assert_eq!(m[0].range, 0..2);
}

#[test]
fn whole_word_case_insensitive() {
    let m = find_all("cat category", "CAT", ci_ww());
    assert_eq!(m.len(), 1);
    assert_eq!(m[0].range, 0..3);
}
