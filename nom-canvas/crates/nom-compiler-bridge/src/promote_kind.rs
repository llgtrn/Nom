#![deny(unsafe_code)]

/// Lifecycle status for kind promotion workflow
#[derive(Debug, Clone, PartialEq)]
pub enum KindStatus {
    Draft,
    Active,
    Deprecated,
}

/// Promotes a kind from one status to another
pub struct KindPromotion {
    pub kind_name: String,
    pub from: KindStatus,
    pub to: KindStatus,
}

impl KindPromotion {
    /// Create a promotion from Draft → Active
    pub fn new(kind_name: &str) -> Self {
        Self {
            kind_name: kind_name.to_string(),
            from: KindStatus::Draft,
            to: KindStatus::Active,
        }
    }

    /// Returns true when from and to are different
    pub fn is_valid(&self) -> bool {
        self.from != self.to
    }

    /// Returns true when from = Draft and to = Active
    pub fn apply(&self) -> bool {
        self.from == KindStatus::Draft && self.to == KindStatus::Active
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn kind_promotion_new_draft_to_active() {
        let p = KindPromotion::new("my_kind");
        assert_eq!(p.kind_name, "my_kind");
        assert_eq!(p.from, KindStatus::Draft);
        assert_eq!(p.to, KindStatus::Active);
    }

    #[test]
    fn kind_promotion_is_valid() {
        let p = KindPromotion::new("my_kind");
        assert!(p.is_valid());

        // from == to → invalid
        let invalid = KindPromotion {
            kind_name: "x".to_string(),
            from: KindStatus::Active,
            to: KindStatus::Active,
        };
        assert!(!invalid.is_valid());
    }

    #[test]
    fn kind_promotion_apply() {
        let p = KindPromotion::new("my_kind");
        assert!(p.apply());

        // Non-Draft → Active is not a standard promotion
        let non_draft = KindPromotion {
            kind_name: "y".to_string(),
            from: KindStatus::Deprecated,
            to: KindStatus::Active,
        };
        assert!(!non_draft.apply());
    }
}
