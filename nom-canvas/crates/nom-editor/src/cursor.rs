use crate::buffer::Buffer;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Cursor {
    /// Primary position (where new text is typed).
    pub head: usize,
    /// Optional selection anchor (None == no selection).
    pub anchor: Option<usize>,
    /// Goal column for vertical navigation (remembered so moving up-down
    /// preserves intent when a shorter line is encountered).
    pub goal_column: GoalColumn,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum GoalColumn {
    /// Use the current column.
    Current,
    /// Use this remembered column when moving vertically.
    Sticky(usize),
}

impl Cursor {
    pub fn at(head: usize) -> Self {
        Self { head, anchor: None, goal_column: GoalColumn::Current }
    }

    pub fn select(head: usize, anchor: usize) -> Self {
        Self { head, anchor: Some(anchor), goal_column: GoalColumn::Current }
    }

    pub fn has_selection(&self) -> bool {
        self.anchor.is_some() && self.anchor != Some(self.head)
    }

    pub fn range(&self) -> std::ops::Range<usize> {
        match self.anchor {
            Some(a) if a <= self.head => a..self.head,
            Some(a) => self.head..a,
            None => self.head..self.head,
        }
    }
}

pub struct CursorSet {
    pub cursors: smallvec::SmallVec<[Cursor; 4]>,
}

impl CursorSet {
    pub fn single(c: Cursor) -> Self {
        Self { cursors: smallvec::smallvec![c] }
    }

    /// Apply an edit atomically: ascending-offset pattern — insert from lowest
    /// to highest, shifting all subsequent cursor positions by the inserted length.
    /// This preserves all cursor heads correctly after multi-cursor insert.
    pub fn insert_at_each(&mut self, buffer: &mut Buffer, text: &str) {
        let delta = text.chars().count();
        // Sort ascending so we can shift later cursors after each insert.
        self.cursors.sort_by(|a, b| a.head.cmp(&b.head));
        let mut accumulated_shift: usize = 0;
        for cursor in &mut self.cursors {
            let insert_at = cursor.head + accumulated_shift;
            buffer.insert(insert_at, text);
            cursor.head = insert_at + delta;
            cursor.anchor = None;
            accumulated_shift += delta;
        }
    }

    pub fn delete_selections(&mut self, buffer: &mut Buffer) {
        // Reverse-offset: delete from highest offset first.
        self.cursors.sort_by(|a, b| b.head.cmp(&a.head));
        for cursor in &mut self.cursors {
            if cursor.has_selection() {
                let r = cursor.range();
                let new_head = buffer.delete(r);
                cursor.head = new_head;
                cursor.anchor = None;
            }
        }
        self.cursors.sort_by(|a, b| a.head.cmp(&b.head));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn single_cursor_insert_moves_head() {
        let mut buf = Buffer::new();
        let mut cs = CursorSet::single(Cursor::at(0));
        cs.insert_at_each(&mut buf, "hi");
        assert_eq!(cs.cursors[0].head, 2);
        assert_eq!(buf.to_string(), "hi");
    }

    #[test]
    fn two_cursors_atomic_insert_preserves_both() {
        // Buffer: "ab"  cursors at 0 and 2.
        // Reverse-offset insert: process offset=2 first → "abX" (head→3),
        // then offset=0 → "XabX" (head→1).
        // Both cursors survive; neither invalidates the other.
        let mut buf = Buffer::from_str("ab");
        let mut cs = CursorSet {
            cursors: smallvec::smallvec![Cursor::at(0), Cursor::at(2)],
        };
        cs.insert_at_each(&mut buf, "X");
        // After re-sort ascending: cursor[0].head=1, cursor[1].head=4.
        assert_eq!(buf.to_string(), "XabX");
        assert_eq!(cs.cursors[0].head, 1);
        assert_eq!(cs.cursors[1].head, 4);
    }

    #[test]
    fn selection_delete_shrinks_buffer() {
        let mut buf = Buffer::from_str("hello world");
        // Select " world" (chars 5..11).
        let mut cs = CursorSet::single(Cursor::select(11, 5));
        cs.delete_selections(&mut buf);
        assert_eq!(buf.to_string(), "hello");
        assert_eq!(cs.cursors[0].head, 5);
        assert!(!cs.cursors[0].has_selection());
    }
}
