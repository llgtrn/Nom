/// Describes the kind of change applied to a scene node during diffing.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DiffKind {
    Added,
    Removed,
    Modified,
    Reordered,
}

impl DiffKind {
    /// Returns `true` for Added, Removed, or Reordered — changes that alter
    /// the structural shape of the scene tree, not just node data.
    pub fn is_structural(&self) -> bool {
        matches!(self, DiffKind::Added | DiffKind::Removed | DiffKind::Reordered)
    }

    /// Stable numeric code for this kind: Added=0, Removed=1, Modified=2, Reordered=3.
    pub fn kind_code(&self) -> u8 {
        match self {
            DiffKind::Added => 0,
            DiffKind::Removed => 1,
            DiffKind::Modified => 2,
            DiffKind::Reordered => 3,
        }
    }
}

/// Opaque identifier for a scene node. A value of 0 is reserved and invalid.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SceneNodeId(pub u64);

impl SceneNodeId {
    /// Returns `true` when the id is non-zero (i.e. refers to a real node).
    pub fn is_valid(&self) -> bool {
        self.0 != 0
    }

    /// Human-readable representation used in diagnostics and summaries.
    pub fn id_str(&self) -> String {
        format!("node:{}", self.0)
    }
}

/// A single change record produced by the scene diffing pass.
#[derive(Debug, Clone)]
pub struct SceneDiff {
    /// Which node this diff refers to.
    pub node_id: SceneNodeId,
    /// What kind of change occurred.
    pub kind: DiffKind,
    /// Depth of the node in the scene tree (0 = root / leaf with no children).
    pub depth: u32,
}

impl SceneDiff {
    /// Returns `true` when the node sits at depth 0 (a leaf in this context).
    pub fn is_leaf(&self) -> bool {
        self.depth == 0
    }

    /// Compact one-line summary suitable for logging.
    /// Format: `[{kind_code}] node:{id} d={depth}`
    pub fn summary(&self) -> String {
        format!("[{}] node:{} d={}", self.kind.kind_code(), self.node_id.0, self.depth)
    }
}

/// A bundle of diffs produced for a single incremental scene update.
#[derive(Debug, Clone)]
pub struct ScenePatch {
    /// All diffs included in this patch.
    pub diffs: Vec<SceneDiff>,
    /// Monotonically increasing identifier assigned by the scene builder.
    pub patch_id: u32,
}

impl ScenePatch {
    /// Number of diffs that are structural (Added, Removed, or Reordered).
    pub fn structural_count(&self) -> usize {
        self.diffs.iter().filter(|d| d.kind.is_structural()).count()
    }

    /// Returns `true` when the patch contains no diffs.
    pub fn is_empty(&self) -> bool {
        self.diffs.is_empty()
    }

    /// Returns `true` when at least one diff has `DiffKind::Removed`.
    pub fn has_removals(&self) -> bool {
        self.diffs.iter().any(|d| d.kind == DiffKind::Removed)
    }
}

/// Tracks how many patches have been applied or rejected and computes a
/// running success rate.
#[derive(Debug, Clone)]
pub struct PatchApplier {
    pub applied: u32,
    pub rejected: u32,
}

impl PatchApplier {
    /// Creates a fresh applier with all counters at zero.
    pub fn new() -> Self {
        PatchApplier { applied: 0, rejected: 0 }
    }

    /// Attempts to apply `patch`.  A non-empty patch is accepted (increments
    /// `applied` and returns `true`); an empty patch is rejected (increments
    /// `rejected` and returns `false`).
    pub fn apply(&mut self, patch: &ScenePatch) -> bool {
        if !patch.is_empty() {
            self.applied += 1;
            true
        } else {
            self.rejected += 1;
            false
        }
    }

    /// Fraction of patches that were accepted.  Returns `0.0` when no patches
    /// have been seen yet to avoid division by zero.
    pub fn success_rate(&self) -> f64 {
        let total = self.applied + self.rejected;
        if total == 0 {
            0.0
        } else {
            self.applied as f64 / total as f64
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // --- DiffKind ---

    #[test]
    fn diff_kind_is_structural_for_each_variant() {
        assert!(DiffKind::Added.is_structural());
        assert!(DiffKind::Removed.is_structural());
        assert!(!DiffKind::Modified.is_structural());
        assert!(DiffKind::Reordered.is_structural());
    }

    #[test]
    fn diff_kind_code_values() {
        assert_eq!(DiffKind::Added.kind_code(), 0);
        assert_eq!(DiffKind::Removed.kind_code(), 1);
        assert_eq!(DiffKind::Modified.kind_code(), 2);
        assert_eq!(DiffKind::Reordered.kind_code(), 3);
    }

    // --- SceneNodeId ---

    #[test]
    fn scene_node_id_validity() {
        assert!(!SceneNodeId(0).is_valid());
        assert!(SceneNodeId(1).is_valid());
        assert!(SceneNodeId(u64::MAX).is_valid());
    }

    // --- SceneDiff ---

    #[test]
    fn scene_diff_is_leaf_at_depth_zero() {
        let diff = SceneDiff { node_id: SceneNodeId(1), kind: DiffKind::Added, depth: 0 };
        assert!(diff.is_leaf());
        let diff_deep = SceneDiff { node_id: SceneNodeId(2), kind: DiffKind::Modified, depth: 3 };
        assert!(!diff_deep.is_leaf());
    }

    #[test]
    fn scene_diff_summary_format() {
        let diff = SceneDiff { node_id: SceneNodeId(42), kind: DiffKind::Modified, depth: 5 };
        assert_eq!(diff.summary(), "[2] node:42 d=5");
    }

    // --- ScenePatch ---

    #[test]
    fn scene_patch_structural_count() {
        let patch = ScenePatch {
            patch_id: 1,
            diffs: vec![
                SceneDiff { node_id: SceneNodeId(1), kind: DiffKind::Added, depth: 0 },
                SceneDiff { node_id: SceneNodeId(2), kind: DiffKind::Modified, depth: 1 },
                SceneDiff { node_id: SceneNodeId(3), kind: DiffKind::Reordered, depth: 2 },
            ],
        };
        assert_eq!(patch.structural_count(), 2);
    }

    #[test]
    fn scene_patch_has_removals_detects_removed_diffs() {
        let patch_no_removal = ScenePatch {
            patch_id: 2,
            diffs: vec![
                SceneDiff { node_id: SceneNodeId(10), kind: DiffKind::Added, depth: 0 },
            ],
        };
        assert!(!patch_no_removal.has_removals());

        let patch_with_removal = ScenePatch {
            patch_id: 3,
            diffs: vec![
                SceneDiff { node_id: SceneNodeId(11), kind: DiffKind::Removed, depth: 1 },
            ],
        };
        assert!(patch_with_removal.has_removals());
    }

    // --- PatchApplier ---

    #[test]
    fn patch_applier_apply_empty_patch_is_rejected() {
        let mut applier = PatchApplier::new();
        let empty_patch = ScenePatch { patch_id: 0, diffs: vec![] };
        let accepted = applier.apply(&empty_patch);
        assert!(!accepted);
        assert_eq!(applier.rejected, 1);
        assert_eq!(applier.applied, 0);
    }

    #[test]
    fn patch_applier_success_rate_calculation() {
        let mut applier = PatchApplier::new();
        // Zero patches -> 0.0
        assert_eq!(applier.success_rate(), 0.0);

        let good = ScenePatch {
            patch_id: 1,
            diffs: vec![SceneDiff { node_id: SceneNodeId(1), kind: DiffKind::Added, depth: 0 }],
        };
        let bad = ScenePatch { patch_id: 2, diffs: vec![] };

        applier.apply(&good); // applied=1
        applier.apply(&bad);  // rejected=1
        // 1 / 2 = 0.5
        assert!((applier.success_rate() - 0.5).abs() < f64::EPSILON);
    }
}
