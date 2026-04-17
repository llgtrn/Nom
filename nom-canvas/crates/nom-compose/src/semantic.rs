#![deny(unsafe_code)]

/// A column in a semantic model (WrenAI MDL pattern).
#[derive(Debug, Clone)]
pub struct SemanticColumn {
    pub name: String,
    pub data_type: SemanticDataType,
    pub description: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SemanticDataType { String, Integer, Float, Boolean, Date, Timestamp, Json }

impl SemanticDataType {
    pub fn from_str(s: &str) -> Option<Self> {
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
        Self { name: name.into(), source_table: source.into(), columns: Vec::new(), description: None }
    }

    pub fn add_column(&mut self, col: SemanticColumn) -> &mut Self {
        self.columns.push(col);
        self
    }

    pub fn column(&self, name: &str) -> Option<&SemanticColumn> {
        self.columns.iter().find(|c| c.name == name)
    }

    pub fn column_count(&self) -> usize { self.columns.len() }

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
    pub fn new() -> Self { Self::default() }
    pub fn register(&mut self, model: SemanticModel) { self.models.push(model); }
    pub fn get(&self, name: &str) -> Option<&SemanticModel> { self.models.iter().find(|m| m.name == name) }
    pub fn model_count(&self) -> usize { self.models.len() }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn semantic_model_to_sql() {
        let mut m = SemanticModel::new("users", "raw.users");
        m.add_column(SemanticColumn { name: "id".into(), data_type: SemanticDataType::Integer, description: None });
        m.add_column(SemanticColumn { name: "name".into(), data_type: SemanticDataType::String, description: None });
        assert_eq!(m.to_select_sql(), "SELECT id, name FROM raw.users");
    }
    #[test]
    fn semantic_model_empty_columns_select_star() {
        let m = SemanticModel::new("t", "raw.t");
        assert_eq!(m.to_select_sql(), "SELECT * FROM raw.t");
    }
    #[test]
    fn semantic_data_type_from_str() {
        assert_eq!(SemanticDataType::from_str("varchar"), Some(SemanticDataType::String));
        assert_eq!(SemanticDataType::from_str("jsonb"), Some(SemanticDataType::Json));
        assert_eq!(SemanticDataType::from_str("unknown"), None);
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
        m.add_column(SemanticColumn { name: "ts".into(), data_type: SemanticDataType::Timestamp, description: None });
        let sql = m.to_select_sql();
        assert!(!sql.is_empty());
        assert!(sql.contains("raw.events"));
    }

    #[test]
    fn semantic_model_table_name() {
        let m = SemanticModel::new("sessions", "raw.sessions");
        assert_eq!(m.source_table, "raw.sessions");
    }
}
