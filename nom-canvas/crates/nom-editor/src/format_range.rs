/// Formatting kind applied to a text range.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FormatKind {
    Bold,
    Italic,
    Code,
    Link,
    Heading(u8),
}

impl FormatKind {
    /// Returns true for inline formats (Bold, Italic, Code, Link).
    pub fn is_inline(&self) -> bool {
        matches!(self, FormatKind::Bold | FormatKind::Italic | FormatKind::Code | FormatKind::Link)
    }

    /// Priority rank: lower number = higher priority.
    pub fn rank(&self) -> u8 {
        match self {
            FormatKind::Bold => 0,
            FormatKind::Italic => 1,
            FormatKind::Code => 2,
            FormatKind::Link => 3,
            FormatKind::Heading(n) => 10 + n,
        }
    }
}

/// A byte-offset range within a text buffer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TextRange {
    pub start: usize,
    pub end: usize,
}

impl TextRange {
    pub fn len(&self) -> usize {
        self.end.saturating_sub(self.start)
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub fn overlaps(&self, other: &TextRange) -> bool {
        self.start < other.end && self.end > other.start
    }

    pub fn contains(&self, pos: usize) -> bool {
        pos >= self.start && pos < self.end
    }
}

/// A formatting kind applied over a text range.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FormatRange {
    pub range: TextRange,
    pub kind: FormatKind,
}

impl FormatRange {
    pub fn is_inline(&self) -> bool {
        self.kind.is_inline()
    }

    pub fn rank(&self) -> u8 {
        self.kind.rank()
    }

    /// Returns true if this format range covers the given position.
    pub fn covers(&self, pos: usize) -> bool {
        self.range.contains(pos)
    }
}

/// A collection of format ranges over a document.
#[derive(Debug, Default)]
pub struct FormatMap {
    pub entries: Vec<FormatRange>,
}

impl FormatMap {
    pub fn new() -> Self {
        Self { entries: Vec::new() }
    }

    pub fn add(&mut self, fr: FormatRange) {
        self.entries.push(fr);
    }

    /// Returns all format ranges that cover the given position.
    pub fn formats_at(&self, pos: usize) -> Vec<&FormatRange> {
        self.entries.iter().filter(|fr| fr.covers(pos)).collect()
    }

    /// Returns the number of inline format ranges in this map.
    pub fn inline_count(&self) -> usize {
        self.entries.iter().filter(|fr| fr.is_inline()).count()
    }
}

/// Utility for applying format queries against a FormatMap.
pub struct FormatApplier;

impl FormatApplier {
    /// Returns all format ranges whose range overlaps the given text range.
    pub fn overlapping<'a>(map: &'a FormatMap, range: &TextRange) -> Vec<&'a FormatRange> {
        map.entries.iter().filter(|fr| fr.range.overlaps(range)).collect()
    }

    /// Returns the minimum (highest-priority) rank among the given format ranges.
    /// Returns None if the slice is empty.
    pub fn highest_rank(fmts: &[&FormatRange]) -> Option<u8> {
        fmts.iter().map(|fr| fr.rank()).min()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn format_kind_is_inline_all_variants() {
        assert!(FormatKind::Bold.is_inline());
        assert!(FormatKind::Italic.is_inline());
        assert!(FormatKind::Code.is_inline());
        assert!(FormatKind::Link.is_inline());
        assert!(!FormatKind::Heading(1).is_inline());
        assert!(!FormatKind::Heading(6).is_inline());
    }

    #[test]
    fn format_kind_rank_ordering() {
        assert!(FormatKind::Bold.rank() < FormatKind::Italic.rank());
        assert!(FormatKind::Italic.rank() < FormatKind::Code.rank());
        assert!(FormatKind::Code.rank() < FormatKind::Link.rank());
        assert_eq!(FormatKind::Heading(1).rank(), 11);
        assert_eq!(FormatKind::Heading(6).rank(), 16);
        assert!(FormatKind::Link.rank() < FormatKind::Heading(1).rank());
    }

    #[test]
    fn text_range_overlaps_true_and_false() {
        let a = TextRange { start: 0, end: 10 };
        let b = TextRange { start: 5, end: 15 };
        let c = TextRange { start: 10, end: 20 };
        assert!(a.overlaps(&b));
        assert!(b.overlaps(&a));
        assert!(!a.overlaps(&c), "touching at boundary should not overlap");
        assert!(!c.overlaps(&a));
    }

    #[test]
    fn text_range_contains() {
        let r = TextRange { start: 3, end: 8 };
        assert!(r.contains(3));
        assert!(r.contains(7));
        assert!(!r.contains(8), "end is exclusive");
        assert!(!r.contains(2));
    }

    #[test]
    fn format_range_covers() {
        let fr = FormatRange {
            range: TextRange { start: 5, end: 10 },
            kind: FormatKind::Bold,
        };
        assert!(fr.covers(5));
        assert!(fr.covers(9));
        assert!(!fr.covers(10));
        assert!(!fr.covers(4));
    }

    #[test]
    fn format_map_formats_at() {
        let mut map = FormatMap::new();
        map.add(FormatRange { range: TextRange { start: 0, end: 5 }, kind: FormatKind::Bold });
        map.add(FormatRange { range: TextRange { start: 3, end: 8 }, kind: FormatKind::Italic });
        map.add(FormatRange { range: TextRange { start: 10, end: 15 }, kind: FormatKind::Code });

        let at4 = map.formats_at(4);
        assert_eq!(at4.len(), 2);
        let at10 = map.formats_at(10);
        assert_eq!(at10.len(), 1);
        let at20 = map.formats_at(20);
        assert!(at20.is_empty());
    }

    #[test]
    fn format_map_inline_count() {
        let mut map = FormatMap::new();
        map.add(FormatRange { range: TextRange { start: 0, end: 5 }, kind: FormatKind::Bold });
        map.add(FormatRange { range: TextRange { start: 0, end: 5 }, kind: FormatKind::Heading(2) });
        map.add(FormatRange { range: TextRange { start: 0, end: 5 }, kind: FormatKind::Italic });
        assert_eq!(map.inline_count(), 2);
    }

    #[test]
    fn format_applier_overlapping() {
        let mut map = FormatMap::new();
        map.add(FormatRange { range: TextRange { start: 0, end: 10 }, kind: FormatKind::Bold });
        map.add(FormatRange { range: TextRange { start: 8, end: 20 }, kind: FormatKind::Code });
        map.add(FormatRange { range: TextRange { start: 20, end: 30 }, kind: FormatKind::Link });

        let query = TextRange { start: 5, end: 12 };
        let result = FormatApplier::overlapping(&map, &query);
        assert_eq!(result.len(), 2);

        let no_overlap = TextRange { start: 25, end: 35 };
        let result2 = FormatApplier::overlapping(&map, &no_overlap);
        assert_eq!(result2.len(), 1);
    }

    #[test]
    fn format_applier_highest_rank_none_and_some() {
        assert_eq!(FormatApplier::highest_rank(&[]), None);

        let fr_bold = FormatRange { range: TextRange { start: 0, end: 1 }, kind: FormatKind::Bold };
        let fr_heading = FormatRange {
            range: TextRange { start: 0, end: 1 },
            kind: FormatKind::Heading(3),
        };
        let fr_italic = FormatRange { range: TextRange { start: 0, end: 1 }, kind: FormatKind::Italic };

        let fmts = vec![&fr_heading, &fr_italic, &fr_bold];
        assert_eq!(FormatApplier::highest_rank(&fmts), Some(0)); // Bold has rank 0
    }
}
