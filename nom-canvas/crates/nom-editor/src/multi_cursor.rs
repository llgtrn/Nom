#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CursorAnchor {
    Start,
    End,
}

impl CursorAnchor {
    pub fn is_start(&self) -> bool {
        matches!(self, CursorAnchor::Start)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Cursor {
    pub row: usize,
    pub col: usize,
    pub anchor: CursorAnchor,
}

impl Cursor {
    pub fn new(row: usize, col: usize) -> Self {
        Cursor { row, col, anchor: CursorAnchor::End }
    }

    pub fn with_anchor(mut self, anchor: CursorAnchor) -> Self {
        self.anchor = anchor;
        self
    }

    pub fn move_right(&mut self, cols: usize) {
        self.col += cols;
    }

    pub fn move_left(&mut self, cols: usize) {
        self.col = self.col.saturating_sub(cols);
    }

    pub fn move_down(&mut self, rows: usize) {
        self.row += rows;
    }

    pub fn move_up(&mut self, rows: usize) {
        self.row = self.row.saturating_sub(rows);
    }
}

#[derive(Debug, Clone)]
pub struct CursorRange {
    pub start: Cursor,
    pub end: Cursor,
}

impl CursorRange {
    pub fn new(start: Cursor, end: Cursor) -> Self {
        CursorRange { start, end }
    }

    pub fn is_empty(&self) -> bool {
        self.start.row == self.end.row && self.start.col == self.end.col
    }

    pub fn contains_row(&self, row: usize) -> bool {
        row >= self.start.row && row <= self.end.row
    }

    pub fn char_count(&self) -> usize {
        if self.start.row == self.end.row {
            self.end.col.saturating_sub(self.start.col)
        } else {
            (self.end.row - self.start.row) * 80 + self.end.col
        }
    }
}

#[derive(Debug, Clone)]
pub struct MultiCursor {
    pub cursors: Vec<Cursor>,
}

impl MultiCursor {
    pub fn new() -> Self {
        MultiCursor { cursors: Vec::new() }
    }

    pub fn add(&mut self, cursor: Cursor) {
        self.cursors.push(cursor);
    }

    pub fn primary(&self) -> Option<&Cursor> {
        self.cursors.first()
    }

    pub fn count(&self) -> usize {
        self.cursors.len()
    }

    pub fn dedup(&mut self) {
        let mut seen = std::collections::HashSet::new();
        self.cursors.retain(|c| seen.insert((c.row, c.col)));
    }

    pub fn move_all_right(&mut self, cols: usize) {
        for c in &mut self.cursors {
            c.move_right(cols);
        }
    }

    pub fn all_in_row(&self, row: usize) -> Vec<&Cursor> {
        self.cursors.iter().filter(|c| c.row == row).collect()
    }
}

impl Default for MultiCursor {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod multi_cursor_tests {
    use super::*;

    #[test]
    fn test_cursor_new_defaults() {
        let c = Cursor::new(3, 7);
        assert_eq!(c.row, 3);
        assert_eq!(c.col, 7);
        assert_eq!(c.anchor, CursorAnchor::End);
        assert!(!c.anchor.is_start());
    }

    #[test]
    fn test_move_right_and_move_left_saturating() {
        let mut c = Cursor::new(0, 5);
        c.move_right(3);
        assert_eq!(c.col, 8);
        c.move_left(10);
        assert_eq!(c.col, 0); // saturating sub — no underflow
        c.move_left(1);
        assert_eq!(c.col, 0);
    }

    #[test]
    fn test_cursor_range_is_empty() {
        let a = Cursor::new(2, 4);
        let b = Cursor::new(2, 4);
        let empty = CursorRange::new(a.clone(), b);
        assert!(empty.is_empty());

        let c = Cursor::new(2, 5);
        let non_empty = CursorRange::new(a, c);
        assert!(!non_empty.is_empty());
    }

    #[test]
    fn test_cursor_range_contains_row() {
        let r = CursorRange::new(Cursor::new(2, 0), Cursor::new(5, 0));
        assert!(r.contains_row(2));
        assert!(r.contains_row(3));
        assert!(r.contains_row(5));
        assert!(!r.contains_row(1));
        assert!(!r.contains_row(6));
    }

    #[test]
    fn test_multi_cursor_add_and_count() {
        let mut mc = MultiCursor::new();
        assert_eq!(mc.count(), 0);
        mc.add(Cursor::new(0, 0));
        mc.add(Cursor::new(1, 5));
        assert_eq!(mc.count(), 2);
    }

    #[test]
    fn test_primary_returns_first() {
        let mut mc = MultiCursor::new();
        assert!(mc.primary().is_none());
        mc.add(Cursor::new(10, 3));
        mc.add(Cursor::new(20, 7));
        let p = mc.primary().unwrap();
        assert_eq!(p.row, 10);
        assert_eq!(p.col, 3);
    }

    #[test]
    fn test_dedup_removes_duplicates() {
        let mut mc = MultiCursor::new();
        mc.add(Cursor::new(1, 1));
        mc.add(Cursor::new(1, 1)); // duplicate
        mc.add(Cursor::new(2, 3));
        mc.add(Cursor::new(1, 1)); // another duplicate
        mc.dedup();
        assert_eq!(mc.count(), 2);
    }

    #[test]
    fn test_move_all_right_moves_all() {
        let mut mc = MultiCursor::new();
        mc.add(Cursor::new(0, 0));
        mc.add(Cursor::new(1, 5));
        mc.move_all_right(3);
        assert_eq!(mc.cursors[0].col, 3);
        assert_eq!(mc.cursors[1].col, 8);
    }

    #[test]
    fn test_all_in_row_filter() {
        let mut mc = MultiCursor::new();
        mc.add(Cursor::new(2, 0));
        mc.add(Cursor::new(3, 1));
        mc.add(Cursor::new(2, 5));
        mc.add(Cursor::new(4, 2));
        let row2 = mc.all_in_row(2);
        assert_eq!(row2.len(), 2);
        assert!(row2.iter().all(|c| c.row == 2));
        let row5 = mc.all_in_row(5);
        assert_eq!(row5.len(), 0);
    }
}
