#![deny(unsafe_code)]

use super::ComposeResult;
use crate::semantic::SemanticRegistry;

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
/// Succeeds when the model exists and a valid SQL string can be generated.
/// Returns `Err` when the model is unknown.
pub fn compose(spec: &DataQuerySpec, registry: &SemanticRegistry) -> ComposeResult {
    match spec.to_sql(registry) {
        Some(_sql) => Ok(()),
        None => Err(format!("unknown semantic model: {}", spec.model_name)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::semantic::{SemanticColumn, SemanticDataType, SemanticModel, SemanticRegistry};

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
        assert!(compose(&spec, &reg).is_ok());
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
        let result = compose(&spec, &reg);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("nonexistent"));
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
        assert!(compose(&spec, &reg).is_ok());
    }
}
