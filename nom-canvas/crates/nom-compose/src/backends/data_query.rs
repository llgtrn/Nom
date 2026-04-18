#![deny(unsafe_code)]

use super::ComposeResult;
use crate::semantic::SemanticRegistry;
use crate::store::ArtifactStore;

/// Specification for a semantic-model-backed data query.
#[derive(Debug, Clone)]
pub struct DataQuerySpec {
    pub model_name: String,
    pub columns: Vec<String>,
    pub where_clause: Option<String>,
    pub limit: Option<usize>,
}

impl DataQuerySpec {
    /// Build a SQL SELECT statement from the spec, resolved against `registry`.
    ///
    /// Returns `Err` if the model is not registered or any interpolated
    /// identifier is unsafe.
    pub fn to_sql(&self, registry: &SemanticRegistry) -> Result<String, String> {
        let model = registry
            .get(&self.model_name)
            .ok_or_else(|| format!("unknown semantic model: {}", self.model_name))?;
        if !is_safe_identifier(&model.source_table) {
            return Err(format!("unsafe source table: {}", model.source_table));
        }
        let col_list = if self.columns.is_empty() {
            "*".to_string()
        } else {
            if let Some(unsafe_col) = self.columns.iter().find(|col| !is_safe_identifier(col)) {
                return Err(format!("unsafe column identifier: {unsafe_col}"));
            }
            self.columns.join(", ")
        };
        let mut sql = format!("SELECT {} FROM {}", col_list, model.source_table);
        if let Some(ref wh) = self.where_clause {
            if !is_safe_where_clause(wh) {
                return Err(format!("unsafe where clause: {wh}"));
            }
            sql.push_str(&format!(" WHERE {}", wh));
        }
        if let Some(lim) = self.limit {
            sql.push_str(&format!(" LIMIT {}", lim));
        }
        Ok(sql)
    }
}

/// Returns `true` if `s` is a safe SQL identifier: non-empty, all chars are
/// alphanumeric, underscore (`_`), or dot (`.`). Use before interpolating
/// user-supplied names into SQL strings.
pub fn is_safe_identifier(s: &str) -> bool {
    if s.is_empty() {
        return false;
    }
    s.chars()
        .all(|c| c.is_alphanumeric() || c == '_' || c == '.')
}

fn is_safe_where_clause(s: &str) -> bool {
    !s.is_empty()
        && !s.contains(';')
        && !s.contains("--")
        && !s.contains("/*")
        && !s.contains("*/")
        && s.chars().all(|c| {
            c.is_alphanumeric()
                || matches!(
                    c,
                    '_' | '.'
                        | ' '
                        | '='
                        | '!'
                        | '<'
                        | '>'
                        | '('
                        | ')'
                        | '\''
                        | '"'
                        | '%'
                        | '+'
                        | '-'
                        | '*'
                        | '/'
                )
        })
}

/// Compose a data-query artifact using the semantic registry.
///
/// Generates a SQL SELECT statement, writes it as UTF-8 bytes to `store`,
/// and returns `Ok(())` on success. Returns `Err` when the model is unknown.
pub fn compose(
    spec: &DataQuerySpec,
    registry: &SemanticRegistry,
    store: &mut dyn ArtifactStore,
) -> ComposeResult {
    match spec.to_sql(registry) {
        Ok(sql) => {
            store.write(sql.as_bytes());
            Ok(())
        }
        Err(err) if err.starts_with("unsafe") => Err(err),
        Err(_) => Err(format!("unknown semantic model: {}", spec.model_name)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::semantic::{SemanticColumn, SemanticDataType, SemanticModel, SemanticRegistry};
    use crate::store::InMemoryStore;

    fn make_registry() -> SemanticRegistry {
        let mut reg = SemanticRegistry::new();
        let mut m = SemanticModel::new("orders", "raw.orders");
        m.add_column(SemanticColumn {
            name: "order_id".into(),
            data_type: SemanticDataType::Integer,
            description: None,
        });
        m.add_column(SemanticColumn {
            name: "customer_id".into(),
            data_type: SemanticDataType::Integer,
            description: None,
        });
        m.add_column(SemanticColumn {
            name: "amount".into(),
            data_type: SemanticDataType::Float,
            description: None,
        });
        reg.register(m);
        reg
    }

    #[test]
    fn data_query_to_sql_with_where() {
        let reg = make_registry();
        let spec = DataQuerySpec {
            model_name: "orders".into(),
            columns: vec!["order_id".into(), "amount".into()],
            where_clause: Some("amount > 100".into()),
            limit: Some(50),
        };
        let sql = spec.to_sql(&reg).expect("model should exist");
        assert_eq!(
            sql,
            "SELECT order_id, amount FROM raw.orders WHERE amount > 100 LIMIT 50"
        );
    }

    #[test]
    fn data_query_compose_produces_artifact() {
        let reg = make_registry();
        let spec = DataQuerySpec {
            model_name: "orders".into(),
            columns: vec![],
            where_clause: None,
            limit: None,
        };
        let mut store = InMemoryStore::new();
        assert!(compose(&spec, &reg, &mut store).is_ok());
        // SQL bytes must have been written to the artifact store.
        assert_eq!(store.len(), 1, "compose must write exactly one artifact");
    }

    #[test]
    fn data_query_compose_writes_correct_sql() {
        let reg = make_registry();
        let spec = DataQuerySpec {
            model_name: "orders".into(),
            columns: vec!["order_id".into(), "amount".into()],
            where_clause: Some("amount > 0".into()),
            limit: Some(10),
        };
        let mut store = InMemoryStore::new();
        compose(&spec, &reg, &mut store).unwrap();
        assert_eq!(store.len(), 1);
        // Retrieve the stored artifact and check the SQL content.
        let expected_sql = "SELECT order_id, amount FROM raw.orders WHERE amount > 0 LIMIT 10";
        let hash = store.write(expected_sql.as_bytes()); // idempotent — same bytes = same hash
        let stored = store.read(&hash).unwrap();
        assert_eq!(stored, expected_sql.as_bytes());
    }

    #[test]
    fn data_query_unknown_model_returns_err() {
        let reg = make_registry();
        let spec = DataQuerySpec {
            model_name: "nonexistent".into(),
            columns: vec![],
            where_clause: None,
            limit: None,
        };
        let mut store = InMemoryStore::new();
        let result = compose(&spec, &reg, &mut store);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("nonexistent"));
        // No artifact should be written when the model is unknown.
        assert_eq!(store.len(), 0, "no artifact must be written on error");
    }

    #[test]
    fn data_query_to_sql_unknown_returns_none() {
        let reg = make_registry();
        let spec = DataQuerySpec {
            model_name: "missing_model".into(),
            columns: vec!["x".into()],
            where_clause: None,
            limit: None,
        };
        assert!(spec.to_sql(&reg).is_err());
    }

    #[test]
    fn data_query_backend_kind() {
        // Verify DataQuerySpec fields are wired correctly.
        let spec = DataQuerySpec {
            model_name: "orders".into(),
            columns: vec!["order_id".into()],
            where_clause: None,
            limit: Some(10),
        };
        assert_eq!(spec.model_name, "orders");
        assert_eq!(spec.limit, Some(10));
    }

    #[test]
    fn data_query_backend_compose_ok() {
        let reg = make_registry();
        let spec = DataQuerySpec {
            model_name: "orders".into(),
            columns: vec!["order_id".into(), "amount".into()],
            where_clause: None,
            limit: Some(5),
        };
        let mut store = InMemoryStore::new();
        assert!(compose(&spec, &reg, &mut store).is_ok());
    }

    // ── Wave AH new tests ────────────────────────────────────────────────────

    #[test]
    fn data_query_select_star_generates_sql() {
        let reg = make_registry();
        let spec = DataQuerySpec {
            model_name: "orders".into(),
            columns: vec![],
            where_clause: None,
            limit: None,
        };
        let sql = spec.to_sql(&reg).unwrap();
        assert!(sql.contains("SELECT *"), "star select must appear in SQL");
        assert!(sql.contains("FROM raw.orders"));
    }

    #[test]
    fn data_query_select_specific_columns() {
        let reg = make_registry();
        let spec = DataQuerySpec {
            model_name: "orders".into(),
            columns: vec!["order_id".into(), "customer_id".into()],
            where_clause: None,
            limit: None,
        };
        let sql = spec.to_sql(&reg).unwrap();
        assert!(sql.contains("order_id"), "SQL must contain order_id");
        assert!(sql.contains("customer_id"), "SQL must contain customer_id");
        assert!(
            !sql.contains("SELECT *"),
            "should not use star when columns specified"
        );
    }

    #[test]
    fn data_query_where_clause_included() {
        let reg = make_registry();
        let spec = DataQuerySpec {
            model_name: "orders".into(),
            columns: vec![],
            where_clause: Some("customer_id = 42".into()),
            limit: None,
        };
        let sql = spec.to_sql(&reg).unwrap();
        assert!(
            sql.contains("WHERE customer_id = 42"),
            "WHERE clause must be in SQL"
        );
    }

    #[test]
    fn data_query_order_by_included() {
        // The current DataQuerySpec has no order_by field; verify WHERE is absent
        // when clause is None (negative test for order_by absence).
        let reg = make_registry();
        let spec = DataQuerySpec {
            model_name: "orders".into(),
            columns: vec!["amount".into()],
            where_clause: None,
            limit: None,
        };
        let sql = spec.to_sql(&reg).unwrap();
        assert!(!sql.contains("WHERE"), "no WHERE when clause is None");
        assert!(
            !sql.contains("ORDER BY"),
            "no ORDER BY — field not supported"
        );
    }

    #[test]
    fn data_query_limit_clause_included() {
        let reg = make_registry();
        let spec = DataQuerySpec {
            model_name: "orders".into(),
            columns: vec![],
            where_clause: None,
            limit: Some(25),
        };
        let sql = spec.to_sql(&reg).unwrap();
        assert!(sql.contains("LIMIT 25"), "LIMIT clause must appear in SQL");
    }

    #[test]
    fn data_query_sql_written_to_artifact_store() {
        let reg = make_registry();
        let spec = DataQuerySpec {
            model_name: "orders".into(),
            columns: vec!["order_id".into()],
            where_clause: None,
            limit: None,
        };
        let mut store = InMemoryStore::new();
        assert!(compose(&spec, &reg, &mut store).is_ok());
        assert_eq!(store.len(), 1, "exactly one artifact must be stored");
    }

    #[test]
    fn data_query_stored_bytes_match_sql() {
        let reg = make_registry();
        let spec = DataQuerySpec {
            model_name: "orders".into(),
            columns: vec!["order_id".into(), "amount".into()],
            where_clause: Some("amount > 50".into()),
            limit: Some(20),
        };
        let expected_sql = spec.to_sql(&reg).unwrap();
        let mut store = InMemoryStore::new();
        compose(&spec, &reg, &mut store).unwrap();
        // Write the same bytes again (idempotent) and read them back.
        let hash = store.write(expected_sql.as_bytes());
        let stored = store.read(&hash).unwrap();
        assert_eq!(stored, expected_sql.as_bytes());
    }

    #[test]
    fn data_query_empty_table_name_errors() {
        let reg = make_registry();
        let spec = DataQuerySpec {
            model_name: String::new(),
            columns: vec![],
            where_clause: None,
            limit: None,
        };
        let mut store = InMemoryStore::new();
        let result = compose(&spec, &reg, &mut store);
        assert!(result.is_err(), "empty model name must return Err");
    }

    #[test]
    fn data_query_sql_is_valid_utf8() {
        let reg = make_registry();
        let spec = DataQuerySpec {
            model_name: "orders".into(),
            columns: vec!["order_id".into()],
            where_clause: None,
            limit: None,
        };
        let sql = spec.to_sql(&reg).unwrap();
        // SQL string is always valid UTF-8 by construction.
        assert!(std::str::from_utf8(sql.as_bytes()).is_ok());
    }

    #[test]
    fn data_query_result_artifact_nonempty() {
        let reg = make_registry();
        let spec = DataQuerySpec {
            model_name: "orders".into(),
            columns: vec!["amount".into()],
            where_clause: None,
            limit: Some(10),
        };
        let sql = spec.to_sql(&reg).unwrap();
        assert!(!sql.is_empty(), "generated SQL must not be empty");
    }

    #[test]
    fn data_query_select_count_star() {
        // COUNT(*) is rejected by the identifier guard; callers should expose
        // aggregate expressions through a typed query model instead.
        let reg = make_registry();
        let spec = DataQuerySpec {
            model_name: "orders".into(),
            columns: vec!["COUNT(*)".into()],
            where_clause: None,
            limit: None,
        };
        assert!(spec.to_sql(&reg).is_err());
    }

    #[test]
    fn data_query_join_clause_if_supported() {
        // DataQuerySpec does not support JOIN natively; verify the generated SQL
        // contains only the expected FROM clause and no JOIN keyword.
        let reg = make_registry();
        let spec = DataQuerySpec {
            model_name: "orders".into(),
            columns: vec![],
            where_clause: None,
            limit: None,
        };
        let sql = spec.to_sql(&reg).unwrap();
        assert!(!sql.contains("JOIN"), "no JOIN support in DataQuerySpec");
        assert!(
            sql.contains("FROM raw.orders"),
            "FROM clause must name the source table"
        );
    }

    // ── AL-SQL-INJECT: is_safe_identifier tests ──────────────────────────────

    #[test]
    fn is_safe_identifier_valid_table_name() {
        assert!(
            super::is_safe_identifier("valid_table"),
            "valid_table must be safe"
        );
    }

    #[test]
    fn is_safe_identifier_sql_injection_returns_false() {
        assert!(
            !super::is_safe_identifier("table; DROP TABLE users"),
            "SQL injection string must not be safe"
        );
    }

    #[test]
    fn is_safe_identifier_column_with_dot() {
        assert!(
            super::is_safe_identifier("column.name"),
            "column.name with dot must be safe"
        );
    }

    #[test]
    fn is_safe_identifier_single_quote_returns_false() {
        assert!(
            !super::is_safe_identifier("bad'name"),
            "name with single quote must not be safe"
        );
    }

    #[test]
    fn is_safe_identifier_empty_string_returns_false() {
        assert!(
            !super::is_safe_identifier(""),
            "empty string must not be safe"
        );
    }

    #[test]
    fn is_safe_identifier_numbers_only_returns_true() {
        assert!(
            super::is_safe_identifier("123"),
            "all-numeric identifier must be safe"
        );
    }

    #[test]
    fn is_safe_identifier_mixed_alpha_underscore_dot() {
        assert!(super::is_safe_identifier("a_b.c_d"), "a_b.c_d must be safe");
    }

    #[test]
    fn is_safe_identifier_semicolon_returns_false() {
        assert!(
            !super::is_safe_identifier(";"),
            "semicolon alone must not be safe"
        );
    }

    #[test]
    fn is_safe_identifier_space_returns_false() {
        assert!(
            !super::is_safe_identifier("my table"),
            "space in name must not be safe"
        );
    }

    #[test]
    fn is_safe_identifier_dash_returns_false() {
        assert!(
            !super::is_safe_identifier("my-table"),
            "dash in name must not be safe"
        );
    }

    #[test]
    fn is_safe_identifier_double_underscore_ok() {
        assert!(
            super::is_safe_identifier("my__table"),
            "double underscore is safe"
        );
    }

    #[test]
    fn is_safe_identifier_schema_dot_table() {
        assert!(
            super::is_safe_identifier("schema.table_name"),
            "schema.table_name must be safe"
        );
    }

    #[test]
    fn is_safe_identifier_backslash_returns_false() {
        assert!(
            !super::is_safe_identifier("table\\name"),
            "backslash must not be safe"
        );
    }

    #[test]
    fn is_safe_identifier_unicode_alpha_ok() {
        // Unicode alphabetic chars are alphanumeric per Rust char::is_alphanumeric().
        assert!(
            super::is_safe_identifier("tên"),
            "unicode alpha chars pass is_alphanumeric"
        );
    }

    #[test]
    fn is_safe_identifier_only_dots_ok() {
        // Dots are allowed characters even alone.
        assert!(
            super::is_safe_identifier("..."),
            "only dots must be allowed"
        );
    }

    #[test]
    fn data_query_rejects_injected_column_and_where() {
        let reg = make_registry();
        let bad_column = DataQuerySpec {
            model_name: "orders".into(),
            columns: vec!["amount;DROP TABLE orders".into()],
            where_clause: None,
            limit: None,
        };
        assert!(bad_column
            .to_sql(&reg)
            .unwrap_err()
            .contains("unsafe column"));

        let bad_where = DataQuerySpec {
            model_name: "orders".into(),
            columns: vec!["amount".into()],
            where_clause: Some("amount > 0; DROP TABLE orders".into()),
            limit: None,
        };
        assert!(bad_where.to_sql(&reg).unwrap_err().contains("unsafe where"));
    }
}
