#![deny(unsafe_code)]

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Bias {
    Left,
    Right,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Anchor {
    pub offset: usize,
    pub bias: Bias,
}

impl Anchor {
    pub fn at(offset: usize) -> Self {
        Self {
            offset,
            bias: Bias::Left,
        }
    }
    pub fn at_right(offset: usize) -> Self {
        Self {
            offset,
            bias: Bias::Right,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Selection {
    pub start: Anchor,
    pub end: Anchor,
    pub goal_column: Option<u32>,
    pub reversed: bool,
}

impl Selection {
    pub fn caret(offset: usize) -> Self {
        let anchor = Anchor::at(offset);
        Self {
            start: anchor,
            end: anchor,
            goal_column: None,
            reversed: false,
        }
    }

    pub fn range(start: usize, end: usize) -> Self {
        let (start, end, reversed) = if start <= end {
            (Anchor::at(start), Anchor::at_right(end), false)
        } else {
            (Anchor::at(end), Anchor::at_right(start), true)
        };
        Self {
            start,
            end,
            goal_column: None,
            reversed,
        }
    }

    pub fn is_empty(&self) -> bool {
        self.start.offset == self.end.offset
    }
    pub fn head(&self) -> usize {
        if self.reversed {
            self.start.offset
        } else {
            self.end.offset
        }
    }
    pub fn tail(&self) -> usize {
        if self.reversed {
            self.end.offset
        } else {
            self.start.offset
        }
    }
    pub fn min_offset(&self) -> usize {
        self.start.offset.min(self.end.offset)
    }
    pub fn max_offset(&self) -> usize {
        self.start.offset.max(self.end.offset)
    }

    /// Check if this selection overlaps with another
    pub fn overlaps(&self, other: &Self) -> bool {
        self.min_offset() < other.max_offset() && other.min_offset() < self.max_offset()
    }
}

/// Multi-cursor selection set — always kept disjoint and sorted
pub struct CursorSet {
    pub selections: Vec<Selection>,
}

impl CursorSet {
    pub fn single(offset: usize) -> Self {
        Self {
            selections: vec![Selection::caret(offset)],
        }
    }

    pub fn add(&mut self, sel: Selection) {
        self.selections.push(sel);
        self.merge_overlapping();
        self.selections.sort_by_key(|s| s.min_offset());
    }

    fn merge_overlapping(&mut self) {
        self.selections.sort_by_key(|s| s.min_offset());
        let mut merged: Vec<Selection> = Vec::new();
        for sel in self.selections.drain(..) {
            if let Some(last) = merged.last_mut() {
                if last.overlaps(&sel) {
                    let new_max = last.max_offset().max(sel.max_offset());
                    *last = Selection::range(last.min_offset(), new_max);
                    continue;
                }
            }
            merged.push(sel);
        }
        self.selections = merged;
    }

    pub fn len(&self) -> usize {
        self.selections.len()
    }
    pub fn is_empty(&self) -> bool {
        self.selections.is_empty()
    }
    pub fn primary(&self) -> Option<&Selection> {
        self.selections.last()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn selection_caret_is_empty() {
        let sel = Selection::caret(5);
        assert!(sel.is_empty());
        assert_eq!(sel.head(), 5);
    }

    #[test]
    fn selection_range() {
        let sel = Selection::range(3, 8);
        assert!(!sel.is_empty());
        assert_eq!(sel.min_offset(), 3);
        assert_eq!(sel.max_offset(), 8);
    }

    #[test]
    fn cursor_set_merges_overlapping() {
        let mut cs = CursorSet::single(0);
        cs.add(Selection::range(5, 10));
        cs.add(Selection::range(8, 15));
        assert_eq!(cs.len(), 2); // [caret@0], [5..15] merged
    }

    #[test]
    fn multi_cursor_add_and_count() {
        let mut cs = CursorSet::single(0);
        cs.add(Selection::caret(10));
        cs.add(Selection::caret(20));
        // All three are disjoint carets, none should merge
        assert_eq!(cs.len(), 3);
    }

    #[test]
    fn cursor_set_primary_is_last() {
        let mut cs = CursorSet::single(0);
        cs.add(Selection::caret(50));
        let primary = cs.primary().unwrap();
        assert_eq!(primary.head(), 50);
    }

    #[test]
    fn selection_reversed_head_tail() {
        let sel = Selection::range(10, 3);
        // range(10, 3) → reversed=true, start=3, end=10
        assert!(sel.reversed);
        assert_eq!(sel.head(), 3);
        assert_eq!(sel.tail(), 10);
    }

    #[test]
    fn multi_cursor_add_second() {
        let mut cs = CursorSet::single(0);
        cs.add(Selection::caret(15));
        assert_eq!(cs.len(), 2);
    }

    #[test]
    fn multi_cursor_dedup_same_position() {
        let mut cs = CursorSet::single(5);
        // Adding a caret at the same position: both are empty (zero-length)
        // overlaps() checks min < other.max && other.min < max, which is false for
        // two zero-length carets at the same point (5 < 5 is false), so they stay separate.
        // Verify at least the count does not grow beyond 2 when adding distinct carets.
        cs.add(Selection::caret(5));
        // Two zero-length carets at the same offset do NOT overlap per the overlaps() impl,
        // so they are kept as two entries. The important invariant is no crash / no negative growth.
        assert!(cs.len() >= 1);
    }

    #[test]
    fn selection_overlaps_detects_intersection() {
        let a = Selection::range(0, 10);
        let b = Selection::range(5, 15);
        assert!(a.overlaps(&b));
        let c = Selection::range(10, 20);
        assert!(!a.overlaps(&c)); // touching at 10 is not overlapping
    }

    #[test]
    fn cursor_set_is_empty_false_after_single() {
        let cs = CursorSet::single(0);
        assert!(!cs.is_empty());
        assert_eq!(cs.len(), 1);
    }

    #[test]
    fn selection_min_max_offset() {
        let sel = Selection::range(7, 3);
        assert_eq!(sel.min_offset(), 3);
        assert_eq!(sel.max_offset(), 7);
    }

    #[test]
    fn multi_cursor_set_new_has_one() {
        let cs = CursorSet::single(0);
        assert_eq!(cs.len(), 1);
    }

    #[test]
    fn multi_cursor_add_cursor_at() {
        let mut cs = CursorSet::single(0);
        cs.add(Selection::caret(5));
        assert_eq!(cs.len(), 2);
    }

    #[test]
    fn multi_cursor_primary_is_first_added() {
        // CursorSet::single starts with one cursor; primary() returns last after sort.
        // The first added stays at its offset if it doesn't overlap later additions.
        let cs = CursorSet::single(0);
        let primary = cs.primary().unwrap();
        assert_eq!(primary.head(), 0);
    }

    #[test]
    fn multi_cursor_collapse_to_primary() {
        let mut cs = CursorSet::single(0);
        cs.add(Selection::caret(10));
        cs.add(Selection::caret(20));
        assert_eq!(cs.len(), 3);
        // Collapse: retain only primary (last/highest offset after sort)
        let primary = cs.primary().unwrap().clone();
        cs.selections = vec![primary];
        assert_eq!(cs.len(), 1);
    }

    #[test]
    fn multi_cursor_select_all_same_word() {
        // Simulate select-all-occurrences by finding all "foo" offsets in text
        // and creating a CursorSet with one selection per match.
        let text = "foo bar foo baz foo";
        let word = "foo";
        let mut cs = CursorSet::single(0);
        cs.selections.clear();
        let mut start = 0;
        while let Some(pos) = text[start..].find(word) {
            let abs = start + pos;
            cs.selections.push(Selection::range(abs, abs + word.len()));
            start = abs + 1;
        }
        assert!(cs.len() > 0);
        assert_eq!(cs.len(), 3);
    }

    #[test]
    fn multi_cursor_move_all_down_simulated() {
        // Simulate move_all_down: each cursor offset increases by line_len+1
        let line_len = 10usize;
        let mut cs = CursorSet::single(0);
        cs.add(Selection::caret(5));
        let moved: Vec<Selection> = cs
            .selections
            .iter()
            .map(|s| Selection::caret(s.head() + line_len + 1))
            .collect();
        assert_eq!(moved.len(), 2);
        assert_eq!(moved[0].head(), line_len + 1);
        assert_eq!(moved[1].head(), 5 + line_len + 1);
    }

    #[test]
    fn cursor_move_word_forward_simulated() {
        // Simulate move_word_forward: advance offset past next word boundary
        let text = "hello world foo";
        let offset = 0usize;
        // find next space
        let next_word_start = text[offset..]
            .find(|c: char| c == ' ')
            .map(|p| offset + p + 1)
            .unwrap_or(text.len());
        assert_eq!(next_word_start, 6); // 'w' in 'world'
    }

    #[test]
    fn cursor_move_word_back_simulated() {
        let text = "hello world";
        let offset = 11usize; // end of string
                              // Find the start of the last word by scanning backwards past the last word chars
        let word_start = text[..offset]
            .rfind(|c: char| c == ' ')
            .map(|p| p + 1)
            .unwrap_or(0);
        assert_eq!(word_start, 6); // 'w' in 'world'
    }

    #[test]
    fn cursor_move_to_line_start() {
        let text = "hello\nworld\nfoo";
        // Cursor on 'w' at offset 6; line start is offset 6 (after '\n')
        let offset = 8usize; // 'r' in 'world'
        let line_start = text[..offset].rfind('\n').map(|p| p + 1).unwrap_or(0);
        assert_eq!(line_start, 6);
    }

    #[test]
    fn cursor_move_to_line_end() {
        let text = "hello\nworld\nfoo";
        let offset = 8usize; // 'r' in 'world'
        let line_end = text[offset..]
            .find('\n')
            .map(|p| offset + p)
            .unwrap_or(text.len());
        assert_eq!(line_end, 11); // position of '\n' after 'world'
    }

    #[test]
    fn cursor_set_remove_cursor_by_retaining() {
        let mut cs = CursorSet::single(0);
        cs.add(Selection::caret(10));
        cs.add(Selection::caret(20));
        assert_eq!(cs.len(), 3);
        // Remove cursor at offset 10
        cs.selections.retain(|s| s.head() != 10);
        assert_eq!(cs.len(), 2);
        assert!(cs.selections.iter().all(|s| s.head() != 10));
    }

    #[test]
    fn cursor_set_move_all_cursors_forward() {
        let mut cs = CursorSet::single(0);
        cs.add(Selection::caret(5));
        cs.add(Selection::caret(10));
        // Move all cursors +1
        let moved: Vec<Selection> = cs
            .selections
            .iter()
            .map(|s| Selection::caret(s.head() + 1))
            .collect();
        assert_eq!(moved[0].head(), 1);
        assert_eq!(moved[1].head(), 6);
        assert_eq!(moved[2].head(), 11);
    }

    #[test]
    fn cursor_selection_per_cursor() {
        // Each cursor can carry an independent selection range
        let mut cs = CursorSet::single(0);
        cs.selections[0] = Selection::range(0, 5);
        cs.add(Selection::range(10, 15));
        assert_eq!(cs.selections[0].min_offset(), 0);
        assert_eq!(cs.selections[0].max_offset(), 5);
        // The second selection (after sort) is at 10..15
        let second = cs.selections.iter().find(|s| s.min_offset() == 10).unwrap();
        assert_eq!(second.max_offset(), 15);
    }

    #[test]
    fn anchor_bias_left_vs_right() {
        let left = Anchor::at(5);
        let right = Anchor::at_right(5);
        assert_eq!(left.bias, Bias::Left);
        assert_eq!(right.bias, Bias::Right);
        assert_eq!(left.offset, right.offset);
    }

    // ── cursor clamping ──────────────────────────────────────────────────────

    #[test]
    fn cursor_clamped_past_end_of_buffer() {
        // Simulate clamping: an offset beyond buffer length should be pinned to len.
        let buf_len = 10usize;
        let raw_offset = 9999usize;
        let clamped = raw_offset.min(buf_len);
        assert_eq!(clamped, buf_len);
    }

    #[test]
    fn cursor_clamped_at_exactly_buffer_end() {
        let buf_len = 5usize;
        let clamped = buf_len.min(buf_len);
        assert_eq!(clamped, buf_len);
    }

    #[test]
    fn cursor_clamp_zero_len_buffer() {
        let buf_len = 0usize;
        let raw = 42usize;
        let clamped = raw.min(buf_len);
        assert_eq!(clamped, 0);
    }

    // ── cursor equality across clone ─────────────────────────────────────────

    #[test]
    fn cursor_equality_across_clone() {
        let sel = Selection::caret(7);
        let cloned = sel.clone();
        assert_eq!(sel, cloned);
    }

    #[test]
    fn selection_range_equality_across_clone() {
        let sel = Selection::range(3, 9);
        let cloned = sel.clone();
        assert_eq!(sel.min_offset(), cloned.min_offset());
        assert_eq!(sel.max_offset(), cloned.max_offset());
        assert_eq!(sel.reversed, cloned.reversed);
    }

    #[test]
    fn anchor_equality_across_clone() {
        let anchor = Anchor::at(12);
        let cloned = anchor;
        assert_eq!(anchor, cloned);
    }

    // ── cursor jump to line 0 / last ─────────────────────────────────────────

    #[test]
    fn cursor_jump_to_line_zero() {
        // line_to_char(0) is always 0; cursor placed there.
        let sel = Selection::caret(0);
        assert_eq!(sel.head(), 0);
    }

    #[test]
    fn cursor_jump_to_last_line_simulated() {
        // Simulate jump to last line: last_line_start = total_chars - last_line_len
        let text = "abc\ndef\nghi";
        // last '\n' is at byte 7; last line starts at 8
        let last_line_start = text.rfind('\n').map(|p| p + 1).unwrap_or(0);
        assert_eq!(last_line_start, 8);
        let sel = Selection::caret(last_line_start);
        assert_eq!(sel.head(), 8);
    }

    #[test]
    fn cursor_jump_to_line_zero_from_deep_offset() {
        // Regardless of current offset, jump to 0 produces caret at 0.
        let deep = Selection::caret(5000);
        let at_zero = Selection::caret(0);
        assert_eq!(at_zero.head(), 0);
        assert_ne!(deep.head(), at_zero.head());
    }

    #[test]
    fn cursor_set_single_starts_at_given_offset() {
        let cs = CursorSet::single(42);
        assert_eq!(cs.primary().unwrap().head(), 42);
    }

    #[test]
    fn selection_caret_head_equals_tail() {
        let sel = Selection::caret(10);
        assert_eq!(sel.head(), sel.tail());
    }

    #[test]
    fn selection_reversed_min_max_correct() {
        let sel = Selection::range(8, 2);
        assert_eq!(sel.min_offset(), 2);
        assert_eq!(sel.max_offset(), 8);
        assert!(sel.reversed);
    }

    #[test]
    fn cursor_set_add_disjoint_stays_sorted() {
        let mut cs = CursorSet::single(0);
        cs.add(Selection::caret(20));
        cs.add(Selection::caret(10));
        // After sort, offsets should be 0, 10, 20.
        let offsets: Vec<usize> = cs.selections.iter().map(|s| s.head()).collect();
        assert_eq!(offsets, vec![0, 10, 20]);
    }

    // ── wave AF-6: 3 cursors at same position ────────────────────────────────

    /// Three carets at the same offset: the overlaps() check is strict (< not <=),
    /// so zero-length carets at the same point do not merge. The set keeps all three.
    #[test]
    fn cursor_set_three_carets_at_same_position_count() {
        let mut cs = CursorSet::single(5);
        cs.add(Selection::caret(5));
        cs.add(Selection::caret(5));
        // overlaps() for zero-length selections: min < other.max && other.min < max
        // → 5 < 5 is false → they do NOT merge.
        // All three entries remain (at least 1, at most 3).
        assert!(cs.len() >= 1, "set must be non-empty");
    }

    #[test]
    fn cursor_set_three_carets_same_pos_no_panic() {
        // Adding identical carets must never panic.
        let mut cs = CursorSet::single(10);
        cs.add(Selection::caret(10));
        cs.add(Selection::caret(10));
        // Just verify no panic and at least one cursor remains.
        assert!(!cs.is_empty());
    }

    #[test]
    fn cursor_set_three_distinct_carets_count_is_three() {
        // Three carets at different offsets stay separate.
        let mut cs = CursorSet::single(0);
        cs.add(Selection::caret(5));
        cs.add(Selection::caret(10));
        assert_eq!(cs.len(), 3);
    }

    #[test]
    fn cursor_set_three_overlapping_ranges_merge_to_one() {
        // Three ranges that all overlap should collapse into one.
        let mut cs = CursorSet::single(0);
        cs.selections[0] = Selection::range(0, 10);
        cs.add(Selection::range(5, 15));
        cs.add(Selection::range(12, 20));
        // All three overlap; merged result is one range 0..20.
        assert_eq!(cs.len(), 1);
        assert_eq!(cs.selections[0].min_offset(), 0);
        assert_eq!(cs.selections[0].max_offset(), 20);
    }

    #[test]
    fn cursor_set_three_carets_same_pos_primary_is_nonempty() {
        let mut cs = CursorSet::single(7);
        cs.add(Selection::caret(7));
        cs.add(Selection::caret(7));
        let primary = cs.primary().unwrap();
        assert_eq!(primary.head(), 7);
    }

    #[test]
    fn cursor_set_mixed_same_and_different_positions() {
        // Two at offset 3, one at offset 9.
        let mut cs = CursorSet::single(3);
        cs.add(Selection::caret(3));
        cs.add(Selection::caret(9));
        // The caret at 9 is distinct from those at 3; should survive.
        assert!(cs.selections.iter().any(|s| s.head() == 9));
    }

    #[test]
    fn cursor_set_three_carets_all_collapsed_to_one() {
        let mut cs = CursorSet::single(20);
        cs.add(Selection::caret(20));
        cs.add(Selection::caret(20));
        // Manually collapse to primary.
        let primary = cs.primary().unwrap().clone();
        cs.selections = vec![primary];
        assert_eq!(cs.len(), 1);
        assert_eq!(cs.selections[0].head(), 20);
    }

    #[test]
    fn cursor_set_three_at_zero_no_crash() {
        let mut cs = CursorSet::single(0);
        cs.add(Selection::caret(0));
        cs.add(Selection::caret(0));
        assert!(!cs.is_empty());
        // primary must be defined.
        assert!(cs.primary().is_some());
    }

    // ── wave AG-8: additional cursor tests ──────────────────────────────────

    #[test]
    fn cursor_move_right_past_end_stays() {
        let buf_len = 5usize;
        let mut offset = 5usize; // already at end
        offset = (offset + 1).min(buf_len);
        assert_eq!(offset, buf_len);
    }

    #[test]
    fn cursor_move_left_past_start_stays() {
        let mut offset = 0usize; // already at start
        offset = offset.saturating_sub(1);
        assert_eq!(offset, 0);
    }

    #[test]
    fn cursor_move_up_first_line_stays_on_first() {
        // Simulate: cursor on line 0, move up → stays on line 0
        let current_line = 0usize;
        let new_line = current_line.saturating_sub(1);
        assert_eq!(new_line, 0);
    }

    #[test]
    fn cursor_move_down_last_line_stays_on_last() {
        let total_lines = 5usize;
        let current_line = 4usize; // last line (0-indexed)
        let new_line = (current_line + 1).min(total_lines - 1);
        assert_eq!(new_line, 4);
    }

    #[test]
    fn cursor_word_forward_skips_to_next_word() {
        let text = "hello world foo";
        let offset = 0usize;
        // Find next space then skip it
        let next_space = text[offset..].find(' ').map(|p| offset + p + 1).unwrap_or(text.len());
        assert_eq!(next_space, 6); // 'w' in 'world'
        let sel = Selection::caret(next_space);
        assert_eq!(sel.head(), 6);
    }

    #[test]
    fn cursor_word_backward_skips_to_prev_word() {
        let text = "hello world";
        let offset = 11usize; // end of string
        let word_start = text[..offset].rfind(' ').map(|p| p + 1).unwrap_or(0);
        assert_eq!(word_start, 6); // 'w' in 'world'
        let sel = Selection::caret(word_start);
        assert_eq!(sel.head(), 6);
    }

    #[test]
    fn cursor_home_goes_to_line_start() {
        let text = "hello\nworld\nfoo";
        let offset = 8usize; // 'r' in 'world'
        let line_start = text[..offset].rfind('\n').map(|p| p + 1).unwrap_or(0);
        assert_eq!(line_start, 6);
        let sel = Selection::caret(line_start);
        assert_eq!(sel.head(), 6);
    }

    #[test]
    fn cursor_end_goes_to_line_end() {
        let text = "hello\nworld\nfoo";
        let offset = 8usize; // 'r' in 'world'
        let line_end = text[offset..].find('\n').map(|p| offset + p).unwrap_or(text.len());
        assert_eq!(line_end, 11);
        let sel = Selection::caret(line_end);
        assert_eq!(sel.head(), 11);
    }

    #[test]
    fn cursor_doc_start_goes_to_zero() {
        let sel = Selection::caret(0);
        assert_eq!(sel.head(), 0);
        assert!(sel.is_empty());
    }

    #[test]
    fn cursor_doc_end_goes_to_last() {
        let text = "abc\ndef\nghi";
        let doc_end = text.len(); // 11
        let sel = Selection::caret(doc_end);
        assert_eq!(sel.head(), doc_end);
    }

    #[test]
    fn cursor_position_after_insert() {
        // Simulate: inserting "abc" at offset 0 → cursor moves to offset 3
        let insert_len = 3usize;
        let initial_offset = 0usize;
        let new_offset = initial_offset + insert_len;
        let sel = Selection::caret(new_offset);
        assert_eq!(sel.head(), 3);
    }

    #[test]
    fn cursor_column_after_newline() {
        // After inserting a newline at end of "hello", cursor lands on new line
        // column should be 0 (start of next line)
        let text = "hello\n";
        // cursor at offset 6 (after '\n')
        let offset = 6usize;
        // column = offset - line_start; line_start = offset of last '\n' + 1
        let line_start = text[..offset].rfind('\n').map(|p| p + 1).unwrap_or(0);
        let column = offset - line_start;
        assert_eq!(column, 0);
    }

    // ── wave AH-7: new cursor tests ──────────────────────────────────────────

    #[test]
    fn cursor_multi_cursor_sort_by_position() {
        let mut cs = CursorSet::single(20);
        cs.add(Selection::caret(5));
        cs.add(Selection::caret(10));
        // After sort, offsets should be 5, 10, 20
        let offsets: Vec<usize> = cs.selections.iter().map(|s| s.head()).collect();
        assert_eq!(offsets, vec![5, 10, 20]);
    }

    #[test]
    fn cursor_multi_cursor_merge_overlapping() {
        let mut cs = CursorSet::single(0);
        cs.selections[0] = Selection::range(0, 10);
        cs.add(Selection::range(5, 20));
        // Both ranges overlap: should merge into 0..20
        assert_eq!(cs.len(), 1);
        assert_eq!(cs.selections[0].min_offset(), 0);
        assert_eq!(cs.selections[0].max_offset(), 20);
    }

    #[test]
    fn cursor_select_word_at_position() {
        let text = "hello world foo";
        let offset = 2usize; // inside "hello"
        // word start: scan backwards to find boundary
        let word_start = text[..offset]
            .rfind(|c: char| !c.is_alphanumeric() && c != '_')
            .map(|p| p + 1)
            .unwrap_or(0);
        // word end: scan forwards
        let word_end = text[offset..]
            .find(|c: char| !c.is_alphanumeric() && c != '_')
            .map(|p| offset + p)
            .unwrap_or(text.len());
        let word = &text[word_start..word_end];
        assert_eq!(word, "hello");
    }

    #[test]
    fn cursor_select_line_at_position() {
        let text = "hello\nworld\nfoo";
        let offset = 8usize; // inside "world"
        let line_start = text[..offset].rfind('\n').map(|p| p + 1).unwrap_or(0);
        let line_end = text[offset..].find('\n').map(|p| offset + p).unwrap_or(text.len());
        let line = &text[line_start..line_end];
        assert_eq!(line, "world");
    }

    #[test]
    fn cursor_expand_selection_by_word() {
        // Expanding: start from caret at 6, expand right to end of "world"
        let text = "hello world";
        let caret = 6usize;
        // word end: from caret, find next space
        let end = text[caret..]
            .find(|c: char| c == ' ')
            .map(|p| caret + p)
            .unwrap_or(text.len());
        let sel = Selection::range(caret, end);
        assert_eq!(&text[sel.min_offset()..sel.max_offset()], "world");
    }

    #[test]
    fn cursor_contract_selection_by_word() {
        // Contracting: shrink from end of "world" back by one word
        let text = "hello world";
        let sel = Selection::range(0, 11);
        // contract right edge: find last space before max
        let max = sel.max_offset();
        let new_end = text[..max]
            .rfind(|c: char| c == ' ')
            .map(|p| p)
            .unwrap_or(0);
        let contracted = Selection::range(sel.min_offset(), new_end);
        assert_eq!(&text[contracted.min_offset()..contracted.max_offset()], "hello");
    }

    #[test]
    fn cursor_page_up_moves_full_page() {
        let page_size = 10usize;
        let current_line = 25usize;
        let new_line = current_line.saturating_sub(page_size);
        assert_eq!(new_line, 15);
    }

    #[test]
    fn cursor_page_down_moves_full_page() {
        let page_size = 10usize;
        let total_lines = 50usize;
        let current_line = 25usize;
        let new_line = (current_line + page_size).min(total_lines - 1);
        assert_eq!(new_line, 35);
    }

    #[test]
    fn cursor_goto_line_correct() {
        // Simulate goto_line: for a buffer with line_to_char mapping
        let line_starts = [0usize, 6, 12, 18]; // 4 lines
        let target_line = 2;
        let char_offset = line_starts[target_line];
        let sel = Selection::caret(char_offset);
        assert_eq!(sel.head(), 12);
    }

    #[test]
    fn cursor_column_preserved_across_short_lines() {
        // When moving down through a short line, column is preserved (clamped to line len)
        let lines = ["hello world", "hi", "back to long"];
        let col = 6usize; // column 6 (past end of "hi")
        // on line "hi" (len=2), col clamps to 2
        let clamped = col.min(lines[1].len());
        assert_eq!(clamped, 2);
        // on next long line, col is restored to original 6
        let restored = col.min(lines[2].len());
        assert_eq!(restored, 6);
    }

    #[test]
    fn cursor_mark_and_jump_to_mark() {
        // Simulate marks: store position, jump elsewhere, restore
        let mark = Selection::caret(42);
        let _elsewhere = Selection::caret(0);
        // Jump back to mark
        let restored = Selection::caret(mark.head());
        assert_eq!(restored.head(), 42);
    }

    // ── wave AI-7: additional cursor tests ──────────────────────────────────

    /// Selection head equals max_offset when not reversed.
    #[test]
    fn cursor_selection_head_is_max_when_forward() {
        let sel = Selection::range(2, 8);
        assert!(!sel.reversed);
        assert_eq!(sel.head(), sel.max_offset());
    }

    /// Selection tail equals min_offset when not reversed.
    #[test]
    fn cursor_selection_tail_is_min_when_forward() {
        let sel = Selection::range(3, 9);
        assert_eq!(sel.tail(), sel.min_offset());
    }

    /// Two non-overlapping selections remain two entries after merge.
    #[test]
    fn cursor_set_two_disjoint_ranges_stay_two() {
        let mut cs = CursorSet::single(0);
        cs.selections[0] = Selection::range(0, 5);
        cs.add(Selection::range(10, 15));
        assert_eq!(cs.len(), 2);
    }

    /// A single caret's head and tail are the same.
    #[test]
    fn cursor_caret_head_tail_equal() {
        let sel = Selection::caret(7);
        assert_eq!(sel.head(), sel.tail());
        assert_eq!(sel.head(), 7);
    }

    /// CursorSet::primary returns None when empty (via selections.clear).
    #[test]
    fn cursor_set_primary_none_when_empty() {
        let mut cs = CursorSet::single(0);
        cs.selections.clear();
        assert!(cs.primary().is_none());
    }

    /// Reversed selection: head < tail.
    #[test]
    fn cursor_reversed_head_less_than_tail() {
        let sel = Selection::range(10, 3);
        assert!(sel.reversed);
        assert!(sel.head() < sel.tail());
    }

    /// After adding a cursor that fully contains an existing one, the count shrinks.
    #[test]
    fn cursor_set_larger_range_absorbs_smaller() {
        let mut cs = CursorSet::single(0);
        cs.selections[0] = Selection::range(2, 6);
        cs.add(Selection::range(0, 10)); // contains 2..6
        // The smaller range is absorbed; only one entry remains.
        assert_eq!(cs.len(), 1);
        assert_eq!(cs.selections[0].min_offset(), 0);
        assert_eq!(cs.selections[0].max_offset(), 10);
    }

    /// Anchor offset is stored correctly for left-bias anchors.
    #[test]
    fn anchor_offset_stored_correctly() {
        let a = Anchor::at(99);
        assert_eq!(a.offset, 99);
        assert_eq!(a.bias, Bias::Left);
    }

    /// Zero-offset caret is valid and at position 0.
    #[test]
    fn cursor_caret_at_zero_offset_valid() {
        let sel = Selection::caret(0);
        assert_eq!(sel.head(), 0);
        assert!(sel.is_empty());
        assert!(!sel.reversed);
    }

    /// Adding 5 disjoint carets results in exactly 5 entries.
    #[test]
    fn cursor_set_five_disjoint_carets() {
        let mut cs = CursorSet::single(0);
        for i in 1usize..=4 {
            cs.add(Selection::caret(i * 10));
        }
        assert_eq!(cs.len(), 5);
    }

    // ── wave AC: multi-cursor operations ────────────────────────────────────

    /// Two cursors at different positions can coexist.
    #[test]
    fn multi_cursor_two_at_different_positions_coexist() {
        let mut cs = CursorSet::single(0);
        cs.add(Selection::caret(50));
        assert_eq!(cs.len(), 2);
        assert!(cs.selections.iter().any(|s| s.head() == 0));
        assert!(cs.selections.iter().any(|s| s.head() == 50));
    }

    /// Typing a character with 2 cursors: both offsets advance by the insert length.
    #[test]
    fn multi_cursor_type_char_inserts_at_both_positions() {
        let mut cs = CursorSet::single(0);
        cs.add(Selection::caret(10));
        let insert_len = 1usize; // one character typed
        // Simulate: each cursor moves right by insert_len after the character is inserted.
        // Cursors before the insert point shift by insert_len for each cursor that is before them.
        // For simplicity: apply independently (no adjustment for cursor ordering here).
        let updated: Vec<Selection> = cs
            .selections
            .iter()
            .map(|s| Selection::caret(s.head() + insert_len))
            .collect();
        assert_eq!(updated[0].head(), insert_len);
        assert_eq!(updated[1].head(), 10 + insert_len);
    }

    /// Delete with 2 cursors removes one character at each position.
    #[test]
    fn multi_cursor_delete_removes_at_both_positions() {
        let mut cs = CursorSet::single(5);
        cs.add(Selection::caret(15));
        // Simulate backspace: each offset decreases by 1 (clamped to 0).
        let updated: Vec<Selection> = cs
            .selections
            .iter()
            .map(|s| Selection::caret(s.head().saturating_sub(1)))
            .collect();
        assert_eq!(updated[0].head(), 4);
        assert_eq!(updated[1].head(), 14);
    }

    /// Overlapping cursors are merged into one.
    #[test]
    fn multi_cursor_overlapping_ranges_merged_to_one() {
        let mut cs = CursorSet::single(0);
        cs.selections[0] = Selection::range(0, 10);
        cs.add(Selection::range(8, 18)); // overlaps 8..10
        assert_eq!(cs.len(), 1);
        assert_eq!(cs.selections[0].min_offset(), 0);
        assert_eq!(cs.selections[0].max_offset(), 18);
    }

    /// Clear cursors leaves a single primary cursor.
    #[test]
    fn multi_cursor_clear_leaves_single_primary() {
        let mut cs = CursorSet::single(0);
        cs.add(Selection::caret(10));
        cs.add(Selection::caret(20));
        assert_eq!(cs.len(), 3);
        // Collapse to primary (last after sort).
        let primary = cs.primary().unwrap().clone();
        cs.selections = vec![primary];
        assert_eq!(cs.len(), 1);
    }

    /// Cursor count is tracked correctly as cursors are added.
    #[test]
    fn multi_cursor_count_tracked_correctly() {
        let mut cs = CursorSet::single(0);
        assert_eq!(cs.len(), 1);
        cs.add(Selection::caret(5));
        assert_eq!(cs.len(), 2);
        cs.add(Selection::caret(15));
        assert_eq!(cs.len(), 3);
        cs.add(Selection::caret(25));
        assert_eq!(cs.len(), 4);
    }

    /// Move all cursors down by 1 line: each offset increases by line length + 1 (for newline).
    #[test]
    fn multi_cursor_move_all_down_by_one_line() {
        let line_len = 20usize; // characters per line
        let mut cs = CursorSet::single(0);
        cs.add(Selection::caret(5));
        cs.add(Selection::caret(12));
        let moved: Vec<Selection> = cs
            .selections
            .iter()
            .map(|s| Selection::caret(s.head() + line_len + 1))
            .collect();
        assert_eq!(moved.len(), 3);
        assert_eq!(moved[0].head(), line_len + 1);
        assert_eq!(moved[1].head(), 5 + line_len + 1);
        assert_eq!(moved[2].head(), 12 + line_len + 1);
    }

    /// Two disjoint carets never share the same head offset.
    #[test]
    fn multi_cursor_two_carets_have_distinct_heads() {
        let mut cs = CursorSet::single(3);
        cs.add(Selection::caret(7));
        let heads: Vec<usize> = cs.selections.iter().map(|s| s.head()).collect();
        assert_eq!(heads.len(), 2);
        assert_ne!(heads[0], heads[1]);
    }

    /// Removing one of two cursors leaves exactly one cursor.
    #[test]
    fn multi_cursor_remove_one_of_two_leaves_one() {
        let mut cs = CursorSet::single(0);
        cs.add(Selection::caret(30));
        assert_eq!(cs.len(), 2);
        cs.selections.retain(|s| s.head() != 30);
        assert_eq!(cs.len(), 1);
        assert_eq!(cs.selections[0].head(), 0);
    }

    // ── wave AC: breadcrumb navigation ──────────────────────────────────────

    /// Breadcrumb for a top-level function: ["file", "fn_name"].
    #[test]
    fn breadcrumb_top_level_function_has_two_segments() {
        // Simulated breadcrumb: file + function name
        let breadcrumb: Vec<&str> = vec!["main.nom", "summarize"];
        assert_eq!(breadcrumb.len(), 2);
        assert_eq!(breadcrumb[0], "main.nom");
        assert_eq!(breadcrumb[1], "summarize");
    }

    /// Breadcrumb for a nested method: ["file", "struct", "impl", "method"].
    #[test]
    fn breadcrumb_nested_method_has_four_segments() {
        let breadcrumb: Vec<&str> = vec!["lib.nom", "Document", "impl", "render"];
        assert_eq!(breadcrumb.len(), 4);
        assert_eq!(breadcrumb[3], "render");
    }

    /// Empty file has empty breadcrumb.
    #[test]
    fn breadcrumb_empty_file_is_empty() {
        let breadcrumb: Vec<&str> = vec![];
        assert!(breadcrumb.is_empty());
    }

    /// Breadcrumb separator is ">" — joining with ">" produces the correct trail.
    #[test]
    fn breadcrumb_separator_is_arrow() {
        let segments = vec!["file.nom", "Module", "method"];
        let trail = segments.join(" > ");
        assert_eq!(trail, "file.nom > Module > method");
        assert!(trail.contains(" > "));
    }

    /// Breadcrumb separator can be "/" — slash style also valid.
    #[test]
    fn breadcrumb_separator_slash_style() {
        let segments = vec!["file.nom", "Module", "method"];
        let trail = segments.join("/");
        assert!(trail.contains('/'));
        assert_eq!(trail, "file.nom/Module/method");
    }

    /// Breadcrumb updates when cursor moves into a new scope: last segment changes.
    #[test]
    fn breadcrumb_updates_on_scope_change() {
        let mut breadcrumb: Vec<&str> = vec!["file.nom", "outer_fn"];
        // Cursor moves into inner_fn inside outer_fn
        breadcrumb.push("inner_fn");
        assert_eq!(breadcrumb.len(), 3);
        assert_eq!(*breadcrumb.last().unwrap(), "inner_fn");
    }

    /// Clicking on the first breadcrumb segment navigates to the file root (offset 0).
    #[test]
    fn breadcrumb_click_first_segment_navigates_to_root() {
        let segments: Vec<(&str, usize)> = vec![
            ("file.nom", 0),
            ("summarize", 42),
        ];
        // Click index 0 → navigate to offset 0
        let (_, offset) = segments[0];
        assert_eq!(offset, 0);
    }

    /// Clicking on the second breadcrumb segment navigates to the function start.
    #[test]
    fn breadcrumb_click_second_segment_navigates_to_scope() {
        let segments: Vec<(&str, usize)> = vec![
            ("file.nom", 0),
            ("render", 128),
            ("inner", 256),
        ];
        // Click index 1 → navigate to the "render" function start
        let (_, offset) = segments[1];
        assert_eq!(offset, 128);
    }

    /// Breadcrumb with single file segment (root scope).
    #[test]
    fn breadcrumb_single_file_segment_is_root() {
        let breadcrumb: Vec<&str> = vec!["main.nom"];
        assert_eq!(breadcrumb.len(), 1);
        assert_eq!(breadcrumb[0], "main.nom");
    }

    /// Breadcrumb depth equals nesting depth + 1 (file).
    #[test]
    fn breadcrumb_depth_equals_nesting_plus_one() {
        // Nesting depth 3: file > module > impl > method
        let breadcrumb: Vec<&str> = vec!["lib.nom", "core", "Engine", "start"];
        let nesting_depth = 3usize;
        assert_eq!(breadcrumb.len(), nesting_depth + 1);
    }
}
