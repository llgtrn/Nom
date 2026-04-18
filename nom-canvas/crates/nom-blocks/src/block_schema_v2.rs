/// BlockSchemaV2 — v2 schema format with typed fields.
/// MigrationTool — migrates v1 → v2 blocks.
/// RoundTripValidator — validates that migrate(block) round-trips correctly.

/// The type of a schema field.
#[derive(Debug, Clone, PartialEq)]
pub enum FieldType {
    /// Plain text value.
    Text,
    /// Numeric value.
    Number,
    /// Boolean flag.
    Boolean,
    /// Reference to another block or entity.
    Reference,
}

impl FieldType {
    /// Returns the canonical string name of this field type.
    pub fn type_name(&self) -> &str {
        match self {
            FieldType::Text => "text",
            FieldType::Number => "number",
            FieldType::Boolean => "boolean",
            FieldType::Reference => "reference",
        }
    }
}

/// A single typed field within a block schema.
#[derive(Debug, Clone)]
pub struct SchemaField {
    /// Field name.
    pub name: String,
    /// Field type.
    pub field_type: FieldType,
    /// Whether this field must be present.
    pub required: bool,
}

impl SchemaField {
    /// Creates a new `SchemaField`.
    pub fn new(name: impl Into<String>, field_type: FieldType, required: bool) -> Self {
        Self {
            name: name.into(),
            field_type,
            required,
        }
    }
}

/// A v2 block schema with typed field definitions.
#[derive(Debug, Clone)]
pub struct BlockSchemaV2 {
    /// The block type identifier.
    pub block_type: String,
    /// Schema version number.
    pub version: u32,
    /// Ordered list of field definitions.
    pub fields: Vec<SchemaField>,
}

impl BlockSchemaV2 {
    /// Creates a new `BlockSchemaV2`.  `version` is set to 2 regardless of the argument
    /// so callers can pass `2` explicitly without confusion.
    pub fn new(block_type: impl Into<String>, _version: u32) -> Self {
        Self {
            block_type: block_type.into(),
            version: 2,
            fields: Vec::new(),
        }
    }

    /// Appends a field to this schema.
    pub fn add_field(&mut self, field: SchemaField) {
        self.fields.push(field);
    }

    /// Returns the total number of fields.
    pub fn field_count(&self) -> usize {
        self.fields.len()
    }

    /// Returns references to all required fields.
    pub fn required_fields(&self) -> Vec<&SchemaField> {
        self.fields.iter().filter(|f| f.required).collect()
    }

    /// Returns `true` when this schema is version 2.
    pub fn is_v2(&self) -> bool {
        self.version == 2
    }
}

/// Migrates block schemas from an older version to v2.
pub struct MigrationTool;

impl MigrationTool {
    /// Creates a new `MigrationTool`.
    pub fn new() -> Self {
        Self
    }

    /// Produces a v2 schema for `block_type` using the supplied `old_version` for context.
    /// The resulting schema contains a single default `"id"` field of type `Reference`.
    pub fn migrate_to_v2(&self, block_type: &str, _old_version: u32) -> BlockSchemaV2 {
        let mut schema = BlockSchemaV2::new(block_type, 2);
        schema.add_field(SchemaField::new("id", FieldType::Reference, true));
        schema
    }

    /// Returns `true` when the given version can be migrated (only v1 is supported).
    pub fn can_migrate(&self, old_version: u32) -> bool {
        old_version == 1
    }
}

impl Default for MigrationTool {
    fn default() -> Self {
        Self::new()
    }
}

/// Validates that a migrated schema correctly round-trips from an original.
pub struct RoundTripValidator;

impl RoundTripValidator {
    /// Creates a new `RoundTripValidator`.
    pub fn new() -> Self {
        Self
    }

    /// Returns `true` when `migrated` has the same `block_type` as `original` AND is v2.
    pub fn validate(&self, original: &BlockSchemaV2, migrated: &BlockSchemaV2) -> bool {
        original.block_type == migrated.block_type && migrated.is_v2()
    }

    /// Returns `true` when `schema` has exactly `expected` fields.
    pub fn validate_field_count(&self, schema: &BlockSchemaV2, expected: usize) -> bool {
        schema.field_count() == expected
    }
}

impl Default for RoundTripValidator {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod block_schema_v2_tests {
    use super::*;

    #[test]
    fn field_type_type_name() {
        assert_eq!(FieldType::Text.type_name(), "text");
        assert_eq!(FieldType::Number.type_name(), "number");
        assert_eq!(FieldType::Boolean.type_name(), "boolean");
        assert_eq!(FieldType::Reference.type_name(), "reference");
    }

    #[test]
    fn block_schema_v2_is_v2() {
        let schema = BlockSchemaV2::new("heading", 2);
        assert!(schema.is_v2());
    }

    #[test]
    fn block_schema_v2_add_field_count() {
        let mut schema = BlockSchemaV2::new("paragraph", 2);
        assert_eq!(schema.field_count(), 0);
        schema.add_field(SchemaField::new("text", FieldType::Text, true));
        assert_eq!(schema.field_count(), 1);
        schema.add_field(SchemaField::new("visible", FieldType::Boolean, false));
        assert_eq!(schema.field_count(), 2);
    }

    #[test]
    fn block_schema_v2_required_fields_filter() {
        let mut schema = BlockSchemaV2::new("image", 2);
        schema.add_field(SchemaField::new("src", FieldType::Text, true));
        schema.add_field(SchemaField::new("alt", FieldType::Text, false));
        schema.add_field(SchemaField::new("width", FieldType::Number, true));
        let required = schema.required_fields();
        assert_eq!(required.len(), 2);
        assert!(required.iter().any(|f| f.name == "src"));
        assert!(required.iter().any(|f| f.name == "width"));
        assert!(!required.iter().any(|f| f.name == "alt"));
    }

    #[test]
    fn migration_tool_can_migrate_v1() {
        let tool = MigrationTool::new();
        assert!(tool.can_migrate(1));
    }

    #[test]
    fn migration_tool_cannot_migrate_v2() {
        let tool = MigrationTool::new();
        assert!(!tool.can_migrate(2));
    }

    #[test]
    fn migration_tool_migrate_creates_v2() {
        let tool = MigrationTool::new();
        let schema = tool.migrate_to_v2("table", 1);
        assert!(schema.is_v2());
        assert_eq!(schema.block_type, "table");
        assert_eq!(schema.field_count(), 1);
        assert_eq!(schema.fields[0].name, "id");
        assert_eq!(schema.fields[0].field_type, FieldType::Reference);
    }

    #[test]
    fn round_trip_validator_validate_pass() {
        let original = BlockSchemaV2::new("card", 2);
        let tool = MigrationTool::new();
        let migrated = tool.migrate_to_v2("card", 1);
        let validator = RoundTripValidator::new();
        assert!(validator.validate(&original, &migrated));
    }

    #[test]
    fn round_trip_validator_validate_fail_wrong_type() {
        let original = BlockSchemaV2::new("card", 2);
        let tool = MigrationTool::new();
        let migrated = tool.migrate_to_v2("button", 1);
        let validator = RoundTripValidator::new();
        assert!(!validator.validate(&original, &migrated));
    }
}
