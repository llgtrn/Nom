#[derive(Debug, Clone, Copy, PartialEq)]
pub enum RuleSeverity {
    Error,
    Warning,
    Info,
}

#[derive(Debug, Clone)]
pub struct UxPattern {
    pub id: String,
    pub name: String,
    pub description: String,
    pub tags: Vec<String>,
}

impl UxPattern {
    pub fn new(id: &str, name: &str) -> Self {
        Self {
            id: id.to_owned(),
            name: name.to_owned(),
            description: String::new(),
            tags: Vec::new(),
        }
    }

    pub fn with_tag(mut self, tag: &str) -> Self {
        self.tags.push(tag.to_owned());
        self
    }

    pub fn matches_tag(&self, tag: &str) -> bool {
        self.tags.iter().any(|t| t == tag)
    }
}

#[derive(Debug, Clone)]
pub struct DesignRule {
    pub id: String,
    pub rule: String,
    pub severity: RuleSeverity,
}

impl DesignRule {
    pub fn new(id: &str, rule: &str, severity: RuleSeverity) -> Self {
        Self {
            id: id.to_owned(),
            rule: rule.to_owned(),
            severity,
        }
    }

    pub fn is_blocking(&self) -> bool {
        self.severity == RuleSeverity::Error
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_pattern_has_empty_tags() {
        let p = UxPattern::new("p1", "Card Layout");
        assert_eq!(p.id, "p1");
        assert_eq!(p.name, "Card Layout");
        assert!(p.tags.is_empty());
    }

    #[test]
    fn with_tag_and_matches_tag() {
        let p = UxPattern::new("p2", "Grid")
            .with_tag("layout")
            .with_tag("responsive");
        assert!(p.matches_tag("layout"));
        assert!(p.matches_tag("responsive"));
        assert!(!p.matches_tag("motion"));
    }

    #[test]
    fn design_rule_blocking() {
        let blocking = DesignRule::new("r1", "min touch target 44px", RuleSeverity::Error);
        let advisory = DesignRule::new("r2", "prefer system font", RuleSeverity::Warning);
        assert!(blocking.is_blocking());
        assert!(!advisory.is_blocking());
    }
}
