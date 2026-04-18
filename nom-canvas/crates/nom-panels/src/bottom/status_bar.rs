#![deny(unsafe_code)]

/// Visual severity of a status bar item.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StatusKind {
    Info,
    Warning,
    Error,
    Success,
}

/// A single item displayed in the status bar.
#[derive(Debug, Clone)]
pub struct StatusItem {
    pub label: String,
    pub kind: StatusKind,
}

impl StatusItem {
    pub fn new(label: impl Into<String>, kind: StatusKind) -> Self {
        Self {
            label: label.into(),
            kind,
        }
    }
}

/// A status bar with left and right item slots.
#[derive(Debug, Clone, Default)]
pub struct StatusBar {
    pub left_items: Vec<StatusItem>,
    pub right_items: Vec<StatusItem>,
}

impl StatusBar {
    pub fn new() -> Self {
        Self::default()
    }

    /// Append an item to the left slot.
    pub fn push_left(mut self, item: StatusItem) -> Self {
        self.left_items.push(item);
        self
    }

    /// Append an item to the right slot.
    pub fn push_right(mut self, item: StatusItem) -> Self {
        self.right_items.push(item);
        self
    }

    /// Count items with [`StatusKind::Error`] across both slots.
    pub fn error_count(&self) -> usize {
        self.left_items
            .iter()
            .chain(self.right_items.iter())
            .filter(|i| i.kind == StatusKind::Error)
            .count()
    }

    /// Count items with [`StatusKind::Warning`] across both slots.
    pub fn warning_count(&self) -> usize {
        self.left_items
            .iter()
            .chain(self.right_items.iter())
            .filter(|i| i.kind == StatusKind::Warning)
            .count()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_has_empty_slots() {
        let sb = StatusBar::new();
        assert!(sb.left_items.is_empty());
        assert!(sb.right_items.is_empty());
    }

    #[test]
    fn push_items_into_slots() {
        let sb = StatusBar::new()
            .push_left(StatusItem::new("Ready", StatusKind::Info))
            .push_right(StatusItem::new("1 warning", StatusKind::Warning));
        assert_eq!(sb.left_items.len(), 1);
        assert_eq!(sb.right_items.len(), 1);
    }

    #[test]
    fn error_count_spans_both_slots() {
        let sb = StatusBar::new()
            .push_left(StatusItem::new("err1", StatusKind::Error))
            .push_right(StatusItem::new("err2", StatusKind::Error))
            .push_right(StatusItem::new("ok", StatusKind::Info));
        assert_eq!(sb.error_count(), 2);
    }

    #[test]
    fn warning_count_spans_both_slots() {
        let sb = StatusBar::new()
            .push_left(StatusItem::new("w1", StatusKind::Warning))
            .push_left(StatusItem::new("err", StatusKind::Error))
            .push_right(StatusItem::new("w2", StatusKind::Warning));
        assert_eq!(sb.warning_count(), 2);
    }
}
