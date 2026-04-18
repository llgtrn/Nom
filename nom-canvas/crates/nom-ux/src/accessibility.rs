#[derive(Debug, Clone, PartialEq, Eq)]
pub enum A11yRole {
    Button,
    TextInput,
    Checkbox,
    Dialog,
    List,
    ListItem,
    Navigation,
    Main,
}

impl A11yRole {
    pub fn aria_role(&self) -> &str {
        match self {
            A11yRole::Button => "button",
            A11yRole::TextInput => "textbox",
            A11yRole::Checkbox => "checkbox",
            A11yRole::Dialog => "dialog",
            A11yRole::List => "list",
            A11yRole::ListItem => "listitem",
            A11yRole::Navigation => "navigation",
            A11yRole::Main => "main",
        }
    }

    pub fn is_interactive(&self) -> bool {
        matches!(self, A11yRole::Button | A11yRole::TextInput | A11yRole::Checkbox)
    }
}

#[derive(Debug, Clone)]
pub struct A11yNode {
    pub id: u64,
    pub role: A11yRole,
    pub label: Option<String>,
    pub tab_index: i32,
}

impl A11yNode {
    pub fn new(id: u64, role: A11yRole) -> Self {
        Self { id, role, label: None, tab_index: -1 }
    }

    pub fn with_label(mut self, label: impl Into<String>) -> Self {
        self.label = Some(label.into());
        self
    }

    pub fn focusable(mut self) -> Self {
        self.tab_index = 0;
        self
    }

    pub fn is_focusable(&self) -> bool {
        self.tab_index >= 0
    }
}

#[derive(Debug, Clone)]
pub struct A11yAuditResult {
    pub violations: Vec<String>,
}

impl A11yAuditResult {
    pub fn new() -> Self {
        Self { violations: Vec::new() }
    }

    pub fn add_violation(&mut self, msg: impl Into<String>) {
        self.violations.push(msg.into());
    }

    pub fn is_pass(&self) -> bool {
        self.violations.is_empty()
    }

    pub fn violation_count(&self) -> usize {
        self.violations.len()
    }
}

impl Default for A11yAuditResult {
    fn default() -> Self {
        Self::new()
    }
}

pub struct A11yAuditor;

impl A11yAuditor {
    pub fn new() -> Self {
        Self
    }

    pub fn audit_node(node: &A11yNode) -> A11yAuditResult {
        let mut result = A11yAuditResult::new();
        if node.role.is_interactive() && node.label.is_none() {
            result.add_violation("interactive element missing label");
        }
        if node.role == A11yRole::Button && !node.is_focusable() {
            result.add_violation("button must be focusable");
        }
        result
    }

    pub fn audit_tree(nodes: &[A11yNode]) -> A11yAuditResult {
        let mut result = A11yAuditResult::new();
        for node in nodes {
            let node_result = Self::audit_node(node);
            for v in node_result.violations {
                result.add_violation(v);
            }
        }
        result
    }
}

impl Default for A11yAuditor {
    fn default() -> Self {
        Self::new()
    }
}

pub struct KeyboardNav;

impl KeyboardNav {
    pub fn tab_order(nodes: &[A11yNode]) -> Vec<u64> {
        let mut focusable: Vec<&A11yNode> = nodes.iter().filter(|n| n.is_focusable()).collect();
        focusable.sort_by_key(|n| n.tab_index);
        focusable.iter().map(|n| n.id).collect()
    }

    pub fn next_focus(nodes: &[A11yNode], current_id: u64) -> Option<u64> {
        let order = Self::tab_order(nodes);
        let pos = order.iter().position(|&id| id == current_id)?;
        order.into_iter().nth(pos + 1)
    }

    pub fn prev_focus(nodes: &[A11yNode], current_id: u64) -> Option<u64> {
        let order = Self::tab_order(nodes);
        let pos = order.iter().position(|&id| id == current_id)?;
        if pos == 0 {
            return None;
        }
        order.into_iter().nth(pos - 1)
    }
}

#[cfg(test)]
mod accessibility_tests {
    use super::*;

    #[test]
    fn test_aria_role() {
        assert_eq!(A11yRole::Button.aria_role(), "button");
        assert_eq!(A11yRole::TextInput.aria_role(), "textbox");
        assert_eq!(A11yRole::Checkbox.aria_role(), "checkbox");
        assert_eq!(A11yRole::Dialog.aria_role(), "dialog");
        assert_eq!(A11yRole::List.aria_role(), "list");
        assert_eq!(A11yRole::ListItem.aria_role(), "listitem");
        assert_eq!(A11yRole::Navigation.aria_role(), "navigation");
        assert_eq!(A11yRole::Main.aria_role(), "main");
    }

    #[test]
    fn test_is_interactive() {
        assert!(A11yRole::Button.is_interactive());
        assert!(A11yRole::TextInput.is_interactive());
        assert!(A11yRole::Checkbox.is_interactive());
        assert!(!A11yRole::Dialog.is_interactive());
        assert!(!A11yRole::List.is_interactive());
        assert!(!A11yRole::Navigation.is_interactive());
        assert!(!A11yRole::Main.is_interactive());
    }

    #[test]
    fn test_node_is_focusable_after_focusable() {
        let node = A11yNode::new(1, A11yRole::Button).focusable();
        assert!(node.is_focusable());
        assert_eq!(node.tab_index, 0);

        let non_focusable = A11yNode::new(2, A11yRole::Button);
        assert!(!non_focusable.is_focusable());
    }

    #[test]
    fn test_audit_node_missing_label_violation() {
        let node = A11yNode::new(1, A11yRole::Button).focusable();
        let result = A11yAuditor::audit_node(&node);
        assert!(!result.is_pass());
        assert!(result.violations.iter().any(|v| v.contains("missing label")));
    }

    #[test]
    fn test_audit_node_pass_with_label() {
        let node = A11yNode::new(1, A11yRole::Button)
            .with_label("Submit")
            .focusable();
        let result = A11yAuditor::audit_node(&node);
        assert!(result.is_pass());
    }

    #[test]
    fn test_audit_tree_collects_all_violations() {
        let nodes = vec![
            A11yNode::new(1, A11yRole::Button).focusable(),
            A11yNode::new(2, A11yRole::TextInput).focusable(),
            A11yNode::new(3, A11yRole::Main),
        ];
        let result = A11yAuditor::audit_tree(&nodes);
        assert_eq!(result.violation_count(), 2);
    }

    #[test]
    fn test_tab_order_returns_only_focusable() {
        let nodes = vec![
            A11yNode::new(1, A11yRole::Main),
            A11yNode::new(2, A11yRole::Button).focusable(),
            A11yNode::new(3, A11yRole::TextInput).focusable(),
        ];
        let order = KeyboardNav::tab_order(&nodes);
        assert_eq!(order, vec![2, 3]);
    }

    #[test]
    fn test_next_focus() {
        let nodes = vec![
            A11yNode::new(10, A11yRole::Button).focusable(),
            A11yNode::new(20, A11yRole::TextInput).focusable(),
            A11yNode::new(30, A11yRole::Checkbox).focusable(),
        ];
        assert_eq!(KeyboardNav::next_focus(&nodes, 10), Some(20));
        assert_eq!(KeyboardNav::next_focus(&nodes, 20), Some(30));
        assert_eq!(KeyboardNav::next_focus(&nodes, 30), None);
    }

    #[test]
    fn test_prev_focus() {
        let nodes = vec![
            A11yNode::new(10, A11yRole::Button).focusable(),
            A11yNode::new(20, A11yRole::TextInput).focusable(),
            A11yNode::new(30, A11yRole::Checkbox).focusable(),
        ];
        assert_eq!(KeyboardNav::prev_focus(&nodes, 30), Some(20));
        assert_eq!(KeyboardNav::prev_focus(&nodes, 20), Some(10));
        assert_eq!(KeyboardNav::prev_focus(&nodes, 10), None);
    }
}
