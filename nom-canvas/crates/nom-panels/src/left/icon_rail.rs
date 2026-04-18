#![deny(unsafe_code)]

/// A single item in the icon rail.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IconRailItem {
    pub icon: String,
    pub tooltip: String,
    pub active: bool,
    pub badge: Option<u32>,
}

/// Vertical icon rail displayed on the far-left edge of the canvas.
#[derive(Debug, Clone, Default)]
pub struct IconRail {
    pub items: Vec<IconRailItem>,
    pub active_index: Option<usize>,
}

impl IconRail {
    pub fn new() -> Self {
        Self::default()
    }

    /// Append an item and return self for chaining.
    pub fn push(mut self, item: IconRailItem) -> Self {
        self.items.push(item);
        self
    }

    /// Set the active item by index (no-op if out of range).
    pub fn set_active(mut self, index: usize) -> Self {
        if index < self.items.len() {
            self.active_index = Some(index);
        }
        self
    }

    /// Return a reference to the currently active item, if any.
    pub fn active_item(&self) -> Option<&IconRailItem> {
        self.active_index.and_then(|i| self.items.get(i))
    }

    /// Sum of all badge counts across every item.
    pub fn badge_total(&self) -> u32 {
        self.items.iter().filter_map(|it| it.badge).sum()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_is_empty() {
        let rail = IconRail::new();
        assert!(rail.items.is_empty());
        assert_eq!(rail.active_index, None);
    }

    #[test]
    fn push_items() {
        let rail = IconRail::new()
            .push(IconRailItem {
                icon: "library".to_string(),
                tooltip: "Library".to_string(),
                active: false,
                badge: None,
            })
            .push(IconRailItem {
                icon: "palette".to_string(),
                tooltip: "Node Palette".to_string(),
                active: false,
                badge: Some(3),
            });
        assert_eq!(rail.items.len(), 2);
    }

    #[test]
    fn set_active_updates_index() {
        let rail = IconRail::new()
            .push(IconRailItem {
                icon: "a".to_string(),
                tooltip: "A".to_string(),
                active: false,
                badge: None,
            })
            .push(IconRailItem {
                icon: "b".to_string(),
                tooltip: "B".to_string(),
                active: false,
                badge: None,
            })
            .set_active(1);
        assert_eq!(rail.active_index, Some(1));
        assert_eq!(rail.active_item().unwrap().icon, "b");
    }

    #[test]
    fn badge_total_sums_correctly() {
        let rail = IconRail::new()
            .push(IconRailItem {
                icon: "x".to_string(),
                tooltip: "X".to_string(),
                active: false,
                badge: Some(5),
            })
            .push(IconRailItem {
                icon: "y".to_string(),
                tooltip: "Y".to_string(),
                active: false,
                badge: None,
            })
            .push(IconRailItem {
                icon: "z".to_string(),
                tooltip: "Z".to_string(),
                active: false,
                badge: Some(2),
            });
        assert_eq!(rail.badge_total(), 7);
    }
}
