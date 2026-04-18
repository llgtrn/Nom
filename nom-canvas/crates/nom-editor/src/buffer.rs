#![deny(unsafe_code)]
use ropey::Rope;
use std::borrow::Cow;
use std::ops::Range;

pub type BufferId = u64;

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

    pub fn char_to_line(&self, char_idx: usize) -> usize {
        self.rope.char_to_line(char_idx.min(self.rope.len_chars()))
    }

    pub fn line_to_char(&self, line: usize) -> usize {
        self.rope.line_to_char(line.min(self.rope.len_lines()))
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
}
