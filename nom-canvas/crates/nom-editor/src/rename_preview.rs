/// Visual preview model for rename operations in the C4 editor canvas.

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RenamePreviewKind {
    InlineEdit,
    HighlightAll,
    SideBySide,
}

impl RenamePreviewKind {
    pub fn is_interactive(&self) -> bool {
        matches!(self, RenamePreviewKind::InlineEdit | RenamePreviewKind::HighlightAll)
    }

    pub fn description(&self) -> &'static str {
        match self {
            RenamePreviewKind::InlineEdit => "Edit the symbol name directly in the editor",
            RenamePreviewKind::HighlightAll => "Highlight all occurrences before applying",
            RenamePreviewKind::SideBySide => "Show old and new names side by side",
        }
    }
}

#[derive(Debug, Clone)]
pub struct RenameChange {
    pub file_path: String,
    pub byte_start: usize,
    pub byte_end: usize,
    pub old_text: String,
    pub new_text: String,
}

impl RenameChange {
    pub fn byte_len(&self) -> usize {
        self.byte_end - self.byte_start
    }

    pub fn is_same_file(&self, path: &str) -> bool {
        self.file_path == path
    }
}

#[derive(Debug, Clone)]
pub struct RenamePreviewModel {
    pub symbol_name: String,
    pub new_name: String,
    pub changes: Vec<RenameChange>,
    pub kind: RenamePreviewKind,
}

impl RenamePreviewModel {
    pub fn total_changes(&self) -> usize {
        self.changes.len()
    }

    pub fn files_affected(&self) -> Vec<&str> {
        let mut seen = Vec::new();
        for change in &self.changes {
            let path = change.file_path.as_str();
            if !seen.contains(&path) {
                seen.push(path);
            }
        }
        seen
    }

    pub fn has_cross_file_changes(&self) -> bool {
        self.files_affected().len() > 1
    }
}

#[derive(Debug, Clone)]
pub struct RenameConflict {
    pub file_path: String,
    pub conflict_name: String,
    pub reason: String,
}

impl RenameConflict {
    pub fn description(&self) -> String {
        format!("conflict in {}: {}", self.file_path, self.reason)
    }
}

#[derive(Debug, Default)]
pub struct RenameApplier {
    pub pending: Option<RenamePreviewModel>,
    pub conflicts: Vec<RenameConflict>,
}

impl RenameApplier {
    pub fn set_preview(&mut self, m: RenamePreviewModel) {
        self.pending = Some(m);
    }

    pub fn add_conflict(&mut self, c: RenameConflict) {
        self.conflicts.push(c);
    }

    pub fn can_apply(&self) -> bool {
        self.pending.is_some() && self.conflicts.is_empty()
    }

    pub fn apply(&mut self) -> Option<RenamePreviewModel> {
        if self.can_apply() {
            self.pending.take()
        } else {
            None
        }
    }
}

#[cfg(test)]
mod rename_preview_tests {
    use super::*;

    fn make_change(file: &str, start: usize, end: usize, old: &str, new: &str) -> RenameChange {
        RenameChange {
            file_path: file.to_string(),
            byte_start: start,
            byte_end: end,
            old_text: old.to_string(),
            new_text: new.to_string(),
        }
    }

    fn make_model(changes: Vec<RenameChange>) -> RenamePreviewModel {
        RenamePreviewModel {
            symbol_name: "foo".to_string(),
            new_name: "bar".to_string(),
            changes,
            kind: RenamePreviewKind::InlineEdit,
        }
    }

    #[test]
    fn test_kind_is_interactive() {
        assert!(RenamePreviewKind::InlineEdit.is_interactive());
        assert!(RenamePreviewKind::HighlightAll.is_interactive());
        assert!(!RenamePreviewKind::SideBySide.is_interactive());
    }

    #[test]
    fn test_change_byte_len() {
        let c = make_change("a.nom", 10, 13, "foo", "bar");
        assert_eq!(c.byte_len(), 3);
    }

    #[test]
    fn test_change_is_same_file() {
        let c = make_change("src/main.nom", 0, 3, "foo", "bar");
        assert!(c.is_same_file("src/main.nom"));
        assert!(!c.is_same_file("src/other.nom"));
    }

    #[test]
    fn test_model_total_changes() {
        let model = make_model(vec![
            make_change("a.nom", 0, 3, "foo", "bar"),
            make_change("b.nom", 5, 8, "foo", "bar"),
        ]);
        assert_eq!(model.total_changes(), 2);
    }

    #[test]
    fn test_model_files_affected_unique() {
        let model = make_model(vec![
            make_change("a.nom", 0, 3, "foo", "bar"),
            make_change("a.nom", 10, 13, "foo", "bar"),
            make_change("b.nom", 5, 8, "foo", "bar"),
        ]);
        let files = model.files_affected();
        assert_eq!(files.len(), 2);
        assert!(files.contains(&"a.nom"));
        assert!(files.contains(&"b.nom"));
    }

    #[test]
    fn test_model_has_cross_file_changes_true() {
        let model = make_model(vec![
            make_change("a.nom", 0, 3, "foo", "bar"),
            make_change("b.nom", 5, 8, "foo", "bar"),
        ]);
        assert!(model.has_cross_file_changes());
    }

    #[test]
    fn test_conflict_description() {
        let c = RenameConflict {
            file_path: "src/lib.nom".to_string(),
            conflict_name: "bar".to_string(),
            reason: "name already defined in scope".to_string(),
        };
        assert_eq!(
            c.description(),
            "conflict in src/lib.nom: name already defined in scope"
        );
    }

    #[test]
    fn test_applier_can_apply_true() {
        let mut applier = RenameApplier::default();
        applier.set_preview(make_model(vec![make_change("a.nom", 0, 3, "foo", "bar")]));
        assert!(applier.can_apply());
    }

    #[test]
    fn test_applier_apply_returns_model() {
        let mut applier = RenameApplier::default();
        let model = make_model(vec![make_change("a.nom", 0, 3, "foo", "bar")]);
        applier.set_preview(model.clone());
        let result = applier.apply();
        assert!(result.is_some());
        assert_eq!(result.unwrap().symbol_name, "foo");
        assert!(applier.pending.is_none());
    }
}
