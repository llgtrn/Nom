//! Enhanced cursor-movement helpers: subword, bracket-match, paragraph jumps.
#![deny(unsafe_code)]

use crate::movement::{classify, CharClass};

/// Move to the previous SUBWORD start.  A subword break occurs on:
///   - camelCase boundary (lowercase -> uppercase)
///   - snake_case underscore / kebab-case hyphen
///   - digit <-> letter boundary
pub fn prev_subword_start(source: &str, offset: usize) -> usize {
    if offset == 0 {
        return 0;
    }
    let chars: Vec<(usize, char)> = source.char_indices().collect();
    // Find index of the char ending at `offset`.
    let mut i = chars
        .iter()
        .position(|(o, c)| *o + c.len_utf8() == offset)
        .unwrap_or(chars.len());
    if i == 0 {
        return 0;
    }
    i = i.saturating_sub(1);
    // Walk back until we hit a subword boundary.
    while i > 0 {
        let (_, prev) = chars[i - 1];
        let (_, cur) = chars[i];
        if is_subword_boundary(prev, cur) {
            break;
        }
        i -= 1;
    }
    chars.get(i).map(|(o, _)| *o).unwrap_or(0)
}

/// Move to the next SUBWORD end.
pub fn next_subword_end(source: &str, offset: usize) -> usize {
    let chars: Vec<(usize, char)> = source.char_indices().collect();
    let mut i = chars
        .iter()
        .position(|(o, _)| *o >= offset)
        .unwrap_or(chars.len());
    if i >= chars.len() {
        return source.len();
    }
    // Walk forward until boundary.
    while i + 1 < chars.len() {
        let (_, cur) = chars[i];
        let (_, next) = chars[i + 1];
        if is_subword_boundary(cur, next) {
            break;
        }
        i += 1;
    }
    chars.get(i + 1).map(|(o, _)| *o).unwrap_or(source.len())
}

fn is_subword_boundary(a: char, b: char) -> bool {
    // camelCase: lowercase -> uppercase
    if a.is_lowercase() && b.is_uppercase() {
        return true;
    }
    // digit <-> letter boundary
    if a.is_ascii_digit() != b.is_ascii_digit() && (a.is_alphabetic() || b.is_alphabetic()) {
        return true;
    }
    // separator characters
    let is_sep = |c: char| c == '_' || c == '-';
    if is_sep(a) || is_sep(b) {
        return true;
    }
    // Different CharClass in general also counts.
    classify(a) != classify(b)
}

/// Match brackets: given offset pointing at a bracket `(`, find its closing `)`.
/// Returns None if unmatched or if offset isn't on a bracket.
pub fn matching_bracket(source: &str, offset: usize) -> Option<usize> {
    let chars: Vec<(usize, char)> = source.char_indices().collect();
    let idx = chars.iter().position(|(o, _)| *o == offset)?;
    let (_, bracket) = chars[idx];
    let (open, close, forward) = match bracket {
        '(' => ('(', ')', true),
        ')' => ('(', ')', false),
        '[' => ('[', ']', true),
        ']' => ('[', ']', false),
        '{' => ('{', '}', true),
        '}' => ('{', '}', false),
        _ => return None,
    };
    let mut depth = 1i32;
    if forward {
        for i in (idx + 1)..chars.len() {
            let (o, c) = chars[i];
            if c == open {
                depth += 1;
            } else if c == close {
                depth -= 1;
                if depth == 0 {
                    return Some(o);
                }
            }
        }
    } else {
        for i in (0..idx).rev() {
            let (o, c) = chars[i];
            if c == close {
                depth += 1;
            } else if c == open {
                depth -= 1;
                if depth == 0 {
                    return Some(o);
                }
            }
        }
    }
    None
}

/// Move to the start of the current paragraph (walk backward over
/// non-blank lines until a blank line or start-of-file).
pub fn paragraph_start(source: &str, offset: usize) -> usize {
    let offset = offset.min(source.len());
    if offset == 0 {
        return 0;
    }
    let before = &source[..offset];
    // Walk backwards line-by-line.
    let mut pos = before.len();
    for line in before.rsplit('\n') {
        if line.trim().is_empty() {
            // pos points to the '\n'; paragraph starts after it.
            return pos + 1;
        }
        pos = pos.saturating_sub(line.len() + 1); // +1 for the '\n'
    }
    // Reached beginning of file without finding a blank line.
    0
}

/// Move to the end of the current paragraph (walk forward until a blank line
/// or end-of-file).
pub fn paragraph_end(source: &str, offset: usize) -> usize {
    let offset = offset.min(source.len());
    let after = &source[offset..];
    let mut pos = offset;
    let mut first = true;
    for line in after.split('\n') {
        if !first && line.trim().is_empty() {
            return pos;
        }
        first = false;
        pos += line.len() + 1; // +1 for '\n'
    }
    source.len()
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── prev_subword_start ──────────────────────────────────────────────────

    #[test]
    fn prev_subword_camel_from_end() {
        // "helloWorld" — from offset 10 (end), should land on 'W' at byte 5.
        let s = "helloWorld";
        assert_eq!(prev_subword_start(s, s.len()), 5);
    }

    #[test]
    fn prev_subword_from_zero_returns_zero() {
        assert_eq!(prev_subword_start("helloWorld", 0), 0);
    }

    #[test]
    fn prev_subword_snake_bar() {
        // "foo_bar" from end (7) → first step lands past '_' to 'b' at 4.
        // The '_' itself is a separator boundary, so 'b' starts the subword.
        let s = "foo_bar";
        assert_eq!(prev_subword_start(s, s.len()), 4);
    }

    #[test]
    fn prev_subword_snake_foo() {
        // From 3 (pointing at '_') should land at 0 ('f').
        let s = "foo_bar";
        assert_eq!(prev_subword_start(s, 3), 0);
    }

    #[test]
    fn prev_subword_digit_boundary() {
        // "abc123" — from end (6) should land at 3 ('1'), the digit run start.
        let s = "abc123";
        assert_eq!(prev_subword_start(s, s.len()), 3);
    }

    // ── next_subword_end ────────────────────────────────────────────────────

    #[test]
    fn next_subword_camel_from_start() {
        // "helloWorld" from 0 → advances past "hello" → stops before 'W' at 5.
        let s = "helloWorld";
        assert_eq!(next_subword_end(s, 0), 5);
    }

    #[test]
    fn next_subword_snake_first_word() {
        // "foo_bar" from 0 → stops at '_' boundary; end of "foo" = byte 3.
        let s = "foo_bar";
        assert_eq!(next_subword_end(s, 0), 3);
    }

    #[test]
    fn next_subword_digit_run() {
        // "abc123" from 0 → stops at digit boundary at byte 3.
        let s = "abc123";
        assert_eq!(next_subword_end(s, 0), 3);
    }

    #[test]
    fn next_subword_at_end_returns_len() {
        let s = "hello";
        assert_eq!(next_subword_end(s, s.len()), s.len());
    }

    // ── matching_bracket ────────────────────────────────────────────────────

    #[test]
    fn bracket_forward_simple() {
        // "(foo)" — offset 0 is '(', closing ')' is at byte 4.
        assert_eq!(matching_bracket("(foo)", 0), Some(4));
    }

    #[test]
    fn bracket_backward_simple() {
        // "(foo)" — offset 4 is ')', opening '(' is at byte 0.
        assert_eq!(matching_bracket("(foo)", 4), Some(0));
    }

    #[test]
    fn bracket_unmatched_returns_none() {
        // "((foo)" — outer '(' at 0 has no matching ')'.
        assert_eq!(matching_bracket("((foo)", 0), None);
    }

    #[test]
    fn bracket_nested_outer_close() {
        // "((foo))" — offset 0 is outer '(', matching ')' is at byte 6.
        assert_eq!(matching_bracket("((foo))", 0), Some(6));
    }

    #[test]
    fn bracket_non_bracket_returns_none() {
        // 'f' is not a bracket.
        assert_eq!(matching_bracket("foo", 0), None);
    }

    #[test]
    fn bracket_square_brackets() {
        // "[bar]" — offset 0 → Some(4).
        assert_eq!(matching_bracket("[bar]", 0), Some(4));
    }

    // ── paragraph_start / paragraph_end ────────────────────────────────────

    #[test]
    fn paragraph_start_at_file_start_returns_zero() {
        let s = "hello\nworld\n";
        assert_eq!(paragraph_start(s, 0), 0);
    }

    #[test]
    fn paragraph_start_finds_blank_line_boundary() {
        // "first\n\nsecond" — cursor at offset 9 ('s' of "second").
        // Blank line at byte 6; paragraph starts at 7.
        let s = "first\n\nsecond";
        assert_eq!(paragraph_start(s, 9), 7);
    }

    #[test]
    fn paragraph_end_at_end_returns_len() {
        let s = "hello\nworld";
        assert_eq!(paragraph_end(s, s.len()), s.len());
    }

    #[test]
    fn paragraph_end_stops_at_blank_line() {
        // "first\n\nsecond" — from offset 0, paragraph ends at byte 6 (the blank line).
        let s = "first\n\nsecond";
        assert_eq!(paragraph_end(s, 0), 6);
    }
}
