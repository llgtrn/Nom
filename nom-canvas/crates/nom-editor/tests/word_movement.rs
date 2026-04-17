// Integration tests for word_movement — compiled as a standalone test crate.
// We cannot use include! with files that contain //! or #![...] inside a mod block.
// Instead we inline the necessary items from movement.rs and word_movement.rs here.
#![deny(unsafe_code)]
#![allow(dead_code)]

// ── Re-inline movement items needed by word_movement ──────────────────────

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum CharClass {
    Whitespace,
    Word,
    Punct,
    Other,
}

pub fn classify(c: char) -> CharClass {
    if c.is_whitespace() {
        CharClass::Whitespace
    } else if c.is_alphanumeric() || c == '_' {
        CharClass::Word
    } else if c.is_ascii_punctuation() {
        CharClass::Punct
    } else {
        CharClass::Other
    }
}

// ── Inline word_movement source ────────────────────────────────────────────

fn is_subword_boundary(a: char, b: char) -> bool {
    if a.is_lowercase() && b.is_uppercase() {
        return true;
    }
    if a.is_ascii_digit() != b.is_ascii_digit() && (a.is_alphabetic() || b.is_alphabetic()) {
        return true;
    }
    let is_sep = |c: char| c == '_' || c == '-';
    if is_sep(a) || is_sep(b) {
        return true;
    }
    classify(a) != classify(b)
}

pub fn prev_subword_start(source: &str, offset: usize) -> usize {
    if offset == 0 {
        return 0;
    }
    let chars: Vec<(usize, char)> = source.char_indices().collect();
    let mut i = chars
        .iter()
        .position(|(o, c)| *o + c.len_utf8() == offset)
        .unwrap_or(chars.len());
    if i == 0 {
        return 0;
    }
    i = i.saturating_sub(1);
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

pub fn next_subword_end(source: &str, offset: usize) -> usize {
    let chars: Vec<(usize, char)> = source.char_indices().collect();
    let mut i = chars
        .iter()
        .position(|(o, _)| *o >= offset)
        .unwrap_or(chars.len());
    if i >= chars.len() {
        return source.len();
    }
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

pub fn paragraph_start(source: &str, offset: usize) -> usize {
    let offset = offset.min(source.len());
    if offset == 0 {
        return 0;
    }
    let before = &source[..offset];
    let mut pos = before.len();
    for line in before.rsplit('\n') {
        if line.trim().is_empty() {
            // pos points to the '\n'; paragraph starts after it.
            return pos + 1;
        }
        pos = pos.saturating_sub(line.len() + 1);
    }
    0
}

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
        pos += line.len() + 1;
    }
    source.len()
}

// ── Tests ──────────────────────────────────────────────────────────────────

#[test]
fn prev_subword_camel_from_end() {
    let s = "helloWorld";
    assert_eq!(prev_subword_start(s, s.len()), 5);
}

#[test]
fn prev_subword_from_zero_returns_zero() {
    assert_eq!(prev_subword_start("helloWorld", 0), 0);
}

#[test]
fn prev_subword_snake_bar() {
    let s = "foo_bar";
    assert_eq!(prev_subword_start(s, s.len()), 4);
}

#[test]
fn prev_subword_snake_foo() {
    let s = "foo_bar";
    assert_eq!(prev_subword_start(s, 3), 0);
}

#[test]
fn prev_subword_digit_boundary() {
    let s = "abc123";
    assert_eq!(prev_subword_start(s, s.len()), 3);
}

#[test]
fn next_subword_camel_from_start() {
    let s = "helloWorld";
    assert_eq!(next_subword_end(s, 0), 5);
}

#[test]
fn next_subword_snake_first_word() {
    let s = "foo_bar";
    assert_eq!(next_subword_end(s, 0), 3);
}

#[test]
fn next_subword_digit_run() {
    let s = "abc123";
    assert_eq!(next_subword_end(s, 0), 3);
}

#[test]
fn next_subword_at_end_returns_len() {
    let s = "hello";
    assert_eq!(next_subword_end(s, s.len()), s.len());
}

#[test]
fn bracket_forward_simple() {
    assert_eq!(matching_bracket("(foo)", 0), Some(4));
}

#[test]
fn bracket_backward_simple() {
    assert_eq!(matching_bracket("(foo)", 4), Some(0));
}

#[test]
fn bracket_unmatched_returns_none() {
    assert_eq!(matching_bracket("((foo)", 0), None);
}

#[test]
fn bracket_nested_outer_close() {
    assert_eq!(matching_bracket("((foo))", 0), Some(6));
}

#[test]
fn bracket_non_bracket_returns_none() {
    assert_eq!(matching_bracket("foo", 0), None);
}

#[test]
fn bracket_square_brackets() {
    assert_eq!(matching_bracket("[bar]", 0), Some(4));
}

#[test]
fn paragraph_start_at_file_start_returns_zero() {
    let s = "hello\nworld\n";
    assert_eq!(paragraph_start(s, 0), 0);
}

#[test]
fn paragraph_start_finds_blank_line_boundary() {
    // "first\n\nsecond" — cursor at 9 ('s'). Blank line ends at byte 6; para starts at 7.
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
    // "first\n\nsecond" — from 0, paragraph ends at byte 6 (the blank '\n').
    let s = "first\n\nsecond";
    assert_eq!(paragraph_end(s, 0), 6);
}
