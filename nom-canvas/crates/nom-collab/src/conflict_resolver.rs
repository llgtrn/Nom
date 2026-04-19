/// Kinds of conflict that can arise during collaborative editing.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConflictKind {
    Insert,
    Delete,
    Move,
    Update,
}

impl ConflictKind {
    /// Returns true for structural operations that alter document topology.
    pub fn is_structural(&self) -> bool {
        matches!(self, ConflictKind::Insert | ConflictKind::Delete | ConflictKind::Move)
    }

    /// Relative severity: higher means more disruptive.
    pub fn severity(&self) -> u8 {
        match self {
            ConflictKind::Insert => 1,
            ConflictKind::Delete => 3,
            ConflictKind::Move => 2,
            ConflictKind::Update => 1,
        }
    }
}

/// Which side of a collaboration introduced the conflicting operation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConflictSide {
    Local,
    Remote,
    Both,
}

impl ConflictSide {
    /// Returns true only when both sides are involved.
    pub fn is_bilateral(&self) -> bool {
        matches!(self, ConflictSide::Both)
    }

    /// Human-readable label for the side.
    pub fn side_label(&self) -> &'static str {
        match self {
            ConflictSide::Local => "local",
            ConflictSide::Remote => "remote",
            ConflictSide::Both => "both",
        }
    }
}

/// A detected conflict between two peers.
#[derive(Debug, Clone)]
pub struct Conflict {
    pub id: u32,
    pub kind: ConflictKind,
    pub side: ConflictSide,
    pub position: u64,
}

impl Conflict {
    /// A conflict is critical when its kind severity is 3 or above.
    pub fn is_critical(&self) -> bool {
        self.kind.severity() >= 3
    }

    /// Stable string key for deduplication or logging.
    pub fn conflict_key(&self) -> String {
        format!("{}:{}:{}", self.id, self.kind.severity(), self.side.side_label())
    }
}

/// Strategy used to resolve a conflict.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ResolutionStrategy {
    TakeLocal,
    TakeRemote,
    Merge,
    Skip,
}

impl ResolutionStrategy {
    /// Returns true for strategies that require no manual intervention.
    pub fn is_automatic(&self) -> bool {
        matches!(
            self,
            ResolutionStrategy::TakeLocal | ResolutionStrategy::TakeRemote | ResolutionStrategy::Skip
        )
    }

    /// Canonical name used in logs and reports.
    pub fn strategy_name(&self) -> &'static str {
        match self {
            ResolutionStrategy::TakeLocal => "take_local",
            ResolutionStrategy::TakeRemote => "take_remote",
            ResolutionStrategy::Merge => "merge",
            ResolutionStrategy::Skip => "skip",
        }
    }
}

/// Applies resolution strategies to conflicts and tracks outcomes.
#[derive(Debug, Default)]
pub struct ConflictResolver {
    pub resolved: u32,
    pub skipped: u32,
}

impl ConflictResolver {
    pub fn new() -> Self {
        Self { resolved: 0, skipped: 0 }
    }

    /// Apply `strategy` to `conflict`. Returns true when the conflict was resolved,
    /// false when it was skipped.
    pub fn resolve(&mut self, _conflict: &Conflict, strategy: &ResolutionStrategy) -> bool {
        if matches!(strategy, ResolutionStrategy::Skip) {
            self.skipped += 1;
            false
        } else {
            self.resolved += 1;
            true
        }
    }

    /// Fraction of processed conflicts that were resolved (not skipped).
    /// Returns 0.0 when no conflicts have been processed yet.
    pub fn resolution_rate(&self) -> f64 {
        let total = self.resolved + self.skipped;
        if total == 0 {
            0.0
        } else {
            self.resolved as f64 / total as f64
        }
    }

    /// Pick the best automatic strategy for a batch of conflicts.
    pub fn auto_resolve(conflicts: &[Conflict]) -> ResolutionStrategy {
        if conflicts.iter().any(|c| c.is_critical()) {
            return ResolutionStrategy::TakeRemote;
        }
        if conflicts.iter().all(|c| !c.kind.is_structural()) {
            return ResolutionStrategy::Skip;
        }
        ResolutionStrategy::TakeLocal
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn conflict_kind_is_structural() {
        assert!(ConflictKind::Insert.is_structural());
        assert!(ConflictKind::Delete.is_structural());
        assert!(ConflictKind::Move.is_structural());
        assert!(!ConflictKind::Update.is_structural());
    }

    #[test]
    fn conflict_kind_severity_delete() {
        assert_eq!(ConflictKind::Delete.severity(), 3);
    }

    #[test]
    fn conflict_side_is_bilateral() {
        assert!(ConflictSide::Both.is_bilateral());
        assert!(!ConflictSide::Local.is_bilateral());
        assert!(!ConflictSide::Remote.is_bilateral());
    }

    #[test]
    fn conflict_side_label() {
        assert_eq!(ConflictSide::Local.side_label(), "local");
        assert_eq!(ConflictSide::Remote.side_label(), "remote");
        assert_eq!(ConflictSide::Both.side_label(), "both");
    }

    #[test]
    fn conflict_is_critical_true() {
        let c = Conflict { id: 1, kind: ConflictKind::Delete, side: ConflictSide::Remote, position: 0 };
        assert!(c.is_critical());
    }

    #[test]
    fn conflict_is_critical_false() {
        let c = Conflict { id: 2, kind: ConflictKind::Update, side: ConflictSide::Local, position: 5 };
        assert!(!c.is_critical());
    }

    #[test]
    fn conflict_key_format() {
        let c = Conflict { id: 7, kind: ConflictKind::Move, side: ConflictSide::Both, position: 10 };
        assert_eq!(c.conflict_key(), "7:2:both");
    }

    #[test]
    fn resolution_strategy_is_automatic() {
        assert!(ResolutionStrategy::TakeLocal.is_automatic());
        assert!(ResolutionStrategy::TakeRemote.is_automatic());
        assert!(ResolutionStrategy::Skip.is_automatic());
        assert!(!ResolutionStrategy::Merge.is_automatic());
    }

    #[test]
    fn conflict_resolver_skip_vs_resolve_and_rate() {
        let mut resolver = ConflictResolver::new();
        let skip_conflict = Conflict { id: 1, kind: ConflictKind::Update, side: ConflictSide::Local, position: 0 };
        let real_conflict = Conflict { id: 2, kind: ConflictKind::Insert, side: ConflictSide::Remote, position: 1 };

        let skipped = resolver.resolve(&skip_conflict, &ResolutionStrategy::Skip);
        assert!(!skipped);
        assert_eq!(resolver.skipped, 1);
        assert_eq!(resolver.resolved, 0);

        let resolved = resolver.resolve(&real_conflict, &ResolutionStrategy::TakeLocal);
        assert!(resolved);
        assert_eq!(resolver.resolved, 1);

        // 1 resolved out of 2 total = 0.5
        let rate = resolver.resolution_rate();
        assert!((rate - 0.5).abs() < f64::EPSILON);
    }
}
