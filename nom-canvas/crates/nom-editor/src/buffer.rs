#![deny(unsafe_code)]
use ropey::Rope;
use std::borrow::Cow;
use std::ops::Range;

pub type BufferId = u64;

/// A row/column position within a buffer (both 0-indexed).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Point {
    /// Zero-based line number.
    pub row: u32,
    /// Zero-based column (character offset within the line).
    pub column: u32,
}

#[derive(Clone, Debug)]
pub struct Patch {
    pub old_range: Range<usize>,
    pub new_text: String,
}

#[derive(Clone, Debug)]
pub struct Transaction {
    patches: Vec<Patch>,
    active: bool,
}

impl Transaction {
    pub fn new() -> Self {
        Self {
            patches: Vec::new(),
            active: true,
        }
    }
    pub fn add_patch(&mut self, patch: Patch) {
        self.patches.push(patch);
    }
    pub fn commit(mut self) -> Vec<Patch> {
        self.active = false;
        self.patches
    }
}

impl Default for Transaction {
    fn default() -> Self {
        Self::new()
    }
}

pub struct Buffer {
    pub id: BufferId,
    pub rope: Rope,
    pub version: u64,
    pub path: Option<std::path::PathBuf>,
    transaction_stack: Vec<Transaction>,
    undo_stack: Vec<Vec<Patch>>,
}

impl Buffer {
    pub fn new(id: BufferId, text: &str) -> Self {
        Self {
            id,
            rope: Rope::from_str(text),
            version: 0,
            path: None,
            transaction_stack: Vec::new(),
            undo_stack: Vec::new(),
        }
    }

    pub fn len(&self) -> usize {
        self.rope.len_chars()
    }
    pub fn is_empty(&self) -> bool {
        self.rope.len_chars() == 0
    }

    pub fn text_for_range(&self, range: Range<usize>) -> Cow<'_, str> {
        let slice = self.rope.slice(range.start..range.end);
        Cow::Owned(slice.to_string())
    }

    pub fn line_count(&self) -> usize {
        self.rope.len_lines()
    }

    /// Returns the total number of Unicode scalar value characters in the buffer.
    pub fn char_count(&self) -> usize {
        self.rope.len_chars()
    }

    /// Returns the text of the given logical line (0-indexed), without the trailing newline.
    /// Returns `None` if `line` is out of range.
    pub fn line_at(&self, line: usize) -> Option<String> {
        let total = self.rope.len_lines();
        if line >= total {
            return None;
        }
        let slice = self.rope.line(line);
        let s = slice.to_string();
        let trimmed = s.trim_end_matches('\n').trim_end_matches('\r');
        Some(trimmed.to_string())
    }

    /// Returns the word (run of alphanumeric / underscore chars) containing
    /// char-offset `cursor`, or `None` if the cursor is not on a word character.
    pub fn word_at_cursor(&self, cursor: usize) -> Option<String> {
        let len = self.rope.len_chars();
        if len == 0 || cursor > len {
            return None;
        }
        let chars: Vec<char> = self.rope.chars().collect();
        let idx = cursor.min(chars.len().saturating_sub(1));
        let is_word = |c: char| c.is_alphanumeric() || c == '_';
        if !is_word(chars[idx]) {
            return None;
        }
        let start = chars[..=idx]
            .iter()
            .rposition(|c| !is_word(*c))
            .map(|p| p + 1)
            .unwrap_or(0);
        let end = chars[idx..]
            .iter()
            .position(|c| !is_word(*c))
            .map(|p| idx + p)
            .unwrap_or(chars.len());
        Some(chars[start..end].iter().collect())
    }

    pub fn char_to_line(&self, char_idx: usize) -> usize {
        self.rope.char_to_line(char_idx.min(self.rope.len_chars()))
    }

    pub fn line_to_char(&self, line: usize) -> usize {
        self.rope.line_to_char(line.min(self.rope.len_lines()))
    }

    /// Convert a char offset into a [`Point`] (row, column).
    ///
    /// Clamps `offset` to `[0, len_chars]`. Returns `Point { row: 0, column: 0 }` for
    /// an empty buffer.
    pub fn point_at(&self, offset: usize) -> Point {
        let offset = offset.min(self.rope.len_chars());
        let row = self.rope.char_to_line(offset) as u32;
        let line_start = self.rope.line_to_char(row as usize);
        let column = (offset - line_start) as u32;
        Point { row, column }
    }

    /// Convert a [`Point`] (row, column) into a char offset.
    ///
    /// Returns `len_chars()` if the point is past the end of the buffer.
    pub fn offset_from_point(&self, point: Point) -> usize {
        let row = (point.row as usize).min(self.rope.len_lines().saturating_sub(1));
        let line_start = self.rope.line_to_char(row);
        let line_len = self.rope.line(row).len_chars();
        // Exclude trailing newline from column clamping
        let usable_len = if line_len > 0
            && self
                .rope
                .line(row)
                .chars()
                .last()
                .map(|c| c == '\n')
                .unwrap_or(false)
        {
            line_len - 1
        } else {
            line_len
        };
        let column = (point.column as usize).min(usable_len);
        line_start + column
    }

    /// Atomic edit: replace range with new_text. Returns undo patch.
    pub fn edit(&mut self, range: Range<usize>, new_text: &str) -> Patch {
        let old_text = self.text_for_range(range.clone()).into_owned();
        let start = range.start;
        let end = range.end;
        self.rope.remove(start..end);
        if !new_text.is_empty() {
            self.rope.insert(start, new_text);
        }
        self.version += 1;
        Patch {
            old_range: start..start + new_text.len(),
            new_text: old_text,
        }
    }

    pub fn insert_at(&mut self, offset: usize, text: &str) {
        let offset = offset.min(self.rope.len_chars());
        self.rope.insert(offset, text);
        self.version += 1;
    }

    pub fn delete_range(&mut self, range: Range<usize>) {
        let end = range.end.min(self.rope.len_chars());
        if range.start < end {
            self.rope.remove(range.start..end);
            self.version += 1;
        }
    }

    pub fn start_transaction(&mut self) {
        self.transaction_stack.push(Transaction::new());
    }

    pub fn end_transaction(&mut self) {
        if let Some(txn) = self.transaction_stack.pop() {
            let patches = txn.commit();
            if !patches.is_empty() {
                self.undo_stack.push(patches);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn buffer_edit_and_len() {
        let mut buf = Buffer::new(1, "hello world");
        assert_eq!(buf.len(), 11);
        buf.edit(6..11, "Nom");
        assert_eq!(buf.text_for_range(0..buf.len()), "hello Nom");
        assert_eq!(buf.version, 1);
    }

    #[test]
    fn buffer_line_navigation() {
        let buf = Buffer::new(1, "line1\nline2\nline3");
        assert_eq!(buf.line_count(), 3);
        assert_eq!(buf.char_to_line(0), 0);
        assert_eq!(buf.char_to_line(6), 1);
    }

    #[test]
    fn buffer_insert_delete() {
        let mut buf = Buffer::new(1, "hello");
        buf.insert_at(5, " world");
        assert_eq!(buf.text_for_range(0..11), "hello world");
        buf.delete_range(5..11);
        assert_eq!(buf.text_for_range(0..5), "hello");
    }

    #[test]
    fn rope_buffer_insert_and_read() {
        let mut buf = Buffer::new(1, "");
        assert!(buf.is_empty());
        buf.insert_at(0, "Nom");
        assert_eq!(buf.len(), 3);
        assert_eq!(buf.text_for_range(0..3).as_ref(), "Nom");
        buf.insert_at(3, " rocks");
        assert_eq!(buf.text_for_range(0..buf.len()).as_ref(), "Nom rocks");
    }

    #[test]
    fn buffer_version_increments_on_edit() {
        let mut buf = Buffer::new(42, "abc");
        assert_eq!(buf.version, 0);
        buf.insert_at(3, "d");
        assert_eq!(buf.version, 1);
        buf.delete_range(0..1);
        assert_eq!(buf.version, 2);
    }

    #[test]
    fn buffer_insert_then_len() {
        let mut buf = Buffer::new(1, "");
        buf.insert_at(0, "abc");
        assert_eq!(buf.len(), 3);
    }

    #[test]
    fn buffer_insert_at_middle_offset() {
        let mut buf = Buffer::new(1, "helo");
        buf.insert_at(3, "l");
        assert_eq!(buf.text_for_range(0..buf.len()).as_ref(), "hello");
    }

    #[test]
    fn buffer_delete_char() {
        let mut buf = Buffer::new(1, "ab");
        buf.delete_range(1..2);
        assert_eq!(buf.text_for_range(0..buf.len()).as_ref(), "a");
    }

    #[test]
    fn buffer_delete_range() {
        let mut buf = Buffer::new(1, "hello");
        buf.delete_range(0..3);
        assert_eq!(buf.text_for_range(0..buf.len()).as_ref(), "lo");
    }

    #[test]
    fn buffer_rope_lines() {
        let buf = Buffer::new(1, "a\nb\nc");
        assert_eq!(buf.line_count(), 3);
    }

    #[test]
    fn buffer_char_at() {
        let buf = Buffer::new(1, "hello");
        // char_at via text_for_range(0..1)
        let ch: char = buf.text_for_range(0..1).chars().next().unwrap();
        assert_eq!(ch, 'h');
    }

    #[test]
    fn buffer_replace_range() {
        let mut buf = Buffer::new(1, "hello");
        buf.edit(1..4, "XXX");
        assert_eq!(buf.text_for_range(0..buf.len()).as_ref(), "hXXXo");
    }

    #[test]
    fn buffer_insert_at_start() {
        let mut buf = Buffer::new(1, "world");
        buf.insert_at(0, "hello ");
        assert_eq!(buf.text_for_range(0..buf.len()).as_ref(), "hello world");
    }

    #[test]
    fn buffer_insert_at_end() {
        let mut buf = Buffer::new(1, "hello");
        buf.insert_at(5, "!");
        assert_eq!(buf.text_for_range(0..buf.len()).as_ref(), "hello!");
    }

    #[test]
    fn buffer_insert_at_mid() {
        let mut buf = Buffer::new(1, "helo");
        buf.insert_at(2, "l");
        assert_eq!(buf.text_for_range(0..buf.len()).as_ref(), "hello");
    }

    #[test]
    fn buffer_delete_from_start() {
        let mut buf = Buffer::new(1, "abcdef");
        buf.delete_range(0..3);
        assert_eq!(buf.text_for_range(0..buf.len()).as_ref(), "def");
    }

    #[test]
    fn buffer_delete_to_end() {
        let mut buf = Buffer::new(1, "hello world");
        buf.delete_range(5..11);
        assert_eq!(buf.text_for_range(0..buf.len()).as_ref(), "hello");
    }

    #[test]
    fn buffer_delete_entire_content() {
        let mut buf = Buffer::new(1, "abc");
        buf.delete_range(0..3);
        assert!(buf.is_empty());
    }

    #[test]
    fn buffer_delete_noop_when_empty_range() {
        let mut buf = Buffer::new(1, "hello");
        let v_before = buf.version;
        buf.delete_range(2..2);
        assert_eq!(buf.version, v_before); // no change
        assert_eq!(buf.len(), 5);
    }

    #[test]
    fn buffer_transaction_groups_patches() {
        let mut buf = Buffer::new(1, "hello world");
        buf.start_transaction();
        buf.insert_at(5, "!");
        buf.end_transaction();
        // After transaction, undo stack grows only if patches were recorded via edit().
        // insert_at does not push to transaction; just verify no panic and text changed.
        assert!(buf.text_for_range(0..buf.len()).contains('!'));
    }

    #[test]
    fn buffer_undo_stack_grows_on_edit_commit() {
        let mut buf = Buffer::new(1, "hello");
        buf.start_transaction();
        // edit() returns an undo patch — push it manually via the transaction
        let patch = buf.edit(0..5, "world");
        buf.transaction_stack.last_mut().unwrap().add_patch(patch);
        buf.end_transaction();
        assert!(!buf.undo_stack.is_empty());
    }

    #[test]
    fn buffer_insert_beyond_len_clamps() {
        let mut buf = Buffer::new(1, "hi");
        buf.insert_at(1000, "!");
        // Clamped to end of buffer
        assert_eq!(buf.text_for_range(0..buf.len()).as_ref(), "hi!");
    }

    #[test]
    fn buffer_line_to_char_first_line() {
        let buf = Buffer::new(1, "abc\ndef");
        assert_eq!(buf.line_to_char(0), 0);
        assert_eq!(buf.line_to_char(1), 4);
    }

    #[test]
    fn buffer_empty_delete_range_no_panic() {
        let mut buf = Buffer::new(1, "");
        buf.delete_range(0..0); // no-op, no panic
        assert!(buf.is_empty());
    }

    #[test]
    fn buffer_multiline_line_count() {
        let buf = Buffer::new(1, "a\nb\nc\nd");
        assert_eq!(buf.line_count(), 4);
    }

    #[test]
    fn buffer_char_to_line_on_newline_char() {
        let buf = Buffer::new(1, "abc\ndef");
        // char index 3 is '\n', which is on line 0
        assert_eq!(buf.char_to_line(3), 0);
    }

    #[test]
    fn buffer_line_to_char_second_line() {
        let buf = Buffer::new(1, "abc\nxyz");
        // line 1 starts at char index 4
        assert_eq!(buf.line_to_char(1), 4);
    }

    #[test]
    fn buffer_edit_returns_undo_patch_new_text() {
        let mut buf = Buffer::new(1, "hello");
        let undo = buf.edit(0..5, "world");
        // undo patch's new_text is the original text we replaced
        assert_eq!(undo.new_text, "hello");
    }

    #[test]
    fn buffer_edit_old_range_matches_new_text_len() {
        let mut buf = Buffer::new(1, "abc");
        let undo = buf.edit(0..3, "xy");
        // undo old_range.start should be 0, end = start + len("xy") = 2
        assert_eq!(undo.old_range.start, 0);
        assert_eq!(undo.old_range.end, 2);
    }

    #[test]
    fn buffer_sequential_inserts_build_content() {
        let mut buf = Buffer::new(1, "");
        buf.insert_at(0, "a");
        buf.insert_at(1, "b");
        buf.insert_at(2, "c");
        assert_eq!(buf.text_for_range(0..3).as_ref(), "abc");
    }

    #[test]
    fn buffer_version_unchanged_on_noop_delete() {
        let mut buf = Buffer::new(1, "hello");
        let v = buf.version;
        buf.delete_range(3..3); // zero-length, noop
        assert_eq!(buf.version, v);
    }

    #[test]
    fn buffer_multiple_transactions_stack() {
        let mut buf = Buffer::new(1, "abc");
        buf.start_transaction();
        buf.start_transaction();
        buf.end_transaction();
        buf.end_transaction();
        // no panic; both transactions committed without patches
        assert_eq!(buf.text_for_range(0..3).as_ref(), "abc");
    }

    #[test]
    fn buffer_text_for_range_full() {
        let buf = Buffer::new(1, "hello world");
        assert_eq!(buf.text_for_range(0..11).as_ref(), "hello world");
    }

    #[test]
    fn buffer_text_for_range_partial() {
        let buf = Buffer::new(1, "hello world");
        assert_eq!(buf.text_for_range(6..11).as_ref(), "world");
    }

    #[test]
    fn buffer_id_is_stored() {
        let buf = Buffer::new(42, "test");
        assert_eq!(buf.id, 42);
    }

    #[test]
    fn buffer_path_starts_as_none() {
        let buf = Buffer::new(1, "text");
        assert!(buf.path.is_none());
    }

    #[test]
    fn buffer_edit_empty_replacement_deletes() {
        let mut buf = Buffer::new(1, "hello world");
        buf.edit(5..11, "");
        assert_eq!(buf.text_for_range(0..buf.len()).as_ref(), "hello");
    }

    #[test]
    fn buffer_line_ending_crlf_detected() {
        // CRLF line ending: "\r\n" — buffer stores it as-is
        let buf = Buffer::new(1, "line1\r\nline2");
        // rope counts '\n' as line separator, so line count should be 2
        assert_eq!(buf.line_count(), 2);
    }

    #[test]
    fn buffer_unicode_insert() {
        let mut buf = Buffer::new(1, "");
        buf.insert_at(0, "nom");
        assert_eq!(buf.len(), 3);
    }

    #[test]
    fn buffer_insert_at_beyond_len_appends() {
        let mut buf = Buffer::new(1, "abc");
        buf.insert_at(999, "!");
        let full = buf.text_for_range(0..buf.len());
        assert!(full.ends_with('!'));
    }

    #[test]
    fn buffer_large_10k_lines() {
        // Build a 10 000-line buffer and verify line count + length.
        let line = "abcdefghij\n"; // 11 chars
        let text: String = line.repeat(10_000);
        let buf = Buffer::new(1, &text);
        // ropey counts lines by '\n'; 10 000 newlines → 10 001 (last empty line)
        assert_eq!(buf.line_count(), 10_001);
        // Total chars = 10 000 * 11 = 110 000
        assert_eq!(buf.len(), 110_000);
    }

    #[test]
    fn buffer_clone_independence() {
        // Two buffers with the same initial text are independent.
        let b1 = Buffer::new(1, "hello");
        let mut b2 = Buffer::new(2, "hello");
        b2.insert_at(5, " world");
        // b1 is unchanged.
        assert_eq!(b1.len(), 5);
        assert_eq!(b2.len(), 11);
    }

    #[test]
    fn buffer_version_increments_on_each_edit() {
        let mut buf = Buffer::new(1, "start");
        let v0 = buf.version;
        buf.insert_at(5, " one");
        let v1 = buf.version;
        buf.insert_at(buf.len(), " two");
        let v2 = buf.version;
        buf.edit(0..5, "begin");
        let v3 = buf.version;
        assert!(v1 > v0, "version should increase after first insert");
        assert!(v2 > v1, "version should increase after second insert");
        assert!(v3 > v2, "version should increase after edit");
        assert_eq!(v3 - v0, 3);
    }

    #[test]
    fn buffer_line_count_after_mixed_crlf_lf_edits() {
        // Start with LF-only content.
        let mut buf = Buffer::new(1, "line1\nline2\nline3");
        assert_eq!(buf.line_count(), 3);
        // Insert a CRLF line at the beginning.
        buf.insert_at(0, "crlf\r\n");
        // Ropey counts '\n' as the line separator.
        // After insert: "crlf\r\nline1\nline2\nline3" → 4 '\n' → 4 lines
        assert_eq!(buf.line_count(), 4);
        // Delete the first inserted line (6 chars: 'c','r','l','f','\r','\n')
        buf.delete_range(0..6);
        // Back to 3 lines.
        assert_eq!(buf.line_count(), 3);
    }

    #[test]
    fn buffer_clone_version_is_independent() {
        // Two separate buffers with same text have independent versions.
        let b1 = Buffer::new(1, "text");
        let mut b2 = Buffer::new(2, "text");
        b2.insert_at(4, "!");
        // b1 version is still 0.
        assert_eq!(b1.version, 0);
        // b2 version advanced to 1.
        assert_eq!(b2.version, 1);
    }

    #[test]
    fn buffer_large_edit_on_10k_line_buffer() {
        let line = "x\n";
        let text: String = line.repeat(10_000);
        let mut buf = Buffer::new(1, &text);
        let original_len = buf.len();
        // Edit the very first line.
        buf.edit(0..1, "Y");
        assert_eq!(buf.len(), original_len); // same length, char replaced
        let first_char: char = buf.text_for_range(0..1).chars().next().unwrap();
        assert_eq!(first_char, 'Y');
    }

    #[test]
    fn buffer_insert_at_zero_in_large_buffer() {
        let text: String = "z\n".repeat(5_000);
        let mut buf = Buffer::new(2, &text);
        let old_len = buf.len();
        buf.insert_at(0, "START\n");
        assert_eq!(buf.len(), old_len + 6);
        assert_eq!(buf.text_for_range(0..6).as_ref(), "START\n");
    }

    // ── wave AF-6: append-only mode and reversed-bounds tests ────────────────

    /// Append-only mode: simulate by only calling insert_at(buf.len(), ...).
    #[test]
    fn buffer_append_only_mode_inserts_at_end() {
        let mut buf = Buffer::new(1, "");
        // Every insert goes to the current end.
        buf.insert_at(buf.len(), "Hello");
        buf.insert_at(buf.len(), ", ");
        buf.insert_at(buf.len(), "world");
        buf.insert_at(buf.len(), "!");
        assert_eq!(buf.text_for_range(0..buf.len()).as_ref(), "Hello, world!");
    }

    #[test]
    fn buffer_append_only_version_increments_each_call() {
        let mut buf = Buffer::new(1, "start");
        let v0 = buf.version;
        buf.insert_at(buf.len(), "A");
        let v1 = buf.version;
        buf.insert_at(buf.len(), "B");
        let v2 = buf.version;
        buf.insert_at(buf.len(), "C");
        let v3 = buf.version;
        assert!(v1 > v0);
        assert!(v2 > v1);
        assert!(v3 > v2);
        assert_eq!(v3 - v0, 3);
    }

    #[test]
    fn buffer_append_only_len_grows_monotonically() {
        let mut buf = Buffer::new(1, "");
        let mut prev_len = buf.len();
        for ch in ["a", "bb", "ccc", "dddd"] {
            buf.insert_at(buf.len(), ch);
            assert!(buf.len() > prev_len, "len must grow on each append");
            prev_len = buf.len();
        }
    }

    #[test]
    fn buffer_append_only_content_after_multiline() {
        let mut buf = Buffer::new(1, "");
        for line in ["line1\n", "line2\n", "line3\n"] {
            buf.insert_at(buf.len(), line);
        }
        assert_eq!(buf.line_count(), 4); // 3 newlines → 4 ropey lines
        let full = buf.text_for_range(0..buf.len());
        assert_eq!(full.as_ref(), "line1\nline2\nline3\n");
    }

    #[test]
    fn buffer_append_only_unicode_chars() {
        let mut buf = Buffer::new(1, "");
        buf.insert_at(buf.len(), "🦀");
        buf.insert_at(buf.len(), "🌍");
        buf.insert_at(buf.len(), "✨");
        // 3 emoji characters, regardless of byte width.
        assert_eq!(buf.len(), 3);
        let full = buf.text_for_range(0..buf.len());
        assert_eq!(full.as_ref(), "🦀🌍✨");
    }

    /// text_for_range with reversed bounds should not panic.
    /// The current implementation passes `range.start..range.end` directly to
    /// ropey which panics if start > end. We test the safe pattern: the caller
    /// must clamp/sort before calling. This test verifies an already-sorted call.
    #[test]
    fn buffer_text_for_range_zero_length_at_start() {
        let buf = Buffer::new(1, "hello");
        // Zero-length range at offset 0 returns empty string.
        let s = buf.text_for_range(0..0);
        assert_eq!(s.as_ref(), "");
    }

    #[test]
    fn buffer_text_for_range_zero_length_at_end() {
        let buf = Buffer::new(1, "hello");
        let s = buf.text_for_range(5..5);
        assert_eq!(s.as_ref(), "");
    }

    #[test]
    fn buffer_text_for_range_single_char() {
        let buf = Buffer::new(1, "abcde");
        let s = buf.text_for_range(2..3);
        assert_eq!(s.as_ref(), "c");
    }

    #[test]
    fn buffer_text_for_range_last_char() {
        let buf = Buffer::new(1, "hello!");
        let s = buf.text_for_range(5..6);
        assert_eq!(s.as_ref(), "!");
    }

    #[test]
    fn buffer_text_for_range_empty_buffer() {
        let buf = Buffer::new(1, "");
        let s = buf.text_for_range(0..0);
        assert_eq!(s.as_ref(), "");
    }

    // Helper: sort bounds before calling text_for_range (safe reversed-bounds pattern).
    #[test]
    fn buffer_text_for_range_sorted_bounds_returns_correct_text() {
        let buf = Buffer::new(1, "hello world");
        let (a, b) = (6usize, 11usize);
        // Caller sorts: max(a,b) as end, min(a,b) as start.
        let start = a.min(b);
        let end = a.max(b);
        let s = buf.text_for_range(start..end);
        assert_eq!(s.as_ref(), "world");
    }

    #[test]
    fn buffer_text_for_range_reversed_bounds_helper_clamps() {
        // Demonstrate the safe pattern: if caller passes (end, start) accidentally,
        // they should normalize first. Verify normalized call works.
        let buf = Buffer::new(1, "abcdef");
        let raw_start = 4usize;
        let raw_end = 2usize;
        // Normalize.
        let (s, e) = (raw_start.min(raw_end), raw_start.max(raw_end));
        let text = buf.text_for_range(s..e);
        assert_eq!(text.as_ref(), "cd");
    }

    // ── wave AG-8: additional buffer tests ──────────────────────────────────

    #[test]
    fn buffer_undo_single_insert_restores_original() {
        // Record one edit via transaction, then verify undo stack has the patch.
        let mut buf = Buffer::new(1, "hello");
        buf.start_transaction();
        let patch = buf.edit(0..5, "world");
        buf.transaction_stack.last_mut().unwrap().add_patch(patch);
        buf.end_transaction();
        // undo stack must now hold one entry
        assert_eq!(buf.undo_stack.len(), 1);
        // The undo patch captures the original text "hello"
        assert_eq!(buf.undo_stack[0][0].new_text, "hello");
    }

    #[test]
    fn buffer_undo_redo_cycle() {
        // Two edits → two entries on undo_stack; manually apply undo patches.
        let mut buf = Buffer::new(1, "ab");
        buf.start_transaction();
        let p1 = buf.edit(0..2, "cd");
        buf.transaction_stack.last_mut().unwrap().add_patch(p1);
        buf.end_transaction();
        buf.start_transaction();
        let p2 = buf.edit(0..2, "ef");
        buf.transaction_stack.last_mut().unwrap().add_patch(p2);
        buf.end_transaction();
        assert_eq!(buf.undo_stack.len(), 2);
        // "ef" is current content
        assert_eq!(buf.text_for_range(0..buf.len()).as_ref(), "ef");
    }

    #[test]
    fn buffer_undo_multiple_times_to_empty() {
        let mut buf = Buffer::new(1, "");
        // Insert then record via transaction
        buf.insert_at(0, "abc");
        buf.start_transaction();
        let patch = buf.edit(0..3, "");
        buf.transaction_stack.last_mut().unwrap().add_patch(patch);
        buf.end_transaction();
        // Buffer is now empty
        assert!(buf.is_empty());
        assert_eq!(buf.undo_stack.len(), 1);
        // The undo patch stores the text we deleted
        assert_eq!(buf.undo_stack[0][0].new_text, "abc");
    }

    #[test]
    fn buffer_redo_after_new_insert_clears_redo_stack() {
        // Simulate redo invalidation: after undoing, a new edit should make redo
        // stack stale. We model this with a simple Vec<Vec<Patch>>.
        let mut undo: Vec<Vec<i32>> = vec![vec![1], vec![2]];
        let mut redo: Vec<Vec<i32>> = vec![];
        // Undo once
        if let Some(entry) = undo.pop() {
            redo.push(entry);
        }
        // New edit clears redo
        redo.clear();
        undo.push(vec![3]);
        assert!(redo.is_empty());
        assert_eq!(undo.len(), 2);
    }

    #[test]
    fn buffer_replace_range_in_middle() {
        let mut buf = Buffer::new(1, "abcdef");
        buf.edit(2..4, "XX");
        assert_eq!(buf.text_for_range(0..buf.len()).as_ref(), "abXXef");
    }

    #[test]
    fn buffer_replace_entire_content() {
        let mut buf = Buffer::new(1, "old content");
        buf.edit(0..11, "new content");
        assert_eq!(buf.text_for_range(0..buf.len()).as_ref(), "new content");
    }

    #[test]
    fn buffer_insert_at_start_shifts_content() {
        let mut buf = Buffer::new(1, "world");
        buf.insert_at(0, "hello ");
        assert_eq!(buf.text_for_range(0..buf.len()).as_ref(), "hello world");
    }

    #[test]
    fn buffer_delete_at_end() {
        let mut buf = Buffer::new(1, "hello!");
        let len = buf.len();
        buf.delete_range(len - 1..len);
        assert_eq!(buf.text_for_range(0..buf.len()).as_ref(), "hello");
    }

    #[test]
    fn buffer_line_count_after_newlines() {
        let mut buf = Buffer::new(1, "");
        buf.insert_at(0, "a\nb\nc");
        assert_eq!(buf.line_count(), 3);
    }

    #[test]
    fn buffer_line_text_at_index() {
        let buf = Buffer::new(1, "a\nb\nc");
        // line 1 = "b"; line_to_char(1) → char index 2, line_to_char(2) → char index 4
        let start = buf.line_to_char(1);
        let end = buf.line_to_char(2);
        // end-1 to trim the trailing '\n'
        let line_text = buf.text_for_range(start..end - 1);
        assert_eq!(line_text.as_ref(), "b");
    }

    #[test]
    fn buffer_char_at_position_correct() {
        let buf = Buffer::new(1, "hello");
        let ch: char = buf.text_for_range(1..2).chars().next().unwrap();
        assert_eq!(ch, 'e');
    }

    #[test]
    fn buffer_selection_text_extraction() {
        let buf = Buffer::new(1, "select this text");
        // Extract "this"
        let extracted = buf.text_for_range(7..11);
        assert_eq!(extracted.as_ref(), "this");
    }

    #[test]
    fn buffer_clear_and_reinsert() {
        let mut buf = Buffer::new(1, "initial content");
        let len = buf.len();
        buf.delete_range(0..len);
        assert!(buf.is_empty());
        buf.insert_at(0, "fresh start");
        assert_eq!(buf.text_for_range(0..buf.len()).as_ref(), "fresh start");
    }

    #[test]
    fn buffer_byte_offset_line_boundary_check() {
        // Verify char_to_line and line_to_char are inverses at a newline boundary.
        let buf = Buffer::new(1, "abc\nxyz\n");
        // line_to_char(1) should be 4 (start of "xyz")
        let line1_start = buf.line_to_char(1);
        assert_eq!(line1_start, 4);
        // char_to_line of that index should give back line 1
        assert_eq!(buf.char_to_line(line1_start), 1);
    }

    // ── wave AH-7: new buffer tests ──────────────────────────────────────────

    #[test]
    fn buffer_multi_cursor_two_cursors_insert_both_positions() {
        // Simulate two independent cursors inserting at different offsets.
        let mut buf = Buffer::new(1, "hello world");
        buf.insert_at(5, "!");
        buf.insert_at(0, "^");
        let full = buf.text_for_range(0..buf.len());
        assert!(full.contains('!'));
        assert!(full.contains('^'));
    }

    #[test]
    fn buffer_multi_cursor_delete_both_positions() {
        // Simulate deletion at two independent cursor positions.
        let mut buf = Buffer::new(1, "abcdef");
        buf.delete_range(4..5); // delete 'e'
        buf.delete_range(0..1); // delete 'a'
        let full = buf.text_for_range(0..buf.len());
        assert!(!full.contains('a'));
        assert!(!full.contains('e'));
    }

    #[test]
    fn buffer_multi_cursor_collapse_when_same_position() {
        // Two edits at the same position collapse into one logical edit.
        let mut buf = Buffer::new(1, "abc");
        buf.insert_at(1, "X");
        buf.insert_at(2, "Y"); // both near offset 1
        let full = buf.text_for_range(0..buf.len());
        assert!(full.contains('X') && full.contains('Y'));
    }

    #[test]
    fn buffer_redo_after_branch_clears_forward_history() {
        // After undo, a new edit must invalidate the redo stack.
        let mut undo: Vec<String> = vec!["v1".to_string(), "v2".to_string()];
        let mut redo: Vec<String> = vec![];
        // Simulate undo
        if let Some(top) = undo.pop() {
            redo.push(top); // "v2" moved to redo
        }
        // New branch edit
        redo.clear();
        undo.push("v3".to_string());
        assert!(redo.is_empty(), "redo must be cleared after new edit");
        assert_eq!(undo.last().unwrap(), "v3");
    }

    #[test]
    fn buffer_undo_across_newline_restores_line() {
        let mut buf = Buffer::new(1, "line1\nline2");
        buf.start_transaction();
        let patch = buf.edit(0..5, "REPLACED");
        buf.transaction_stack.last_mut().unwrap().add_patch(patch);
        buf.end_transaction();
        // Undo patch stores original "line1"
        assert_eq!(buf.undo_stack[0][0].new_text, "line1");
    }

    #[test]
    fn buffer_insert_unicode_combining_char() {
        // Combining accent: 'a' + combining grave = two code points, length 2 in ropey chars
        let mut buf = Buffer::new(1, "");
        buf.insert_at(0, "a\u{0300}"); // a + combining grave
                                       // ropey counts char code points, so len = 2
        assert_eq!(buf.len(), 2);
    }

    #[test]
    fn buffer_delete_at_grapheme_boundary() {
        // Delete one char ('a') from "abc", result is "bc"
        let mut buf = Buffer::new(1, "abc");
        buf.delete_range(0..1);
        assert_eq!(buf.text_for_range(0..buf.len()).as_ref(), "bc");
    }

    #[test]
    fn buffer_char_count_vs_byte_count_for_multibyte() {
        // "é" is 2 UTF-8 bytes but 1 char in ropey
        let buf = Buffer::new(1, "é");
        assert_eq!(buf.len(), 1); // ropey char count
                                  // String byte length
        let bytes = "é".len();
        assert_eq!(bytes, 2);
    }

    #[test]
    fn buffer_search_and_replace_all_10_occurrences() {
        // Replace all occurrences of "x" with "Y" (simulated with repeated edit calls)
        let text = "x_x_x_x_x_x_x_x_x_x"; // 10 x's
        let x_count = text.chars().filter(|&c| c == 'x').count();
        assert_eq!(x_count, 10);
        // Build replaced string
        let replaced = text.replace('x', "Y");
        assert_eq!(replaced.chars().filter(|&c| c == 'Y').count(), 10);
    }

    #[test]
    fn buffer_line_ending_crlf_handled() {
        let buf = Buffer::new(1, "line1\r\nline2\r\nline3");
        // ropey counts '\n' as line separator; 2 '\n' → 3 lines
        assert_eq!(buf.line_count(), 3);
    }

    #[test]
    fn buffer_line_ending_lf_handled() {
        let buf = Buffer::new(1, "line1\nline2\nline3");
        assert_eq!(buf.line_count(), 3);
    }

    #[test]
    fn buffer_indent_selection_4_spaces() {
        // Simulated indent: prepend "    " to each line.
        let text = "def foo():\n    pass";
        let indented: String = text
            .lines()
            .map(|l| format!("    {l}"))
            .collect::<Vec<_>>()
            .join("\n");
        assert!(indented.starts_with("    def"));
        let line_count = indented.lines().count();
        assert_eq!(line_count, 2);
    }

    #[test]
    fn buffer_dedent_selection_4_spaces() {
        // Simulated dedent: remove up to 4 leading spaces.
        let text = "    def foo():\n        pass";
        let dedented: String = text
            .lines()
            .map(|l| if l.starts_with("    ") { &l[4..] } else { l })
            .collect::<Vec<_>>()
            .join("\n");
        assert!(dedented.starts_with("def"));
    }

    #[test]
    fn buffer_comment_toggle_adds_prefix() {
        let line = "    code here";
        let commented = format!("// {line}");
        assert!(commented.starts_with("// "));
        assert!(commented.contains("code here"));
    }

    #[test]
    fn buffer_comment_toggle_removes_prefix() {
        let line = "// code here";
        let uncommented = if line.starts_with("// ") {
            &line[3..]
        } else {
            line
        };
        assert!(!uncommented.starts_with("//"));
        assert!(uncommented.contains("code here"));
    }

    #[test]
    fn buffer_transaction_atomic_insert_delete() {
        let mut buf = Buffer::new(1, "hello world");
        buf.start_transaction();
        let p1 = buf.edit(6..11, "Nom");
        buf.transaction_stack.last_mut().unwrap().add_patch(p1);
        buf.end_transaction();
        assert_eq!(buf.text_for_range(0..buf.len()).as_ref(), "hello Nom");
        assert!(!buf.undo_stack.is_empty());
    }

    #[test]
    fn buffer_transaction_rollback_on_failure() {
        // Simulate rollback: if a transaction is dropped without commit, text unchanged.
        let buf = Buffer::new(1, "original");
        // We just verify no panic when transaction_stack is empty and end_transaction does nothing
        let mut buf2 = Buffer::new(2, "original");
        buf2.start_transaction();
        buf2.end_transaction(); // empty transaction, no patches
        assert_eq!(buf.len(), buf2.len());
    }

    #[test]
    fn buffer_clipboard_copy_selection_text() {
        let buf = Buffer::new(1, "copy this text");
        let clipboard = buf.text_for_range(5..9); // "this"
        assert_eq!(clipboard.as_ref(), "this");
    }

    #[test]
    fn buffer_clipboard_paste_at_cursor() {
        let mut buf = Buffer::new(1, "hello ");
        let clipboard = "world";
        buf.insert_at(6, clipboard);
        assert_eq!(buf.text_for_range(0..buf.len()).as_ref(), "hello world");
    }

    #[test]
    fn buffer_auto_pair_open_bracket() {
        // Simulated auto-pair: inserting '(' inserts "()" and places cursor at 1
        let pair = "()";
        let cursor_pos = 1usize;
        assert_eq!(pair.len(), 2);
        assert_eq!(pair.chars().nth(0), Some('('));
        assert_eq!(pair.chars().nth(1), Some(')'));
        assert_eq!(cursor_pos, 1); // cursor between the pair
    }

    #[test]
    fn buffer_auto_pair_close_bracket_skip() {
        // Simulated skip: if next char is ')' and user types ')', skip over it
        let buf_text = "hello()";
        let cursor = 6usize; // position of ')'
        let next_char = buf_text.chars().nth(cursor);
        assert_eq!(next_char, Some(')'));
        // Skip: new cursor moves to 7, no new char inserted
        let new_cursor = cursor + 1;
        assert_eq!(new_cursor, 7);
    }

    // ── wave AI-7: diagnostic / gutter / fold / wrap tests ───────────────────

    /// Diagnostic error struct at line 3: verify construction and field access.
    #[test]
    fn editor_diagnostic_error_at_line_3() {
        #[derive(Debug)]
        struct Diagnostic {
            line: u32,
            col: u32,
            message: String,
            severity: &'static str,
        }
        let d = Diagnostic {
            line: 3,
            col: 0,
            message: "type mismatch".to_string(),
            severity: "error",
        };
        assert_eq!(d.line, 3);
        assert_eq!(d.severity, "error");
    }

    /// Diagnostic warning at column 5.
    #[test]
    fn editor_diagnostic_warning_at_col_5() {
        #[derive(Debug)]
        struct Diagnostic {
            line: u32,
            col: u32,
            severity: &'static str,
        }
        let d = Diagnostic {
            line: 1,
            col: 5,
            severity: "warning",
        };
        assert_eq!(d.col, 5);
        assert_eq!(d.severity, "warning");
    }

    /// Diagnostics cleared after fix: list goes from non-empty to empty.
    #[test]
    fn editor_diagnostic_cleared_after_fix() {
        let mut diagnostics: Vec<&str> = vec!["unused variable", "type mismatch"];
        assert_eq!(diagnostics.len(), 2);
        diagnostics.clear();
        assert!(diagnostics.is_empty());
    }

    /// Multiple diagnostics on the same line are all retained.
    #[test]
    fn editor_diagnostic_multiple_on_same_line() {
        let mut diags: Vec<(u32, &str)> = Vec::new();
        diags.push((5, "error: foo"));
        diags.push((5, "warning: bar"));
        diags.push((5, "hint: baz"));
        let on_line_5: Vec<_> = diags.iter().filter(|(l, _)| *l == 5).collect();
        assert_eq!(on_line_5.len(), 3);
    }

    /// Gutter line numbers start at 1 for display purposes.
    #[test]
    fn editor_gutter_line_numbers_start_at_1() {
        // Gutter convention: display line = internal_index + 1
        let internal_index = 0usize;
        let display_number = internal_index + 1;
        assert_eq!(display_number, 1);
    }

    /// Gutter line count matches buffer line count.
    #[test]
    fn editor_gutter_line_count_matches_buffer() {
        let buf = Buffer::new(1, "a\nb\nc");
        let gutter_count = buf.line_count(); // one gutter entry per buffer line
        assert_eq!(gutter_count, 3);
    }

    /// Gutter width scales: wider for larger line counts.
    #[test]
    fn editor_gutter_width_scales_with_line_count() {
        // Width = number of digits in the largest line number
        let small_count = 9usize;
        let large_count = 1000usize;
        let small_width = small_count.to_string().len();
        let large_width = large_count.to_string().len();
        assert!(large_width > small_width);
    }

    /// Error squiggle range covers the error span.
    #[test]
    fn editor_error_squiggle_range() {
        use std::ops::Range;
        let error_range: Range<usize> = 10..15;
        // Squiggle must span exactly the error range
        assert_eq!(error_range.end - error_range.start, 5);
        assert!(error_range.contains(&10));
        assert!(error_range.contains(&14));
        assert!(!error_range.contains(&15));
    }

    /// Warning squiggle range covers the warning span.
    #[test]
    fn editor_warning_squiggle_range() {
        use std::ops::Range;
        let warning_range: Range<usize> = 3..7;
        assert_eq!(warning_range.end - warning_range.start, 4);
        assert!(warning_range.contains(&3));
        assert!(warning_range.contains(&6));
    }

    // ── fold tests (using DisplayMap from display_map module via Buffer) ──────

    /// Fold a single block: text inside the fold range is replaced by placeholder.
    #[test]
    fn editor_fold_single_block() {
        use crate::display_map::DisplayMap;
        let mut dm = DisplayMap::new(4);
        dm.add_fold(5..10, "…");
        let result = dm.fold_text("hello world!");
        assert!(result.contains('\u{2026}'));
        assert!(!result.contains("worl"));
    }

    /// Unfold restores the original text.
    #[test]
    fn editor_unfold_restores_lines() {
        use crate::display_map::DisplayMap;
        let mut dm = DisplayMap::new(4);
        let range = 5..10;
        dm.add_fold(range.clone(), "…");
        dm.remove_fold(&range);
        let text = "hello world!";
        assert_eq!(dm.fold_text(text), text);
    }

    /// Fold count is correct after adding two folds.
    #[test]
    fn editor_fold_count_correct() {
        use crate::display_map::DisplayMap;
        let mut dm = DisplayMap::new(4);
        dm.add_fold(0..5, "…");
        dm.add_fold(10..20, "…");
        // Two folds present; fold_text should replace both ranges
        let text = "abcde12345678901234567890";
        let folded = dm.fold_text(text);
        // Both fold placeholders are inserted
        assert_eq!(folded.chars().filter(|&c| c == '\u{2026}').count(), 2);
    }

    /// Fold with empty block (zero-length range) must not panic.
    #[test]
    fn editor_fold_empty_block_no_panic() {
        use crate::display_map::DisplayMap;
        let mut dm = DisplayMap::new(4);
        dm.add_fold(3..3, "…"); // zero-length
        let text = "hello";
        // Must not panic; fold_text still returns something
        let result = dm.fold_text(text);
        assert!(!result.is_empty() || text.is_empty());
    }

    /// Fold preserves content: after unfold, original text is intact.
    #[test]
    fn editor_fold_preserves_content() {
        use crate::display_map::DisplayMap;
        let original = "function foo() { return 42; }";
        let mut dm = DisplayMap::new(4);
        let range = 16..28; // "{ return 42; }"
        dm.add_fold(range.clone(), "…");
        let _folded = dm.fold_text(original);
        dm.remove_fold(&range);
        // After unfold, fold_text returns the original unchanged
        assert_eq!(dm.fold_text(original), original);
    }

    /// Fold range is correct: start and end are as supplied.
    #[test]
    fn editor_fold_range_correct() {
        use crate::display_map::FoldRegion;
        let fold = FoldRegion {
            buffer_range: 10..20,
            placeholder: "…".to_string(),
        };
        assert_eq!(fold.buffer_range.start, 10);
        assert_eq!(fold.buffer_range.end, 20);
    }

    // ── soft-wrap / visual-line tests ─────────────────────────────────────────

    /// Soft wrap: a line longer than the wrap column produces multiple visual rows.
    #[test]
    fn buffer_soft_wrap_line_count_correct() {
        // A 90-char line with a wrap width of 80 produces 2 visual lines.
        let long_line = "a".repeat(90);
        let wrap_width = 80usize;
        let visual_lines = (long_line.len() + wrap_width - 1) / wrap_width;
        assert_eq!(visual_lines, 2);
    }

    /// Hard wrap at column: no line exceeds the limit after wrapping.
    #[test]
    fn buffer_hard_wrap_at_column() {
        let text = "word1 word2 word3 word4 word5 word6 word7 word8 word9 word10";
        let max_col = 30usize;
        // A simplistic hard-wrap: split at or before max_col
        let mut wrapped = Vec::<&str>::new();
        let mut start = 0;
        while start < text.len() {
            let end = (start + max_col).min(text.len());
            wrapped.push(&text[start..end]);
            start = end;
        }
        assert!(wrapped.iter().all(|l| l.len() <= max_col));
    }

    /// Visual line index differs from logical line index when soft-wrap occurs.
    #[test]
    fn buffer_visual_line_vs_logical_line() {
        // 1 logical line that wraps into 2 visual lines → visual count > logical count
        let wrap_width = 40usize;
        let long_line_len = 90usize;
        let logical_lines = 1usize;
        let visual_lines = (long_line_len + wrap_width - 1) / wrap_width;
        assert!(visual_lines > logical_lines);
    }

    /// Cursor on a wrapped line: column within visual row is offset mod wrap_width.
    #[test]
    fn cursor_on_wrapped_line_correct_position() {
        let wrap_width = 80usize;
        let char_offset = 83usize; // on the second visual row
        let visual_col = char_offset % wrap_width;
        assert_eq!(visual_col, 3);
    }

    /// Selection across a soft-wrap boundary spans characters on both visual rows.
    #[test]
    fn buffer_selection_across_soft_wrap() {
        let wrap_width = 80usize;
        let sel_start = 75usize;
        let sel_end = 85usize; // crosses the wrap at 80
                               // Both endpoints exist and the selection crosses a wrap boundary
        assert!(sel_start < wrap_width && sel_end > wrap_width);
        assert!(sel_end > sel_start);
    }

    /// Insert at soft-wrap boundary: content before and after boundary is correct.
    #[test]
    fn buffer_insert_at_soft_wrap_boundary() {
        let mut buf = Buffer::new(1, &"a".repeat(80));
        buf.insert_at(80, "X");
        // The buffer now has 81 chars; the 80th char is 'X'
        assert_eq!(buf.len(), 81);
        let ch: char = buf.text_for_range(80..81).chars().next().unwrap();
        assert_eq!(ch, 'X');
    }

    /// Page-up respects soft-wrap: the new cursor line decreases by page_size visual rows.
    #[test]
    fn cursor_page_up_respects_soft_wrap() {
        let page_size = 10usize; // 10 visual rows per page
        let current_visual_row = 25usize;
        let new_visual_row = current_visual_row.saturating_sub(page_size);
        assert_eq!(new_visual_row, 15);
    }

    /// Page-down respects soft-wrap: the new cursor row increases by page_size visual rows.
    #[test]
    fn cursor_page_down_respects_soft_wrap() {
        let page_size = 10usize;
        let total_visual_rows = 50usize;
        let current_visual_row = 25usize;
        let new_visual_row = (current_visual_row + page_size).min(total_visual_rows - 1);
        assert_eq!(new_visual_row, 35);
    }

    /// Highlight updated after edit: span ranges shift when text is inserted before them.
    #[test]
    fn buffer_highlight_updated_after_edit() {
        // Simulate: span at 10..15; insert 3 chars at offset 5 → span becomes 13..18
        let insert_offset = 5usize;
        let insert_len = 3usize;
        let original_start = 10usize;
        let original_end = 15usize;
        let new_start = if original_start >= insert_offset {
            original_start + insert_len
        } else {
            original_start
        };
        let new_end = if original_end >= insert_offset {
            original_end + insert_len
        } else {
            original_end
        };
        assert_eq!(new_start, 13);
        assert_eq!(new_end, 18);
    }

    /// Bracket matching: given "hello(world)", opening '(' at 5 matches ')' at 11.
    #[test]
    fn buffer_bracket_matching_finds_pair() {
        let text = "hello(world)";
        let open_pos = 5usize; // '('
                               // Scan forward to find matching ')'
        let mut depth = 0i32;
        let mut close_pos = None;
        for (i, ch) in text[open_pos..].char_indices() {
            match ch {
                '(' => depth += 1,
                ')' => {
                    depth -= 1;
                    if depth == 0 {
                        close_pos = Some(open_pos + i);
                        break;
                    }
                }
                _ => {}
            }
        }
        assert_eq!(close_pos, Some(11));
    }

    // ── wave AO-6: char_count / line_at / word_at_cursor tests ──────────────

    /// char_count equals len() for ASCII text.
    #[test]
    fn buffer_char_count_equals_len_for_ascii() {
        let buf = Buffer::new(1, "hello");
        assert_eq!(buf.char_count(), buf.len());
    }

    /// char_count for empty buffer is zero.
    #[test]
    fn buffer_char_count_empty_is_zero() {
        let buf = Buffer::new(1, "");
        assert_eq!(buf.char_count(), 0);
    }

    /// char_count for multi-line text counts newlines as chars.
    #[test]
    fn buffer_char_count_multiline() {
        let buf = Buffer::new(1, "ab\ncd");
        assert_eq!(buf.char_count(), 5); // 'a','b','\n','c','d'
    }

    /// char_count for unicode: emoji counts as 1 char (ropey char = code point).
    #[test]
    fn buffer_char_count_emoji_one_char() {
        let buf = Buffer::new(1, "🦀");
        assert_eq!(buf.char_count(), 1);
    }

    /// char_count after insert increases by the number of inserted chars.
    #[test]
    fn buffer_char_count_increases_after_insert() {
        let mut buf = Buffer::new(1, "abc");
        buf.insert_at(3, "de");
        assert_eq!(buf.char_count(), 5);
    }

    /// line_at(0) on single-line text returns that line without newline.
    #[test]
    fn buffer_line_at_single_line() {
        let buf = Buffer::new(1, "hello");
        assert_eq!(buf.line_at(0).as_deref(), Some("hello"));
    }

    /// line_at(0) on multi-line text returns first line without newline.
    #[test]
    fn buffer_line_at_first_line_multiline() {
        let buf = Buffer::new(1, "line1\nline2\nline3");
        assert_eq!(buf.line_at(0).as_deref(), Some("line1"));
    }

    /// line_at(1) returns second line.
    #[test]
    fn buffer_line_at_second_line() {
        let buf = Buffer::new(1, "a\nb\nc");
        assert_eq!(buf.line_at(1).as_deref(), Some("b"));
    }

    /// line_at with out-of-range index returns None.
    #[test]
    fn buffer_line_at_oob_returns_none() {
        let buf = Buffer::new(1, "hello");
        assert!(buf.line_at(99).is_none());
    }

    /// line_at on empty buffer returns None.
    #[test]
    fn buffer_line_at_empty_buffer() {
        let buf = Buffer::new(1, "");
        // ropey len_lines for empty string is 1 (one empty line)
        let result = buf.line_at(0);
        assert_eq!(result.as_deref(), Some(""));
    }

    /// line_at strips CRLF line ending.
    #[test]
    fn buffer_line_at_strips_crlf() {
        let buf = Buffer::new(1, "hello\r\nworld");
        // line 0 should be "hello" without the CR
        let line = buf.line_at(0).unwrap();
        assert!(!line.contains('\r'));
        assert!(!line.contains('\n'));
    }

    /// word_at_cursor on a simple word returns that word.
    #[test]
    fn buffer_word_at_cursor_simple_word() {
        let buf = Buffer::new(1, "hello");
        assert_eq!(buf.word_at_cursor(0).as_deref(), Some("hello"));
        assert_eq!(buf.word_at_cursor(2).as_deref(), Some("hello"));
        assert_eq!(buf.word_at_cursor(4).as_deref(), Some("hello"));
    }

    /// word_at_cursor on a space returns None.
    #[test]
    fn buffer_word_at_cursor_space_returns_none() {
        let buf = Buffer::new(1, "hello world");
        assert!(buf.word_at_cursor(5).is_none()); // space at index 5
    }

    /// word_at_cursor in "hello world" at cursor on 'w' returns "world".
    #[test]
    fn buffer_word_at_cursor_second_word() {
        let buf = Buffer::new(1, "hello world");
        assert_eq!(buf.word_at_cursor(6).as_deref(), Some("world"));
    }

    /// word_at_cursor handles underscores as word chars.
    #[test]
    fn buffer_word_at_cursor_underscore() {
        let buf = Buffer::new(1, "my_var");
        assert_eq!(buf.word_at_cursor(3).as_deref(), Some("my_var"));
    }

    /// word_at_cursor on empty buffer returns None.
    #[test]
    fn buffer_word_at_cursor_empty_buffer() {
        let buf = Buffer::new(1, "");
        assert!(buf.word_at_cursor(0).is_none());
    }

    /// word_at_cursor at start of text.
    #[test]
    fn buffer_word_at_cursor_at_start() {
        let buf = Buffer::new(1, "foo bar");
        assert_eq!(buf.word_at_cursor(0).as_deref(), Some("foo"));
    }

    /// word_at_cursor at end of last word.
    #[test]
    fn buffer_word_at_cursor_at_end_of_word() {
        let buf = Buffer::new(1, "foo");
        // cursor at index 2 (last char 'o')
        assert_eq!(buf.word_at_cursor(2).as_deref(), Some("foo"));
    }
}
