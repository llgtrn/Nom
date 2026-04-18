#![deny(unsafe_code)]

/// A column in a semantic model (WrenAI MDL pattern).
#[derive(Debug, Clone)]
pub struct SemanticColumn {
    pub name: String,
    pub data_type: SemanticDataType,
    pub description: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SemanticDataType {
    String,
    Integer,
    Float,
    Boolean,
    Date,
    Timestamp,
    Json,
}

impl SemanticDataType {
    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "string" | "text" | "varchar" => Some(Self::String),
            "int" | "integer" | "bigint" => Some(Self::Integer),
            "float" | "double" | "decimal" => Some(Self::Float),
            "bool" | "boolean" => Some(Self::Boolean),
            "date" => Some(Self::Date),
            "timestamp" | "datetime" => Some(Self::Timestamp),
            "json" | "jsonb" => Some(Self::Json),
            _ => None,
        }
    }
}

/// A semantic model — describes a table/view in the data layer.
#[derive(Debug, Clone)]
pub struct SemanticModel {
    pub name: String,
    pub source_table: String,
    pub columns: Vec<SemanticColumn>,
    pub description: Option<String>,
}

impl SemanticModel {
    pub fn new(name: impl Into<String>, source: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            source_table: source.into(),
            columns: Vec::new(),
            description: None,
        }
    }

    pub fn add_column(&mut self, col: SemanticColumn) -> &mut Self {
        self.columns.push(col);
        self
    }

    pub fn column(&self, name: &str) -> Option<&SemanticColumn> {
        self.columns.iter().find(|c| c.name == name)
    }

    pub fn column_count(&self) -> usize {
        self.columns.len()
    }

    /// Generate a simple SQL SELECT from this model.
    pub fn to_select_sql(&self) -> String {
        if self.columns.is_empty() {
            return format!("SELECT * FROM {}", self.source_table);
        }
        let cols: Vec<&str> = self.columns.iter().map(|c| c.name.as_str()).collect();
        format!("SELECT {} FROM {}", cols.join(", "), self.source_table)
    }
}

/// Registry of semantic models.
#[derive(Debug, Default)]
pub struct SemanticRegistry {
    models: Vec<SemanticModel>,
}

impl SemanticRegistry {
    pub fn new() -> Self {
        Self::default()
    }
    pub fn register(&mut self, model: SemanticModel) {
        self.models.push(model);
    }
    pub fn get(&self, name: &str) -> Option<&SemanticModel> {
        self.models.iter().find(|m| m.name == name)
    }
    pub fn model_count(&self) -> usize {
        self.models.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn semantic_model_to_sql() {
        let mut m = SemanticModel::new("users", "raw.users");
        m.add_column(SemanticColumn {
            name: "id".into(),
            data_type: SemanticDataType::Integer,
            description: None,
        });
        m.add_column(SemanticColumn {
            name: "name".into(),
            data_type: SemanticDataType::String,
            description: None,
        });
        assert_eq!(m.to_select_sql(), "SELECT id, name FROM raw.users");
    }
    #[test]
    fn semantic_model_empty_columns_select_star() {
        let m = SemanticModel::new("t", "raw.t");
        assert_eq!(m.to_select_sql(), "SELECT * FROM raw.t");
    }
    #[test]
    fn semantic_data_type_from_str() {
        assert_eq!(
            SemanticDataType::parse("varchar"),
            Some(SemanticDataType::String)
        );
        assert_eq!(
            SemanticDataType::parse("jsonb"),
            Some(SemanticDataType::Json)
        );
        assert_eq!(SemanticDataType::parse("unknown"), None);
    }
    #[test]
    fn semantic_registry_register_and_get() {
        let mut reg = SemanticRegistry::new();
        reg.register(SemanticModel::new("orders", "raw.orders"));
        assert!(reg.get("orders").is_some());
        assert!(reg.get("missing").is_none());
    }

    #[test]
    fn semantic_model_name_preserved() {
        let m = SemanticModel::new("orders", "raw.orders");
        assert_eq!(m.name, "orders");
    }

    #[test]
    fn semantic_registry_register_lookup() {
        let mut reg = SemanticRegistry::new();
        reg.register(SemanticModel::new("products", "raw.products"));
        assert!(reg.get("products").is_some());
    }

    #[test]
    fn semantic_registry_unknown_returns_none() {
        let reg = SemanticRegistry::new();
        assert!(reg.get("nonexistent").is_none());
    }

    #[test]
    fn semantic_sql_generation() {
        let mut m = SemanticModel::new("events", "raw.events");
        m.add_column(SemanticColumn {
            name: "ts".into(),
            data_type: SemanticDataType::Timestamp,
            description: None,
        });
        let sql = m.to_select_sql();
        assert!(!sql.is_empty());
        assert!(sql.contains("raw.events"));
    }

    #[test]
    fn semantic_model_table_name() {
        let m = SemanticModel::new("sessions", "raw.sessions");
        assert_eq!(m.source_table, "raw.sessions");
    }

    #[test]
    fn semantic_model_field_access_name_and_source() {
        let m = SemanticModel::new("metrics", "warehouse.metrics");
        assert_eq!(m.name, "metrics");
        assert_eq!(m.source_table, "warehouse.metrics");
        assert!(m.columns.is_empty());
        assert!(m.description.is_none());
    }

    #[test]
    fn semantic_model_column_count_grows() {
        let mut m = SemanticModel::new("t", "raw.t");
        assert_eq!(m.column_count(), 0);
        m.add_column(SemanticColumn {
            name: "a".into(),
            data_type: SemanticDataType::Integer,
            description: None,
        });
        assert_eq!(m.column_count(), 1);
        m.add_column(SemanticColumn {
            name: "b".into(),
            data_type: SemanticDataType::String,
            description: None,
        });
        assert_eq!(m.column_count(), 2);
    }

    #[test]
    fn semantic_model_column_lookup_hit_and_miss() {
        let mut m = SemanticModel::new("t", "raw.t");
        m.add_column(SemanticColumn {
            name: "user_id".into(),
            data_type: SemanticDataType::Integer,
            description: Some("primary key".into()),
        });
        let col = m.column("user_id").unwrap();
        assert_eq!(col.name, "user_id");
        assert_eq!(col.data_type, SemanticDataType::Integer);
        assert!(m.column("missing").is_none());
    }

    #[test]
    fn semantic_sql_join_style_multi_column() {
        let mut m = SemanticModel::new("orders", "raw.orders");
        for name in &["id", "user_id", "total", "created_at"] {
            m.add_column(SemanticColumn {
                name: (*name).into(),
                data_type: SemanticDataType::String,
                description: None,
            });
        }
        let sql = m.to_select_sql();
        assert!(sql.contains("id"));
        assert!(sql.contains("user_id"));
        assert!(sql.contains("total"));
        assert!(sql.contains("created_at"));
        assert!(sql.contains("raw.orders"));
    }

    #[test]
    fn semantic_registry_count_after_multiple_registers() {
        let mut reg = SemanticRegistry::new();
        assert_eq!(reg.model_count(), 0);
        reg.register(SemanticModel::new("a", "raw.a"));
        reg.register(SemanticModel::new("b", "raw.b"));
        reg.register(SemanticModel::new("c", "raw.c"));
        assert_eq!(reg.model_count(), 3);
    }

    #[test]
    fn semantic_registry_empty_get_returns_none() {
        let reg = SemanticRegistry::new();
        assert!(reg.get("anything").is_none());
        assert_eq!(reg.model_count(), 0);
    }

    #[test]
    fn semantic_data_type_all_variants_from_str() {
        assert_eq!(
            SemanticDataType::parse("int"),
            Some(SemanticDataType::Integer)
        );
        assert_eq!(
            SemanticDataType::parse("bigint"),
            Some(SemanticDataType::Integer)
        );
        assert_eq!(
            SemanticDataType::parse("float"),
            Some(SemanticDataType::Float)
        );
        assert_eq!(
            SemanticDataType::parse("double"),
            Some(SemanticDataType::Float)
        );
        assert_eq!(
            SemanticDataType::parse("decimal"),
            Some(SemanticDataType::Float)
        );
        assert_eq!(
            SemanticDataType::parse("bool"),
            Some(SemanticDataType::Boolean)
        );
        assert_eq!(
            SemanticDataType::parse("boolean"),
            Some(SemanticDataType::Boolean)
        );
        assert_eq!(
            SemanticDataType::parse("date"),
            Some(SemanticDataType::Date)
        );
        assert_eq!(
            SemanticDataType::parse("timestamp"),
            Some(SemanticDataType::Timestamp)
        );
        assert_eq!(
            SemanticDataType::parse("datetime"),
            Some(SemanticDataType::Timestamp)
        );
        assert_eq!(
            SemanticDataType::parse("json"),
            Some(SemanticDataType::Json)
        );
        assert_eq!(
            SemanticDataType::parse("text"),
            Some(SemanticDataType::String)
        );
    }

    #[test]
    fn semantic_model_description_field() {
        let mut m = SemanticModel::new("facts", "dw.facts");
        m.description = Some("fact table".into());
        assert_eq!(m.description.as_deref(), Some("fact table"));
    }

    #[test]
    fn semantic_model_no_description_by_default() {
        let m = SemanticModel::new("x", "raw.x");
        assert!(m.description.is_none());
    }

    #[test]
    fn semantic_model_overwrite_description() {
        let mut m = SemanticModel::new("t", "raw.t");
        m.description = Some("first".into());
        m.description = Some("second".into());
        assert_eq!(m.description.as_deref(), Some("second"));
    }

    #[test]
    fn semantic_data_type_unknown_returns_none() {
        assert_eq!(SemanticDataType::parse("notype"), None);
        assert_eq!(SemanticDataType::parse(""), None);
        assert_eq!(SemanticDataType::parse("INT"), None); // case-sensitive
    }

    #[test]
    fn semantic_model_column_with_description() {
        let mut m = SemanticModel::new("t", "raw.t");
        m.add_column(SemanticColumn {
            name: "col".into(),
            data_type: SemanticDataType::String,
            description: Some("a column".into()),
        });
        let col = m.column("col").unwrap();
        assert_eq!(col.description.as_deref(), Some("a column"));
    }

    #[test]
    fn semantic_registry_allows_multiple_models_with_same_source() {
        let mut reg = SemanticRegistry::new();
        reg.register(SemanticModel::new("view_a", "raw.events"));
        reg.register(SemanticModel::new("view_b", "raw.events"));
        assert_eq!(reg.model_count(), 2);
        assert!(reg.get("view_a").is_some());
        assert!(reg.get("view_b").is_some());
    }

    #[test]
    fn semantic_sql_single_column() {
        let mut m = SemanticModel::new("t", "raw.t");
        m.add_column(SemanticColumn {
            name: "only_col".into(),
            data_type: SemanticDataType::Boolean,
            description: None,
        });
        assert_eq!(m.to_select_sql(), "SELECT only_col FROM raw.t");
    }

    #[test]
    fn semantic_registry_get_returns_correct_model() {
        let mut reg = SemanticRegistry::new();
        reg.register(SemanticModel::new("alpha", "raw.alpha"));
        reg.register(SemanticModel::new("beta", "raw.beta"));
        assert_eq!(reg.get("alpha").unwrap().source_table, "raw.alpha");
        assert_eq!(reg.get("beta").unwrap().source_table, "raw.beta");
    }

    #[test]
    fn semantic_model_add_column_chaining() {
        let mut m = SemanticModel::new("chain", "raw.chain");
        m.add_column(SemanticColumn {
            name: "x".into(),
            data_type: SemanticDataType::Integer,
            description: None,
        })
        .add_column(SemanticColumn {
            name: "y".into(),
            data_type: SemanticDataType::Float,
            description: None,
        });
        assert_eq!(m.column_count(), 2);
    }

    #[test]
    fn semantic_data_type_json_variants() {
        assert_eq!(SemanticDataType::parse("json"), Some(SemanticDataType::Json));
        assert_eq!(SemanticDataType::parse("jsonb"), Some(SemanticDataType::Json));
    }

    #[test]
    fn semantic_registry_first_model_returned() {
        let mut reg = SemanticRegistry::new();
        reg.register(SemanticModel::new("first", "raw.first"));
        reg.register(SemanticModel::new("second", "raw.second"));
        assert!(reg.get("first").is_some());
        assert!(reg.get("second").is_some());
    }

    #[test]
    fn semantic_model_sql_preserves_column_order() {
        let mut m = SemanticModel::new("t", "raw.t");
        for name in &["z", "a", "m"] {
            m.add_column(SemanticColumn {
                name: (*name).into(),
                data_type: SemanticDataType::String,
                description: None,
            });
        }
        let sql = m.to_select_sql();
        let z_pos = sql.find('z').unwrap();
        let a_pos = sql.find('a').unwrap();
        let m_pos = sql.find('m').unwrap();
        assert!(z_pos < a_pos, "z must appear before a in SELECT");
        assert!(a_pos < m_pos, "a must appear before m in SELECT");
    }

    #[test]
    fn semantic_data_type_date_variant() {
        assert_eq!(SemanticDataType::parse("date"), Some(SemanticDataType::Date));
    }

    #[test]
    fn semantic_model_registry_ten_models() {
        let mut reg = SemanticRegistry::new();
        for i in 0..10 {
            reg.register(SemanticModel::new(format!("model_{i}"), format!("raw.t{i}")));
        }
        assert_eq!(reg.model_count(), 10);
        for i in 0..10 {
            assert!(reg.get(&format!("model_{i}")).is_some());
        }
    }

    #[test]
    fn semantic_column_data_type_equality() {
        assert_eq!(SemanticDataType::Integer, SemanticDataType::Integer);
        assert_ne!(SemanticDataType::Integer, SemanticDataType::String);
        assert_ne!(SemanticDataType::Boolean, SemanticDataType::Float);
    }

    // ── Wave AG new tests ────────────────────────────────────────────────────

    #[test]
    fn semantic_model_zero_columns_select_star() {
        let m = SemanticModel::new("empty", "raw.empty");
        assert_eq!(m.to_select_sql(), "SELECT * FROM raw.empty");
    }

    #[test]
    fn semantic_registry_default_is_empty() {
        let reg: SemanticRegistry = Default::default();
        assert_eq!(reg.model_count(), 0);
    }

    #[test]
    fn semantic_model_column_none_for_unknown() {
        let m = SemanticModel::new("t", "raw.t");
        assert!(m.column("does_not_exist").is_none());
    }

    #[test]
    fn semantic_data_type_string_aliases() {
        assert_eq!(SemanticDataType::parse("string"), Some(SemanticDataType::String));
        assert_eq!(SemanticDataType::parse("text"), Some(SemanticDataType::String));
        assert_eq!(SemanticDataType::parse("varchar"), Some(SemanticDataType::String));
    }

    #[test]
    fn semantic_data_type_integer_aliases() {
        assert_eq!(SemanticDataType::parse("int"), Some(SemanticDataType::Integer));
        assert_eq!(SemanticDataType::parse("integer"), Some(SemanticDataType::Integer));
        assert_eq!(SemanticDataType::parse("bigint"), Some(SemanticDataType::Integer));
    }

    #[test]
    fn semantic_data_type_float_aliases() {
        assert_eq!(SemanticDataType::parse("float"), Some(SemanticDataType::Float));
        assert_eq!(SemanticDataType::parse("double"), Some(SemanticDataType::Float));
        assert_eq!(SemanticDataType::parse("decimal"), Some(SemanticDataType::Float));
    }

    #[test]
    fn semantic_data_type_bool_aliases() {
        assert_eq!(SemanticDataType::parse("bool"), Some(SemanticDataType::Boolean));
        assert_eq!(SemanticDataType::parse("boolean"), Some(SemanticDataType::Boolean));
    }

    #[test]
    fn semantic_data_type_timestamp_aliases() {
        assert_eq!(SemanticDataType::parse("timestamp"), Some(SemanticDataType::Timestamp));
        assert_eq!(SemanticDataType::parse("datetime"), Some(SemanticDataType::Timestamp));
    }

    #[test]
    fn semantic_model_sql_contains_source_table() {
        let m = SemanticModel::new("snap", "dw.snap");
        let sql = m.to_select_sql();
        assert!(sql.contains("dw.snap"));
    }

    #[test]
    fn semantic_model_multiple_columns_sql_comma_separated() {
        let mut m = SemanticModel::new("t", "raw.t");
        m.add_column(SemanticColumn { name: "a".into(), data_type: SemanticDataType::Integer, description: None });
        m.add_column(SemanticColumn { name: "b".into(), data_type: SemanticDataType::String, description: None });
        m.add_column(SemanticColumn { name: "c".into(), data_type: SemanticDataType::Boolean, description: None });
        let sql = m.to_select_sql();
        assert!(sql.contains("a, b, c"), "columns must be comma-separated: {sql}");
    }

    // ── Wave AK artifact-diff tests ──────────────────────────────────────────
    //
    // Minimal inline diff helper: compares two byte slices and returns a
    // structured diff summary.  No external crate required.

    #[derive(Debug, PartialEq, Eq)]
    struct ArtifactDiff {
        /// Bytes added (new is larger by this amount).
        added_bytes: i64,
        /// True when the two payloads are byte-identical.
        identical: bool,
    }

    fn artifact_diff(a: &[u8], b: &[u8]) -> ArtifactDiff {
        let identical = a == b;
        let added_bytes = b.len() as i64 - a.len() as i64;
        ArtifactDiff { added_bytes, identical }
    }

    #[test]
    fn artifact_diff_identical_payloads_empty_diff() {
        let payload = b"hello world".to_vec();
        let diff = artifact_diff(&payload, &payload);
        assert!(diff.identical, "identical payloads must produce empty diff");
        assert_eq!(diff.added_bytes, 0);
    }

    #[test]
    fn artifact_diff_size_differs_reports_size_diff() {
        let a = vec![0u8; 100];
        let b = vec![0u8; 200];
        let diff = artifact_diff(&a, &b);
        assert!(!diff.identical, "different-size payloads must not be identical");
        assert_ne!(diff.added_bytes, 0, "size diff must be non-zero");
    }

    #[test]
    fn artifact_diff_larger_new_reports_added_bytes() {
        let a = vec![0u8; 50];
        let b = vec![0u8; 80];
        let diff = artifact_diff(&a, &b);
        assert_eq!(diff.added_bytes, 30, "added_bytes must be 30 when new is 30 bytes larger");
        assert!(!diff.identical);
    }

    #[test]
    fn artifact_diff_smaller_new_reports_negative_added_bytes() {
        let a = vec![0u8; 80];
        let b = vec![0u8; 50];
        let diff = artifact_diff(&a, &b);
        assert_eq!(diff.added_bytes, -30, "added_bytes must be -30 when new is 30 bytes smaller");
        assert!(!diff.identical);
    }

    #[test]
    fn artifact_diff_same_content_hash_null_diff() {
        // Same content bytes = null diff regardless of how they were produced.
        let content = b"NOM-ARTIFACT-v1".to_vec();
        let hash_a = {
            let mut h: u64 = 14695981039346656037;
            for &b in &content { h ^= b as u64; h = h.wrapping_mul(1099511628211); }
            h
        };
        let hash_b = {
            let mut h: u64 = 14695981039346656037;
            for &b in &content { h ^= b as u64; h = h.wrapping_mul(1099511628211); }
            h
        };
        assert_eq!(hash_a, hash_b, "same content must produce same hash");
        let diff = artifact_diff(&content, &content);
        assert!(diff.identical, "same-hash content must yield a null diff");
    }
}
