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
}
