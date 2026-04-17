#![deny(unsafe_code)]
use ropey::Rope;
use std::ops::Range;
use std::borrow::Cow;

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
    pub fn new() -> Self { Self { patches: Vec::new(), active: true } }
    pub fn add_patch(&mut self, patch: Patch) { self.patches.push(patch); }
    pub fn commit(mut self) -> Vec<Patch> { self.active = false; self.patches }
}

impl Default for Transaction { fn default() -> Self { Self::new() } }

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

    pub fn len(&self) -> usize { self.rope.len_chars() }
    pub fn is_empty(&self) -> bool { self.rope.len_chars() == 0 }

    pub fn text_for_range(&self, range: Range<usize>) -> Cow<'_, str> {
        let slice = self.rope.slice(range.start..range.end);
        Cow::Owned(slice.to_string())
    }

    pub fn line_count(&self) -> usize { self.rope.len_lines() }

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
        Patch { old_range: start..start + new_text.len(), new_text: old_text }
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
}
