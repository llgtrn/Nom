//! Text selection with goal column preservation.
#![deny(unsafe_code)]

use crate::anchor::Anchor;

/// Unique identifier for a selection within a `SelectionsCollection`.
pub type SelectionId = u64;

/// Tracks the horizontal intent when moving the cursor vertically.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum SelectionGoal {
    /// No goal stored; recompute from current position.
    None,
    /// Goal expressed as a character column index.
    Column(u32),
    /// Goal expressed as a rendered horizontal pixel position.
    HorizontalPosition(f32),
}

/// A single contiguous selection (or cursor) in a text buffer.
#[derive(Clone, Debug)]
pub struct TextSelection {
    pub id: SelectionId,
    /// Logical start of the selection (lower offset in buffer order when not reversed).
    pub start: Anchor,
    /// Logical end of the selection (higher offset in buffer order when not reversed).
    pub end: Anchor,
    /// When `true`, the cursor (head) is at `start`; the tail is at `end`.
    pub reversed: bool,
    /// Remembered horizontal goal for vertical movement.
    pub goal: SelectionGoal,
}

impl TextSelection {
    /// Create a new forward (non-reversed) selection with no goal.
    pub fn new(id: SelectionId, start: Anchor, end: Anchor) -> Self {
        Self { id, start, end, reversed: false, goal: SelectionGoal::None }
    }

    /// Buffer byte range in ascending order, regardless of `reversed`.
    pub fn range(&self) -> std::ops::Range<usize> {
        let lo = self.start.offset.min(self.end.offset);
        let hi = self.start.offset.max(self.end.offset);
        lo..hi
    }

    /// `true` when start and end coincide (cursor with no selection).
    pub fn is_empty(&self) -> bool {
        self.start == self.end
    }

    /// The active cursor endpoint (moves when extending selection).
    pub fn head(&self) -> Anchor {
        if self.reversed { self.start } else { self.end }
    }

    /// The fixed anchor endpoint (stays put when extending selection).
    pub fn tail(&self) -> Anchor {
        if self.reversed { self.end } else { self.start }
    }

    /// Whether `offset` falls within (or on the boundary of) this selection.
    pub fn contains_offset(&self, offset: usize) -> bool {
        let r = self.range();
        offset >= r.start && offset <= r.end
    }

    /// Whether this selection overlaps with `other` (touching endpoints count).
    pub fn overlaps(&self, other: &TextSelection) -> bool {
        let a = self.range();
        let b = other.range();
        a.start <= b.end && b.start <= a.end
    }

    /// Clear the stored goal (call after any horizontal movement).
    pub fn clear_goal(&mut self) {
        self.goal = SelectionGoal::None;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn anchor(offset: usize) -> Anchor {
        use crate::anchor::Bias;
        Anchor::new(offset, Bias::Left)
    }

    #[test]
    fn new_defaults_reversed_false_goal_none() {
        let s = TextSelection::new(0, anchor(2), anchor(8));
        assert!(!s.reversed);
        assert_eq!(s.goal, SelectionGoal::None);
    }

    #[test]
    fn head_tail_forward() {
        let s = TextSelection::new(1, anchor(3), anchor(7));
        assert_eq!(s.head().offset, 7);
        assert_eq!(s.tail().offset, 3);
    }

    #[test]
    fn head_tail_reversed() {
        let mut s = TextSelection::new(1, anchor(3), anchor(7));
        s.reversed = true;
        assert_eq!(s.head().offset, 3);
        assert_eq!(s.tail().offset, 7);
    }

    #[test]
    fn range_always_ascending() {
        let mut s = TextSelection::new(0, anchor(8), anchor(2));
        s.reversed = true;
        assert_eq!(s.range(), 2..8);
    }

    #[test]
    fn empty_selection() {
        let s = TextSelection::new(0, anchor(5), anchor(5));
        assert!(s.is_empty());
        assert_eq!(s.range(), 5..5);
    }

    #[test]
    fn contains_offset_hit_and_miss() {
        let s = TextSelection::new(0, anchor(4), anchor(9));
        assert!(s.contains_offset(4));
        assert!(s.contains_offset(6));
        assert!(s.contains_offset(9));
        assert!(!s.contains_offset(3));
        assert!(!s.contains_offset(10));
    }

    #[test]
    fn overlaps_touching_counts() {
        let a = TextSelection::new(0, anchor(0), anchor(5));
        let b = TextSelection::new(1, anchor(5), anchor(10));
        assert!(a.overlaps(&b));
        assert!(b.overlaps(&a));
    }

    #[test]
    fn clear_goal_resets() {
        let mut s = TextSelection::new(0, anchor(0), anchor(1));
        s.goal = SelectionGoal::Column(10);
        s.clear_goal();
        assert_eq!(s.goal, SelectionGoal::None);
    }
}
