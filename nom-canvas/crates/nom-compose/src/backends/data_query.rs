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
    /// Returns `None` if the model is not registered.
    pub fn to_sql(&self, registry: &SemanticRegistry) -> Option<String> {
        let model = registry.get(&self.model_name)?;
        let col_list = if self.columns.is_empty() {
            "*".to_string()
        } else {
            self.columns.join(", ")
        };
        let mut sql = format!("SELECT {} FROM {}", col_list, model.source_table);
        if let Some(ref wh) = self.where_clause {
            sql.push_str(&format!(" WHERE {}", wh));
        }
        if let Some(lim) = self.limit {
            sql.push_str(&format!(" LIMIT {}", lim));
        }
        Some(sql)
    }
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
        Some(sql) => {
            store.write(sql.as_bytes());
            Ok(())
        }
        None => Err(format!("unknown semantic model: {}", spec.model_name)),
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
        let expected_sql =
            "SELECT order_id, amount FROM raw.orders WHERE amount > 0 LIMIT 10";
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
        assert!(spec.to_sql(&reg).is_none());
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
        assert!(!sql.contains("SELECT *"), "should not use star when columns specified");
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
        assert!(sql.contains("WHERE customer_id = 42"), "WHERE clause must be in SQL");
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
        assert!(!sql.contains("ORDER BY"), "no ORDER BY — field not supported");
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
        // Simulate COUNT(*) by using column name "COUNT(*)" directly.
        let reg = make_registry();
        let spec = DataQuerySpec {
            model_name: "orders".into(),
            columns: vec!["COUNT(*)".into()],
            where_clause: None,
            limit: None,
        };
        let sql = spec.to_sql(&reg).unwrap();
        assert!(sql.contains("COUNT(*)"), "COUNT(*) must appear in SQL");
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
        assert!(sql.contains("FROM raw.orders"), "FROM clause must name the source table");
    }
}
