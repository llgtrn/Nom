#![deny(unsafe_code)]

/// An individual action item that can appear in a header panel slot.
#[derive(Debug, Clone)]
pub enum HeaderAction {
    Button { label: String },
    Icon { name: String },
    Separator,
}

/// A header bar with left actions, a center breadcrumb, and right actions.
#[derive(Debug, Clone)]
pub struct HeaderPanel {
    pub left: Vec<HeaderAction>,
    pub center: String,
    pub right: Vec<HeaderAction>,
}

impl HeaderPanel {
    /// Create a new header panel with the given breadcrumb text.
    pub fn new(center: &str) -> Self {
        Self {
            left: Vec::new(),
            center: center.to_string(),
            right: Vec::new(),
        }
    }

    /// Append an action to the left slot.
    pub fn push_left(mut self, action: HeaderAction) -> Self {
        self.left.push(action);
        self
    }

    /// Append an action to the right slot.
    pub fn push_right(mut self, action: HeaderAction) -> Self {
        self.right.push(action);
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_has_empty_slots_and_center() {
        let hp = HeaderPanel::new("Project / File");
        assert_eq!(hp.center, "Project / File");
        assert!(hp.left.is_empty());
        assert!(hp.right.is_empty());
    }

    #[test]
    fn push_actions_into_slots() {
        let hp = HeaderPanel::new("Root")
            .push_left(HeaderAction::Icon { name: "back".to_string() })
            .push_right(HeaderAction::Button { label: "Share".to_string() })
            .push_right(HeaderAction::Separator);
        assert_eq!(hp.left.len(), 1);
        assert_eq!(hp.right.len(), 2);
    }

    #[test]
    fn action_count_reflects_pushes() {
        let hp = HeaderPanel::new("x")
            .push_left(HeaderAction::Separator)
            .push_left(HeaderAction::Separator)
            .push_right(HeaderAction::Separator);
        assert_eq!(hp.left.len() + hp.right.len(), 3);
    }
}
