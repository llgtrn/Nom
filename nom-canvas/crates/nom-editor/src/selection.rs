/// A single point (line + column) that anchors a selection.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SelectionAnchor {
    pub line: u32,
    pub col: u32,
}

impl SelectionAnchor {
    pub fn new(line: u32, col: u32) -> Self {
        Self { line, col }
    }
}

/// A contiguous range of text defined by two anchors.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SelectionRange {
    pub start: SelectionAnchor,
    pub end: SelectionAnchor,
}

impl SelectionRange {
    pub fn new(start: SelectionAnchor, end: SelectionAnchor) -> Self {
        Self { start, end }
    }

    /// Returns `true` when the selection covers no text (start == end).
    pub fn is_empty(&self) -> bool {
        self.start == self.end
    }

    /// Number of lines spanned by the selection (inclusive on both ends).
    pub fn line_count(&self) -> u32 {
        self.end.line.saturating_sub(self.start.line) + 1
    }

    /// Returns `true` if the given line falls within the selection range.
    pub fn contains_line(&self, line: u32) -> bool {
        line >= self.start.line && line <= self.end.line
    }
}

/// Manages a collection of (possibly multi-cursor) selections.
#[derive(Debug, Default)]
pub struct SelectionManager {
    selections: Vec<SelectionRange>,
}

impl SelectionManager {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add(&mut self, sel: SelectionRange) {
        self.selections.push(sel);
    }

    pub fn clear(&mut self) {
        self.selections.clear();
    }

    pub fn count(&self) -> usize {
        self.selections.len()
    }

    /// Returns `true` when more than one selection is active.
    pub fn has_multi_selection(&self) -> bool {
        self.selections.len() > 1
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn anchor_new() {
        let a = SelectionAnchor::new(3, 7);
        assert_eq!(a.line, 3);
        assert_eq!(a.col, 7);
    }

    #[test]
    fn selection_new() {
        let start = SelectionAnchor::new(0, 0);
        let end = SelectionAnchor::new(2, 5);
        let sel = SelectionRange::new(start, end);
        assert_eq!(sel.start.line, 0);
        assert_eq!(sel.end.col, 5);
    }

    #[test]
    fn selection_is_empty() {
        let a = SelectionAnchor::new(1, 4);
        let empty = SelectionRange::new(a, a);
        assert!(empty.is_empty());

        let b = SelectionAnchor::new(1, 5);
        let non_empty = SelectionRange::new(a, b);
        assert!(!non_empty.is_empty());
    }

    #[test]
    fn selection_line_count() {
        let sel = SelectionRange::new(SelectionAnchor::new(2, 0), SelectionAnchor::new(5, 0));
        assert_eq!(sel.line_count(), 4);

        let single = SelectionRange::new(SelectionAnchor::new(3, 0), SelectionAnchor::new(3, 10));
        assert_eq!(single.line_count(), 1);
    }

    #[test]
    fn selection_contains_line() {
        let sel = SelectionRange::new(SelectionAnchor::new(2, 0), SelectionAnchor::new(5, 0));
        assert!(sel.contains_line(2));
        assert!(sel.contains_line(4));
        assert!(sel.contains_line(5));
        assert!(!sel.contains_line(1));
        assert!(!sel.contains_line(6));
    }

    #[test]
    fn manager_add() {
        let mut mgr = SelectionManager::new();
        mgr.add(SelectionRange::new(
            SelectionAnchor::new(0, 0),
            SelectionAnchor::new(0, 5),
        ));
        assert_eq!(mgr.count(), 1);
    }

    #[test]
    fn manager_clear() {
        let mut mgr = SelectionManager::new();
        mgr.add(SelectionRange::new(
            SelectionAnchor::new(0, 0),
            SelectionAnchor::new(1, 0),
        ));
        mgr.clear();
        assert_eq!(mgr.count(), 0);
    }

    #[test]
    fn manager_multi() {
        let mut mgr = SelectionManager::new();
        assert!(!mgr.has_multi_selection());
        mgr.add(SelectionRange::new(
            SelectionAnchor::new(0, 0),
            SelectionAnchor::new(0, 3),
        ));
        assert!(!mgr.has_multi_selection());
        mgr.add(SelectionRange::new(
            SelectionAnchor::new(2, 0),
            SelectionAnchor::new(2, 3),
        ));
        assert!(mgr.has_multi_selection());
    }
}
