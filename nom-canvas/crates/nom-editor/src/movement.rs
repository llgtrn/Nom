//! Cursor movement primitives over a Rope buffer.
#![deny(unsafe_code)]

use ropey::Rope;

/// Character classification for word-boundary detection.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum CharClass {
    Whitespace,
    Word,
    Punct,
    Other,
}

/// Classify a single character.
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

/// Saturating left: returns prev char offset or 0 if already at start.
/// Offsets are in chars (matching ropey's char-indexed API).
pub fn left(_rope: &Rope, offset: usize) -> usize {
    if offset == 0 { 0 } else { offset - 1 }
}

/// Saturating right: returns next char offset or len_chars() if at end.
pub fn right(rope: &Rope, offset: usize) -> usize {
    let len = rope.len_chars();
    if offset >= len { len } else { offset + 1 }
}

/// Column of `offset` = number of chars since the start of its line.
pub fn column_at(rope: &Rope, offset: usize) -> u32 {
    if rope.len_chars() == 0 {
        return 0;
    }
    let offset = offset.min(rope.len_chars());
    let line = rope.char_to_line(offset);
    let line_start = rope.line_to_char(line);
    (offset - line_start) as u32
}

/// Line index (0-based) containing `offset`.
pub fn line_at(rope: &Rope, offset: usize) -> u32 {
    if rope.len_chars() == 0 {
        return 0;
    }
    let offset = offset.min(rope.len_chars());
    rope.char_to_line(offset) as u32
}

/// Char offset for (line, col), clamping col to line length.
pub fn offset_at(rope: &Rope, line: u32, col: u32) -> usize {
    let line = line as usize;
    let col = col as usize;
    let num_lines = rope.len_lines();
    if num_lines == 0 {
        return 0;
    }
    let line = line.min(num_lines.saturating_sub(1));
    let line_start = rope.line_to_char(line);
    // Line length excluding newline at end.
    let line_slice = rope.line(line);
    let line_len = {
        let s = line_slice.len_chars();
        // Trim trailing newline characters.
        let raw = line_slice.to_string();
        let trimmed = raw.trim_end_matches(|c| c == '\n' || c == '\r');
        trimmed.chars().count().min(s)
    };
    line_start + col.min(line_len)
}

/// Move up one visual line, preserving goal column.
/// Returns (new_offset, new_goal_col).
/// If already on line 0, returns offset 0 with goal_col preserved.
pub fn up(rope: &Rope, offset: usize, goal_col: u32) -> (usize, u32) {
    let cur_line = line_at(rope, offset);
    let cur_col = column_at(rope, offset);
    // Use the larger of current column and sticky goal.
    let effective_goal = goal_col.max(cur_col);
    if cur_line == 0 {
        return (0, effective_goal);
    }
    let new_line = cur_line - 1;
    let new_offset = offset_at(rope, new_line, effective_goal);
    (new_offset, effective_goal)
}

/// Move down one visual line, preserving goal column.
/// Returns (new_offset, new_goal_col).
/// If already on last line, returns end-of-buffer with goal_col preserved.
pub fn down(rope: &Rope, offset: usize, goal_col: u32) -> (usize, u32) {
    let cur_line = line_at(rope, offset);
    let cur_col = column_at(rope, offset);
    let effective_goal = goal_col.max(cur_col);
    let last_line = rope.len_lines().saturating_sub(1) as u32;
    if cur_line >= last_line {
        return (rope.len_chars(), effective_goal);
    }
    let new_line = cur_line + 1;
    let new_offset = offset_at(rope, new_line, effective_goal);
    (new_offset, effective_goal)
}

/// Move to start of previous word (skipping whitespace, then skipping same class).
pub fn prev_word_start(rope: &Rope, offset: usize) -> usize {
    if offset == 0 {
        return 0;
    }
    let mut pos = offset;
    // Step back over any trailing whitespace.
    while pos > 0 {
        let c = rope.char(pos - 1);
        if classify(c) != CharClass::Whitespace {
            break;
        }
        pos -= 1;
    }
    if pos == 0 {
        return 0;
    }
    // Step back while same class.
    let cls = classify(rope.char(pos - 1));
    while pos > 0 && classify(rope.char(pos - 1)) == cls {
        pos -= 1;
    }
    pos
}

/// Move to end of next word (skipping whitespace, then running through same class).
pub fn next_word_end(rope: &Rope, offset: usize) -> usize {
    let len = rope.len_chars();
    if offset >= len {
        return len;
    }
    let mut pos = offset;
    // Skip leading whitespace.
    while pos < len {
        let c = rope.char(pos);
        if classify(c) != CharClass::Whitespace {
            break;
        }
        pos += 1;
    }
    if pos >= len {
        return len;
    }
    // Advance while same class.
    let cls = classify(rope.char(pos));
    while pos < len && classify(rope.char(pos)) == cls {
        pos += 1;
    }
    pos
}

#[cfg(test)]
mod tests {
    use super::*;
    use ropey::Rope;

    fn rope(s: &str) -> Rope {
        Rope::from_str(s)
    }

    #[test]
    fn left_at_zero_returns_zero() {
        let r = rope("hello");
        assert_eq!(left(&r, 0), 0);
    }

    #[test]
    fn left_moves_back_one_char() {
        let r = rope("hello");
        assert_eq!(left(&r, 3), 2);
    }

    #[test]
    fn left_on_multibyte_char_lands_on_boundary() {
        // "é" is one char in ropey regardless of UTF-8 byte count.
        let r = rope("héllo");
        // offset 2 = after 'é'; left should go to 1.
        assert_eq!(left(&r, 2), 1);
    }

    #[test]
    fn right_past_end_returns_len() {
        let r = rope("hi");
        assert_eq!(right(&r, 2), 2);
        assert_eq!(right(&r, 5), 2);
    }

    #[test]
    fn right_moves_forward_one_char() {
        let r = rope("hello");
        assert_eq!(right(&r, 0), 1);
    }

    #[test]
    fn up_down_round_trip() {
        let r = rope("abc\ndef\nghi");
        // Start at offset 5 (line 1, col 1 = 'd'+1 = 'e').
        let (up_off, goal) = up(&r, 5, 0);
        // Should land on line 0, col 1 = offset 1.
        assert_eq!(up_off, 1);
        let (down_off, _) = down(&r, up_off, goal);
        // Back to line 1, col 1 = offset 5.
        assert_eq!(down_off, 5);
    }

    #[test]
    fn up_at_top_line_stays_at_zero_with_goal_preserved() {
        let r = rope("hello\nworld");
        let (off, goal) = up(&r, 2, 5);
        assert_eq!(off, 0);
        assert_eq!(goal, 5);
    }

    #[test]
    fn down_at_last_line_returns_end() {
        let r = rope("hello\nworld");
        let (off, _goal) = down(&r, 8, 0);
        assert_eq!(off, r.len_chars());
    }

    #[test]
    fn column_at_multiline() {
        let r = rope("abc\ndef\nghi");
        // offset 5 = line 1, col 1.
        assert_eq!(column_at(&r, 5), 1);
        // offset 0 = line 0, col 0.
        assert_eq!(column_at(&r, 0), 0);
    }

    #[test]
    fn offset_at_clamps_col() {
        let r = rope("hi\nbye");
        // line 0 has 2 chars; col 99 clamps to 2.
        assert_eq!(offset_at(&r, 0, 99), 2);
    }

    #[test]
    fn prev_word_skips_whitespace() {
        let r = rope("hello world");
        // offset 11 (end), should step back past " " then "world".
        assert_eq!(prev_word_start(&r, 11), 6);
    }

    #[test]
    fn next_word_hits_punctuation_boundary() {
        let r = rope("hello.world");
        // Starting at 0: advances through "hello" (Word class), stops at '.'.
        assert_eq!(next_word_end(&r, 0), 5);
    }

    #[test]
    fn classify_whitespace_word_punct() {
        assert_eq!(classify(' '), CharClass::Whitespace);
        assert_eq!(classify('\n'), CharClass::Whitespace);
        assert_eq!(classify('a'), CharClass::Word);
        assert_eq!(classify('_'), CharClass::Word);
        assert_eq!(classify('5'), CharClass::Word);
        assert_eq!(classify('.'), CharClass::Punct);
        assert_eq!(classify(';'), CharClass::Punct);
    }
}
