/// Identifies a schema version by semantic version triple.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SchemaVersionId {
    pub major: u32,
    pub minor: u32,
    pub patch: u32,
}

impl SchemaVersionId {
    pub fn version_string(&self) -> String {
        format!("{}.{}.{}", self.major, self.minor, self.patch)
    }

    pub fn is_compatible_with(&self, other: &SchemaVersionId) -> bool {
        self.major == other.major
    }

    pub fn is_newer_than(&self, other: &SchemaVersionId) -> bool {
        if self.major != other.major {
            return self.major > other.major;
        }
        if self.minor != other.minor {
            return self.minor > other.minor;
        }
        self.patch > other.patch
    }
}

/// A directed edge between two schema versions with a migration name.
#[derive(Debug, Clone)]
pub struct VersionEdge {
    pub from_version: SchemaVersionId,
    pub to_version: SchemaVersionId,
    pub migration_name: String,
}

impl VersionEdge {
    pub fn is_upgrade(&self) -> bool {
        self.to_version.is_newer_than(&self.from_version)
    }

    pub fn edge_label(&self) -> String {
        format!("{} -> {}", self.from_version.version_string(), self.to_version.version_string())
    }
}

/// A graph of schema versions connected by migration edges.
#[derive(Debug, Default)]
pub struct SchemaVersionGraph {
    pub versions: Vec<SchemaVersionId>,
    pub edges: Vec<VersionEdge>,
}

impl SchemaVersionGraph {
    pub fn add_version(&mut self, v: SchemaVersionId) {
        self.versions.push(v);
    }

    pub fn add_edge(&mut self, e: VersionEdge) {
        self.edges.push(e);
    }

    /// Returns the version that is newer than all others, or None if empty.
    pub fn latest(&self) -> Option<&SchemaVersionId> {
        self.versions.iter().reduce(|acc, v| {
            if v.is_newer_than(acc) { v } else { acc }
        })
    }

    /// Returns edges whose from_version matches `from`, in insertion order.
    pub fn upgrade_path<'a>(&'a self, from: &SchemaVersionId) -> Vec<&'a VersionEdge> {
        self.edges.iter().filter(|e| &e.from_version == from).collect()
    }
}

/// Describes the differences between two schema versions.
#[derive(Debug, Default)]
pub struct VersionDiff {
    pub added_kinds: Vec<String>,
    pub removed_kinds: Vec<String>,
    pub changed_kinds: Vec<String>,
}

impl VersionDiff {
    pub fn is_breaking(&self) -> bool {
        !self.removed_kinds.is_empty()
    }

    pub fn total_changes(&self) -> usize {
        self.added_kinds.len() + self.removed_kinds.len() + self.changed_kinds.len()
    }
}

/// A plan to migrate a schema through a sequence of edges.
#[derive(Debug, Default)]
pub struct SchemaMigrationPlan {
    pub steps: Vec<VersionEdge>,
    pub diff: VersionDiff,
}

impl SchemaMigrationPlan {
    pub fn step_count(&self) -> usize {
        self.steps.len()
    }

    pub fn is_safe(&self) -> bool {
        !self.diff.is_breaking() && self.step_count() > 0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn v(major: u32, minor: u32, patch: u32) -> SchemaVersionId {
        SchemaVersionId { major, minor, patch }
    }

    #[test]
    fn test_version_string() {
        assert_eq!(v(1, 2, 3).version_string(), "1.2.3");
        assert_eq!(v(0, 0, 0).version_string(), "0.0.0");
    }

    #[test]
    fn test_is_compatible_with_same_major() {
        let a = v(2, 0, 0);
        let b = v(2, 5, 3);
        assert!(a.is_compatible_with(&b));
        let c = v(3, 0, 0);
        assert!(!a.is_compatible_with(&c));
    }

    #[test]
    fn test_is_newer_than() {
        assert!(v(2, 0, 0).is_newer_than(&v(1, 9, 9)));
        assert!(v(1, 1, 0).is_newer_than(&v(1, 0, 9)));
        assert!(v(1, 0, 1).is_newer_than(&v(1, 0, 0)));
        assert!(!v(1, 0, 0).is_newer_than(&v(1, 0, 0)));
        assert!(!v(1, 0, 0).is_newer_than(&v(2, 0, 0)));
    }

    #[test]
    fn test_edge_is_upgrade() {
        let upgrade = VersionEdge {
            from_version: v(1, 0, 0),
            to_version: v(1, 1, 0),
            migration_name: "add_foo".into(),
        };
        assert!(upgrade.is_upgrade());

        let downgrade = VersionEdge {
            from_version: v(1, 1, 0),
            to_version: v(1, 0, 0),
            migration_name: "remove_foo".into(),
        };
        assert!(!downgrade.is_upgrade());
    }

    #[test]
    fn test_edge_label() {
        let edge = VersionEdge {
            from_version: v(1, 0, 0),
            to_version: v(2, 0, 0),
            migration_name: "major_bump".into(),
        };
        assert_eq!(edge.edge_label(), "1.0.0 -> 2.0.0");
    }

    #[test]
    fn test_graph_latest() {
        let mut g = SchemaVersionGraph::default();
        assert_eq!(g.latest(), None);
        g.add_version(v(1, 0, 0));
        g.add_version(v(1, 2, 0));
        g.add_version(v(1, 1, 0));
        assert_eq!(g.latest(), Some(&v(1, 2, 0)));
    }

    #[test]
    fn test_graph_upgrade_path() {
        let mut g = SchemaVersionGraph::default();
        g.add_edge(VersionEdge { from_version: v(1, 0, 0), to_version: v(1, 1, 0), migration_name: "m1".into() });
        g.add_edge(VersionEdge { from_version: v(1, 1, 0), to_version: v(1, 2, 0), migration_name: "m2".into() });
        g.add_edge(VersionEdge { from_version: v(1, 0, 0), to_version: v(1, 0, 1), migration_name: "m3".into() });

        let path = g.upgrade_path(&v(1, 0, 0));
        assert_eq!(path.len(), 2);
        assert_eq!(path[0].migration_name, "m1");
        assert_eq!(path[1].migration_name, "m3");
    }

    #[test]
    fn test_diff_is_breaking() {
        let safe = VersionDiff {
            added_kinds: vec!["Foo".into()],
            removed_kinds: vec![],
            changed_kinds: vec![],
        };
        assert!(!safe.is_breaking());

        let breaking = VersionDiff {
            added_kinds: vec![],
            removed_kinds: vec!["Bar".into()],
            changed_kinds: vec![],
        };
        assert!(breaking.is_breaking());
    }

    #[test]
    fn test_plan_is_safe() {
        let safe_plan = SchemaMigrationPlan {
            steps: vec![VersionEdge {
                from_version: v(1, 0, 0),
                to_version: v(1, 1, 0),
                migration_name: "add_kinds".into(),
            }],
            diff: VersionDiff {
                added_kinds: vec!["NewKind".into()],
                removed_kinds: vec![],
                changed_kinds: vec![],
            },
        };
        assert!(safe_plan.is_safe());

        let breaking_plan = SchemaMigrationPlan {
            steps: vec![VersionEdge {
                from_version: v(1, 0, 0),
                to_version: v(1, 1, 0),
                migration_name: "remove_kinds".into(),
            }],
            diff: VersionDiff {
                added_kinds: vec![],
                removed_kinds: vec!["OldKind".into()],
                changed_kinds: vec![],
            },
        };
        assert!(!breaking_plan.is_safe());

        let empty_plan = SchemaMigrationPlan {
            steps: vec![],
            diff: VersionDiff::default(),
        };
        assert!(!empty_plan.is_safe());
    }
}
