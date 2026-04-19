/// Operation kinds for CRDT history tracking.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CrdtOpKind {
    Insert,
    Delete,
    Update,
    Move,
}

impl CrdtOpKind {
    pub fn op_name(&self) -> &str {
        match self {
            CrdtOpKind::Insert => "insert",
            CrdtOpKind::Delete => "delete",
            CrdtOpKind::Update => "update",
            CrdtOpKind::Move => "move",
        }
    }

    /// All CRDT ops are reversible.
    pub fn is_reversible(&self) -> bool {
        true
    }
}

/// A single CRDT operation with Lamport timestamp and actor identity.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CrdtOp {
    pub id: u64,
    pub kind: CrdtOpKind,
    pub clock: u64,
    pub actor: String,
    pub payload: String,
}

impl CrdtOp {
    pub fn new(
        id: u64,
        kind: CrdtOpKind,
        clock: u64,
        actor: impl Into<String>,
        payload: impl Into<String>,
    ) -> Self {
        Self {
            id,
            kind,
            clock,
            actor: actor.into(),
            payload: payload.into(),
        }
    }
}

/// Append-only log of CRDT operations.
#[derive(Debug, Default)]
pub struct CrdtHistory {
    pub ops: Vec<CrdtOp>,
}

impl CrdtHistory {
    pub fn new() -> Self {
        Self { ops: Vec::new() }
    }

    pub fn append(&mut self, op: CrdtOp) {
        self.ops.push(op);
    }

    pub fn ops_by_actor(&self, actor: &str) -> Vec<&CrdtOp> {
        self.ops.iter().filter(|op| op.actor == actor).collect()
    }

    /// Returns the maximum Lamport clock value seen; 0 if history is empty.
    pub fn max_clock(&self) -> u64 {
        self.ops.iter().map(|op| op.clock).max().unwrap_or(0)
    }

    /// Returns ops whose clock is strictly greater than `clock`.
    pub fn ops_since(&self, clock: u64) -> Vec<&CrdtOp> {
        self.ops.iter().filter(|op| op.clock > clock).collect()
    }
}

/// Detects and resolves conflicts between concurrent CRDT operations.
pub struct ConflictResolver;

impl ConflictResolver {
    pub fn new() -> Self {
        Self
    }

    /// Returns pairs of op IDs that share the same clock value but have different actors.
    /// Such pairs represent concurrent (potentially conflicting) operations.
    pub fn detect_conflicts(ops: &[CrdtOp]) -> Vec<(u64, u64)> {
        let mut conflicts = Vec::new();
        for i in 0..ops.len() {
            for j in (i + 1)..ops.len() {
                if ops[i].clock == ops[j].clock && ops[i].actor != ops[j].actor {
                    conflicts.push((ops[i].id, ops[j].id));
                }
            }
        }
        conflicts
    }

    /// For each actor, retains only the op with the highest clock (last-write-wins).
    pub fn resolve_last_write_wins(ops: &[CrdtOp]) -> Vec<&CrdtOp> {
        use std::collections::HashMap;
        let mut best: HashMap<&str, &CrdtOp> = HashMap::new();
        for op in ops {
            let entry = best.entry(op.actor.as_str()).or_insert(op);
            if op.clock > entry.clock {
                *entry = op;
            }
        }
        let mut result: Vec<&CrdtOp> = best.into_values().collect();
        result.sort_by_key(|op| op.id);
        result
    }
}

impl Default for ConflictResolver {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod crdt_history_tests {
    use super::*;

    #[test]
    fn test_op_kind_is_reversible() {
        assert!(CrdtOpKind::Insert.is_reversible());
        assert!(CrdtOpKind::Delete.is_reversible());
        assert!(CrdtOpKind::Update.is_reversible());
        assert!(CrdtOpKind::Move.is_reversible());
    }

    #[test]
    fn test_history_append_increments_len() {
        let mut h = CrdtHistory::new();
        assert_eq!(h.ops.len(), 0);
        h.append(CrdtOp::new(1, CrdtOpKind::Insert, 1, "alice", "a"));
        assert_eq!(h.ops.len(), 1);
        h.append(CrdtOp::new(2, CrdtOpKind::Delete, 2, "bob", "b"));
        assert_eq!(h.ops.len(), 2);
    }

    #[test]
    fn test_ops_by_actor_filters_correctly() {
        let mut h = CrdtHistory::new();
        h.append(CrdtOp::new(1, CrdtOpKind::Insert, 1, "alice", "x"));
        h.append(CrdtOp::new(2, CrdtOpKind::Update, 2, "bob", "y"));
        h.append(CrdtOp::new(3, CrdtOpKind::Delete, 3, "alice", "z"));
        let alice_ops = h.ops_by_actor("alice");
        assert_eq!(alice_ops.len(), 2);
        assert!(alice_ops.iter().all(|op| op.actor == "alice"));
    }

    #[test]
    fn test_max_clock_returns_highest() {
        let mut h = CrdtHistory::new();
        h.append(CrdtOp::new(1, CrdtOpKind::Insert, 5, "alice", ""));
        h.append(CrdtOp::new(2, CrdtOpKind::Update, 10, "bob", ""));
        h.append(CrdtOp::new(3, CrdtOpKind::Delete, 3, "alice", ""));
        assert_eq!(h.max_clock(), 10);
    }

    #[test]
    fn test_ops_since_returns_only_newer() {
        let mut h = CrdtHistory::new();
        h.append(CrdtOp::new(1, CrdtOpKind::Insert, 1, "alice", ""));
        h.append(CrdtOp::new(2, CrdtOpKind::Insert, 5, "bob", ""));
        h.append(CrdtOp::new(3, CrdtOpKind::Insert, 10, "alice", ""));
        let newer = h.ops_since(5);
        assert_eq!(newer.len(), 1);
        assert_eq!(newer[0].id, 3);
    }

    #[test]
    fn test_detect_conflicts_finds_concurrent_ops() {
        let ops = vec![
            CrdtOp::new(1, CrdtOpKind::Insert, 7, "alice", ""),
            CrdtOp::new(2, CrdtOpKind::Insert, 7, "bob", ""),
            CrdtOp::new(3, CrdtOpKind::Insert, 8, "alice", ""),
        ];
        let conflicts = ConflictResolver::detect_conflicts(&ops);
        assert_eq!(conflicts.len(), 1);
        assert_eq!(conflicts[0], (1, 2));
    }

    #[test]
    fn test_detect_conflicts_no_conflicts_sequential() {
        let ops = vec![
            CrdtOp::new(1, CrdtOpKind::Insert, 1, "alice", ""),
            CrdtOp::new(2, CrdtOpKind::Insert, 2, "bob", ""),
            CrdtOp::new(3, CrdtOpKind::Insert, 3, "alice", ""),
        ];
        let conflicts = ConflictResolver::detect_conflicts(&ops);
        assert!(conflicts.is_empty());
    }

    #[test]
    fn test_resolve_last_write_wins_per_actor() {
        let ops = vec![
            CrdtOp::new(1, CrdtOpKind::Insert, 1, "alice", "v1"),
            CrdtOp::new(2, CrdtOpKind::Update, 5, "alice", "v2"),
            CrdtOp::new(3, CrdtOpKind::Insert, 2, "bob", "b1"),
            CrdtOp::new(4, CrdtOpKind::Update, 8, "bob", "b2"),
        ];
        let winners = ConflictResolver::resolve_last_write_wins(&ops);
        assert_eq!(winners.len(), 2);
        let alice_winner = winners.iter().find(|op| op.actor == "alice").unwrap();
        assert_eq!(alice_winner.id, 2);
        let bob_winner = winners.iter().find(|op| op.actor == "bob").unwrap();
        assert_eq!(bob_winner.id, 4);
    }

    #[test]
    fn test_crdt_history_new_is_empty() {
        let h = CrdtHistory::new();
        assert!(h.ops.is_empty());
        assert_eq!(h.max_clock(), 0);
    }
}
