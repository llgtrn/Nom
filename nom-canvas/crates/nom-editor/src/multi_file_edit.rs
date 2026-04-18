/// Scope of an edit operation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EditScope {
    SingleFile,
    MultiFile,
    Workspace,
}

impl EditScope {
    pub fn is_multi(&self) -> bool {
        matches!(self, EditScope::MultiFile | EditScope::Workspace)
    }

    pub fn scope_label(&self) -> &'static str {
        match self {
            EditScope::SingleFile => "single",
            EditScope::MultiFile => "multi",
            EditScope::Workspace => "workspace",
        }
    }
}

/// A single change to apply within a file.
#[derive(Debug, Clone)]
pub struct MultiFileChange {
    pub file_path: String,
    pub byte_start: usize,
    pub byte_end: usize,
    pub replacement: String,
}

impl MultiFileChange {
    pub fn replacement_len(&self) -> usize {
        self.replacement.len()
    }

    pub fn is_deletion(&self) -> bool {
        self.replacement.is_empty()
    }
}

/// A collection of changes spanning potentially many files.
#[derive(Debug, Clone)]
pub struct MultiFileSession {
    pub changes: Vec<MultiFileChange>,
    pub scope: EditScope,
}

impl MultiFileSession {
    pub fn add_change(&mut self, c: MultiFileChange) {
        self.changes.push(c);
    }

    pub fn files_count(&self) -> usize {
        let mut paths: Vec<&str> = self.changes.iter().map(|c| c.file_path.as_str()).collect();
        paths.sort_unstable();
        paths.dedup();
        paths.len()
    }

    pub fn changes_for_file<'a>(&'a self, path: &str) -> Vec<&'a MultiFileChange> {
        self.changes.iter().filter(|c| c.file_path == path).collect()
    }

    pub fn total_changes(&self) -> usize {
        self.changes.len()
    }
}

/// Before/after snapshot of a file used for diffing.
#[derive(Debug, Clone)]
pub struct MultiFileDiff {
    pub before: String,
    pub after: String,
    pub file_path: String,
}

impl MultiFileDiff {
    pub fn is_changed(&self) -> bool {
        self.before != self.after
    }

    pub fn diff_size(&self) -> usize {
        self.after.len().abs_diff(self.before.len())
    }
}

/// Applies a `MultiFileSession` (or simulates application).
pub struct SessionApplier {
    pub session: MultiFileSession,
}

impl SessionApplier {
    pub fn new(session: MultiFileSession) -> Self {
        Self { session }
    }

    pub fn can_apply(&self) -> bool {
        self.session.total_changes() > 0
    }

    /// Returns the deduplicated list of file paths touched by all changes.
    pub fn simulate_apply(&self) -> Vec<String> {
        let mut paths: Vec<String> = self
            .session
            .changes
            .iter()
            .map(|c| c.file_path.clone())
            .collect();
        paths.sort_unstable();
        paths.dedup();
        paths
    }
}

#[cfg(test)]
mod multi_file_edit_tests {
    use super::*;

    fn make_session(scope: EditScope) -> MultiFileSession {
        MultiFileSession { changes: vec![], scope }
    }

    // 1. EditScope::is_multi returns true for MultiFile and Workspace, false for SingleFile
    #[test]
    fn scope_is_multi() {
        assert!(!EditScope::SingleFile.is_multi());
        assert!(EditScope::MultiFile.is_multi());
        assert!(EditScope::Workspace.is_multi());
    }

    // 2. scope_label returns correct string
    #[test]
    fn scope_label() {
        assert_eq!(EditScope::SingleFile.scope_label(), "single");
        assert_eq!(EditScope::MultiFile.scope_label(), "multi");
        assert_eq!(EditScope::Workspace.scope_label(), "workspace");
    }

    // 3. is_deletion returns true when replacement is empty
    #[test]
    fn change_is_deletion_true() {
        let c = MultiFileChange {
            file_path: "a.nom".into(),
            byte_start: 0,
            byte_end: 5,
            replacement: String::new(),
        };
        assert!(c.is_deletion());
    }

    // 4. replacement_len returns length of replacement string
    #[test]
    fn change_replacement_len() {
        let c = MultiFileChange {
            file_path: "b.nom".into(),
            byte_start: 0,
            byte_end: 3,
            replacement: "hello".into(),
        };
        assert_eq!(c.replacement_len(), 5);
    }

    // 5. files_count returns number of unique file paths
    #[test]
    fn session_files_count_unique() {
        let mut s = make_session(EditScope::MultiFile);
        s.add_change(MultiFileChange { file_path: "x.nom".into(), byte_start: 0, byte_end: 1, replacement: "a".into() });
        s.add_change(MultiFileChange { file_path: "x.nom".into(), byte_start: 2, byte_end: 3, replacement: "b".into() });
        s.add_change(MultiFileChange { file_path: "y.nom".into(), byte_start: 0, byte_end: 1, replacement: "c".into() });
        assert_eq!(s.files_count(), 2);
    }

    // 6. changes_for_file returns only changes matching the given path
    #[test]
    fn session_changes_for_file() {
        let mut s = make_session(EditScope::MultiFile);
        s.add_change(MultiFileChange { file_path: "x.nom".into(), byte_start: 0, byte_end: 1, replacement: "a".into() });
        s.add_change(MultiFileChange { file_path: "y.nom".into(), byte_start: 0, byte_end: 1, replacement: "b".into() });
        s.add_change(MultiFileChange { file_path: "x.nom".into(), byte_start: 5, byte_end: 6, replacement: "c".into() });
        let for_x = s.changes_for_file("x.nom");
        assert_eq!(for_x.len(), 2);
        assert!(for_x.iter().all(|c| c.file_path == "x.nom"));
    }

    // 7. total_changes returns the total number of changes
    #[test]
    fn session_total_changes() {
        let mut s = make_session(EditScope::Workspace);
        assert_eq!(s.total_changes(), 0);
        s.add_change(MultiFileChange { file_path: "a.nom".into(), byte_start: 0, byte_end: 1, replacement: "x".into() });
        s.add_change(MultiFileChange { file_path: "b.nom".into(), byte_start: 0, byte_end: 1, replacement: "y".into() });
        assert_eq!(s.total_changes(), 2);
    }

    // 8. is_changed returns false when before == after, true otherwise
    #[test]
    fn diff_is_changed() {
        let unchanged = MultiFileDiff { before: "abc".into(), after: "abc".into(), file_path: "f.nom".into() };
        assert!(!unchanged.is_changed());
        let changed = MultiFileDiff { before: "abc".into(), after: "xyz".into(), file_path: "f.nom".into() };
        assert!(changed.is_changed());
    }

    // 9. simulate_apply returns deduplicated file paths
    #[test]
    fn applier_simulate_apply_dedup() {
        let mut s = make_session(EditScope::MultiFile);
        s.add_change(MultiFileChange { file_path: "z.nom".into(), byte_start: 0, byte_end: 1, replacement: "a".into() });
        s.add_change(MultiFileChange { file_path: "z.nom".into(), byte_start: 2, byte_end: 3, replacement: "b".into() });
        s.add_change(MultiFileChange { file_path: "w.nom".into(), byte_start: 0, byte_end: 1, replacement: "c".into() });
        let applier = SessionApplier::new(s);
        let paths = applier.simulate_apply();
        assert_eq!(paths.len(), 2);
        assert!(paths.contains(&"w.nom".to_string()));
        assert!(paths.contains(&"z.nom".to_string()));
    }
}
