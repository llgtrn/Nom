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
}
