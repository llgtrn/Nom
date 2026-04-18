/// Scope of a rename operation.
#[derive(Debug, Clone, PartialEq)]
pub enum RenameScope {
    /// Rename is limited to the current local context (e.g. a single function body).
    Local,
    /// Rename propagates across the entire workspace.
    Global,
    /// Rename is limited to a specific file.
    FileScoped(String),
}

/// A single rename operation: old name → new name within a scope.
#[derive(Debug, Clone)]
pub struct RenameOp {
    pub old_name: String,
    pub new_name: String,
    pub scope: RenameScope,
}

impl RenameOp {
    pub fn new(old_name: impl Into<String>, new_name: impl Into<String>, scope: RenameScope) -> Self {
        Self {
            old_name: old_name.into(),
            new_name: new_name.into(),
            scope,
        }
    }

    /// Returns true when this op has `Local` scope.
    pub fn is_local(&self) -> bool {
        matches!(self.scope, RenameScope::Local)
    }
}

/// A preview of what would change if the pending ops were applied.
#[derive(Debug)]
pub struct RenamePreview {
    pub ops: Vec<RenameOp>,
    pub affected_count: usize,
}

impl RenamePreview {
    pub fn new(ops: Vec<RenameOp>, affected_count: usize) -> Self {
        Self { ops, affected_count }
    }

    /// Returns true when at least one symbol would be affected.
    pub fn has_changes(&self) -> bool {
        self.affected_count > 0
    }

    /// Number of rename ops in this preview.
    pub fn op_count(&self) -> usize {
        self.ops.len()
    }
}

/// Collects rename operations, validates them, and applies them.
#[derive(Debug, Default)]
pub struct WorkspaceRenamer {
    ops: Vec<RenameOp>,
}

impl WorkspaceRenamer {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add_op(&mut self, op: RenameOp) {
        self.ops.push(op);
    }

    /// Validates that no two ops share the same `old_name`.
    /// Returns `Err` with a descriptive message on the first duplicate found.
    pub fn validate(&self) -> Result<(), String> {
        for i in 0..self.ops.len() {
            for j in (i + 1)..self.ops.len() {
                if self.ops[i].old_name == self.ops[j].old_name {
                    return Err(format!(
                        "duplicate old_name '{}' at indices {} and {}",
                        self.ops[i].old_name, i, j
                    ));
                }
            }
        }
        Ok(())
    }

    /// Returns a preview using `ops.len()` as the affected count.
    pub fn preview(&self) -> RenamePreview {
        RenamePreview::new(self.ops.clone(), self.ops.len())
    }

    /// Applies all ops and returns a list of human-readable change descriptions.
    pub fn apply(&self) -> Vec<String> {
        self.ops
            .iter()
            .map(|op| format!("renamed: {} → {}", op.old_name, op.new_name))
            .collect()
    }
}

#[cfg(test)]
mod workspace_rename_tests {
    use super::*;

    #[test]
    fn rename_op_is_local() {
        let op = RenameOp::new("foo", "bar", RenameScope::Local);
        assert!(op.is_local());
    }

    #[test]
    fn rename_op_new_global() {
        let op = RenameOp::new("foo", "bar", RenameScope::Global);
        assert!(!op.is_local());
        assert_eq!(op.old_name, "foo");
        assert_eq!(op.new_name, "bar");
        assert_eq!(op.scope, RenameScope::Global);
    }

    #[test]
    fn workspace_renamer_add_and_validate_ok() {
        let mut renamer = WorkspaceRenamer::new();
        renamer.add_op(RenameOp::new("alpha", "Alpha", RenameScope::Global));
        renamer.add_op(RenameOp::new("beta", "Beta", RenameScope::Local));
        assert!(renamer.validate().is_ok());
    }

    #[test]
    fn workspace_renamer_validate_duplicate_fails() {
        let mut renamer = WorkspaceRenamer::new();
        renamer.add_op(RenameOp::new("dup", "Dup1", RenameScope::Global));
        renamer.add_op(RenameOp::new("dup", "Dup2", RenameScope::Local));
        let result = renamer.validate();
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("dup"));
    }

    #[test]
    fn workspace_renamer_preview_has_changes() {
        let mut renamer = WorkspaceRenamer::new();
        renamer.add_op(RenameOp::new("x", "y", RenameScope::Global));
        let preview = renamer.preview();
        assert!(preview.has_changes());
    }

    #[test]
    fn workspace_renamer_preview_count() {
        let mut renamer = WorkspaceRenamer::new();
        renamer.add_op(RenameOp::new("a", "A", RenameScope::Local));
        renamer.add_op(RenameOp::new("b", "B", RenameScope::Global));
        renamer.add_op(RenameOp::new("c", "C", RenameScope::Local));
        let preview = renamer.preview();
        assert_eq!(preview.op_count(), 3);
        assert_eq!(preview.affected_count, 3);
    }

    #[test]
    fn workspace_renamer_apply_returns_strings() {
        let mut renamer = WorkspaceRenamer::new();
        renamer.add_op(RenameOp::new("old_fn", "new_fn", RenameScope::Global));
        let results = renamer.apply();
        assert_eq!(results.len(), 1);
        assert!(results[0].contains("old_fn"));
        assert!(results[0].contains("new_fn"));
    }

    #[test]
    fn rename_preview_no_changes_when_empty() {
        let preview = RenamePreview::new(vec![], 0);
        assert!(!preview.has_changes());
        assert_eq!(preview.op_count(), 0);
    }

    #[test]
    fn rename_scope_file_scoped() {
        let path = "src/main.rs".to_string();
        let op = RenameOp::new("my_var", "my_variable", RenameScope::FileScoped(path.clone()));
        assert!(!op.is_local());
        assert_eq!(op.scope, RenameScope::FileScoped(path));
    }
}
