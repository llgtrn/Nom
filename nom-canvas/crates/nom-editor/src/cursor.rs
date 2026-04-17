#![deny(unsafe_code)]

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Bias { Left, Right }

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Anchor {
    pub offset: usize,
    pub bias: Bias,
}

impl Anchor {
    pub fn at(offset: usize) -> Self { Self { offset, bias: Bias::Left } }
    pub fn at_right(offset: usize) -> Self { Self { offset, bias: Bias::Right } }
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
        Self { start: anchor, end: anchor, goal_column: None, reversed: false }
    }

    pub fn range(start: usize, end: usize) -> Self {
        let (start, end, reversed) = if start <= end {
            (Anchor::at(start), Anchor::at_right(end), false)
        } else {
            (Anchor::at(end), Anchor::at_right(start), true)
        };
        Self { start, end, goal_column: None, reversed }
    }

    pub fn is_empty(&self) -> bool { self.start.offset == self.end.offset }
    pub fn head(&self) -> usize { if self.reversed { self.start.offset } else { self.end.offset } }
    pub fn tail(&self) -> usize { if self.reversed { self.end.offset } else { self.start.offset } }
    pub fn min_offset(&self) -> usize { self.start.offset.min(self.end.offset) }
    pub fn max_offset(&self) -> usize { self.start.offset.max(self.end.offset) }

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
        Self { selections: vec![Selection::caret(offset)] }
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

    pub fn len(&self) -> usize { self.selections.len() }
    pub fn is_empty(&self) -> bool { self.selections.is_empty() }
    pub fn primary(&self) -> Option<&Selection> { self.selections.last() }
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
}
