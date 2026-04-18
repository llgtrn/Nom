/// Schema versioning, workspace schema tracking, and migration planning for NomCanvas workspaces.

/// Semantic version for workspace schemas.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SchemaVersion {
    /// Major version — breaking changes.
    pub major: u32,
    /// Minor version — backwards-compatible additions.
    pub minor: u32,
    /// Patch version — backwards-compatible fixes.
    pub patch: u32,
}

impl SchemaVersion {
    /// Construct a new SchemaVersion.
    pub fn new(major: u32, minor: u32, patch: u32) -> Self {
        Self { major, minor, patch }
    }

    /// Format as "major.minor.patch".
    pub fn as_string(&self) -> String {
        format!("{}.{}.{}", self.major, self.minor, self.patch)
    }

    /// Two versions are compatible when they share the same major version.
    pub fn is_compatible_with(&self, other: &SchemaVersion) -> bool {
        self.major == other.major
    }

    /// Increment minor, reset patch to 0.
    pub fn bump_minor(mut self) -> Self {
        self.minor += 1;
        self.patch = 0;
        self
    }
}

/// Snapshot of a workspace's structural metrics at a given schema version.
#[derive(Debug, Clone)]
pub struct WorkspaceSchema {
    /// Schema version this workspace conforms to.
    pub version: SchemaVersion,
    /// Number of blocks in the workspace.
    pub block_count: u32,
    /// Number of connectors in the workspace.
    pub connector_count: u32,
    /// Total bytes consumed by block/connector metadata.
    pub metadata_size_bytes: u64,
}

impl WorkspaceSchema {
    /// Create a new empty workspace schema at the given version.
    pub fn new(version: SchemaVersion) -> Self {
        Self {
            version,
            block_count: 0,
            connector_count: 0,
            metadata_size_bytes: 0,
        }
    }

    /// Increment the block count by one.
    pub fn add_block(&mut self) {
        self.block_count += 1;
    }

    /// Increment the connector count by one.
    pub fn add_connector(&mut self) {
        self.connector_count += 1;
    }

    /// Sum of blocks and connectors.
    pub fn total_elements(&self) -> u32 {
        self.block_count + self.connector_count
    }

    /// Ratio of blocks to total elements (avoids divide-by-zero via +1 denominator).
    pub fn compact_ratio(&self) -> f32 {
        self.block_count as f32 / (self.total_elements() + 1) as f32
    }
}

/// A single schema migration step.
#[derive(Debug, Clone)]
pub struct SchemaMigration {
    /// Version being migrated from.
    pub from_version: SchemaVersion,
    /// Version being migrated to.
    pub to_version: SchemaVersion,
    /// Human-readable description of what the migration does.
    pub description: String,
}

impl SchemaMigration {
    /// Construct a new migration step.
    pub fn new(from: SchemaVersion, to: SchemaVersion, description: impl Into<String>) -> Self {
        Self {
            from_version: from,
            to_version: to,
            description: description.into(),
        }
    }

    /// Returns true when the migration moves forward (major increase, or same major + minor increase).
    pub fn is_upgrade(&self) -> bool {
        let f = &self.from_version;
        let t = &self.to_version;
        t.major > f.major || (t.major == f.major && t.minor > f.minor)
    }
}

/// An ordered sequence of migration steps forming a complete migration path.
#[derive(Debug, Clone)]
pub struct MigrationPlan {
    /// Ordered list of migration steps.
    pub steps: Vec<SchemaMigration>,
}

impl MigrationPlan {
    /// Create an empty migration plan.
    pub fn new() -> Self {
        Self { steps: Vec::new() }
    }

    /// Append a migration step to the plan.
    pub fn add_step(&mut self, migration: SchemaMigration) {
        self.steps.push(migration);
    }

    /// Number of steps in the plan.
    pub fn step_count(&self) -> usize {
        self.steps.len()
    }

    /// True when the plan has no steps.
    pub fn is_empty(&self) -> bool {
        self.steps.is_empty()
    }
}

impl Default for MigrationPlan {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod workspace_schema_tests {
    use super::*;

    #[test]
    fn schema_version_as_string() {
        let v = SchemaVersion::new(1, 2, 3);
        assert_eq!(v.as_string(), "1.2.3");
    }

    #[test]
    fn is_compatible_with_same_major() {
        let a = SchemaVersion::new(2, 0, 0);
        let b = SchemaVersion::new(2, 5, 1);
        assert!(a.is_compatible_with(&b));
    }

    #[test]
    fn is_compatible_with_different_major() {
        let a = SchemaVersion::new(1, 0, 0);
        let b = SchemaVersion::new(2, 0, 0);
        assert!(!a.is_compatible_with(&b));
    }

    #[test]
    fn bump_minor_resets_patch() {
        let v = SchemaVersion::new(1, 3, 7).bump_minor();
        assert_eq!(v.minor, 4);
        assert_eq!(v.patch, 0);
        assert_eq!(v.major, 1);
    }

    #[test]
    fn workspace_schema_total_elements() {
        let mut ws = WorkspaceSchema::new(SchemaVersion::new(1, 0, 0));
        ws.block_count = 3;
        ws.connector_count = 2;
        assert_eq!(ws.total_elements(), 5);
    }

    #[test]
    fn add_block_increments_count() {
        let mut ws = WorkspaceSchema::new(SchemaVersion::new(1, 0, 0));
        ws.add_block();
        ws.add_block();
        assert_eq!(ws.block_count, 2);
    }

    #[test]
    fn schema_migration_is_upgrade_true() {
        let from = SchemaVersion::new(1, 0, 0);
        let to = SchemaVersion::new(1, 1, 0);
        let m = SchemaMigration::new(from, to, "add field");
        assert!(m.is_upgrade());
    }

    #[test]
    fn schema_migration_is_upgrade_false_for_downgrade() {
        let from = SchemaVersion::new(2, 0, 0);
        let to = SchemaVersion::new(1, 5, 0);
        let m = SchemaMigration::new(from, to, "rollback");
        assert!(!m.is_upgrade());
    }

    #[test]
    fn migration_plan_step_count() {
        let mut plan = MigrationPlan::new();
        let m1 = SchemaMigration::new(
            SchemaVersion::new(1, 0, 0),
            SchemaVersion::new(1, 1, 0),
            "step one",
        );
        let m2 = SchemaMigration::new(
            SchemaVersion::new(1, 1, 0),
            SchemaVersion::new(2, 0, 0),
            "step two",
        );
        plan.add_step(m1);
        plan.add_step(m2);
        assert_eq!(plan.step_count(), 2);
    }
}
