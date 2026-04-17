//! Semantic data-layer model (MDL-like).
//!
//! An end-user describes business entities, derived metrics, and entity
//! relationships.  An LLM grounded against this model can then generate
//! syntactically-valid SQL/Cypher without hallucinating tables or columns.
//!
//! This module is pure data + validation.  LLM grounding + SQL generation
//! live in the data/query backend (Phase 4 Part C).
#![deny(unsafe_code)]

use std::collections::HashMap;

#[derive(Clone, Debug, PartialEq)]
pub enum ColumnType {
    Text,
    Number,
    Integer,
    Date,
    Timestamp,
    Boolean,
    Json,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ColumnSpec {
    pub name: String,
    pub kind: ColumnType,
    pub business_meaning: Option<String>,
    pub nullable: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub struct SemanticEntity {
    pub name: String,
    pub table: String,
    pub columns: Vec<ColumnSpec>,
    pub business_meaning: String,
}

impl SemanticEntity {
    pub fn new(
        name: impl Into<String>,
        table: impl Into<String>,
        business_meaning: impl Into<String>,
    ) -> Self {
        Self {
            name: name.into(),
            table: table.into(),
            columns: Vec::new(),
            business_meaning: business_meaning.into(),
        }
    }

    pub fn with_column(mut self, col: ColumnSpec) -> Self {
        self.columns.push(col);
        self
    }

    pub fn column(&self, name: &str) -> Option<&ColumnSpec> {
        self.columns.iter().find(|c| c.name == name)
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct DerivedMetric {
    pub name: String,
    /// SQL-style formula, e.g. "SUM(revenue)".  The MDL layer does NOT parse this.
    pub formula: String,
    pub grouping: Vec<String>,
    pub filter: Option<String>,
    pub business_meaning: String,
}

impl DerivedMetric {
    pub fn new(
        name: impl Into<String>,
        formula: impl Into<String>,
        business_meaning: impl Into<String>,
    ) -> Self {
        Self {
            name: name.into(),
            formula: formula.into(),
            grouping: Vec::new(),
            filter: None,
            business_meaning: business_meaning.into(),
        }
    }

    pub fn group_by(mut self, col: impl Into<String>) -> Self {
        self.grouping.push(col.into());
        self
    }

    pub fn with_filter(mut self, filter: impl Into<String>) -> Self {
        self.filter = Some(filter.into());
        self
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum RelationKind {
    OneToOne,
    OneToMany,
    ManyToMany,
}

#[derive(Clone, Debug, PartialEq)]
pub struct EntityRelation {
    pub from: String,
    pub to: String,
    pub kind: RelationKind,
    pub join_keys: Vec<(String, String)>,
}

impl EntityRelation {
    pub fn new(
        from: impl Into<String>,
        to: impl Into<String>,
        kind: RelationKind,
    ) -> Self {
        Self {
            from: from.into(),
            to: to.into(),
            kind,
            join_keys: Vec::new(),
        }
    }

    pub fn on(mut self, from_col: impl Into<String>, to_col: impl Into<String>) -> Self {
        self.join_keys.push((from_col.into(), to_col.into()));
        self
    }
}

#[derive(Clone, Debug, PartialEq, Default)]
pub struct SemanticModel {
    pub entities: HashMap<String, SemanticEntity>,
    pub metrics: HashMap<String, DerivedMetric>,
    pub relationships: Vec<EntityRelation>,
}

impl SemanticModel {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add_entity(&mut self, entity: SemanticEntity) -> Result<(), SemanticError> {
        if self.entities.contains_key(&entity.name) {
            return Err(SemanticError::DuplicateEntity(entity.name));
        }
        self.entities.insert(entity.name.clone(), entity);
        Ok(())
    }

    pub fn add_metric(&mut self, metric: DerivedMetric) -> Result<(), SemanticError> {
        if self.metrics.contains_key(&metric.name) {
            return Err(SemanticError::DuplicateMetric(metric.name));
        }
        self.metrics.insert(metric.name.clone(), metric);
        Ok(())
    }

    pub fn add_relationship(&mut self, rel: EntityRelation) {
        self.relationships.push(rel);
    }

    pub fn entity(&self, name: &str) -> Option<&SemanticEntity> {
        self.entities.get(name)
    }

    pub fn metric(&self, name: &str) -> Option<&DerivedMetric> {
        self.metrics.get(name)
    }

    /// Verify all relationships reference existing entities.
    pub fn validate(&self) -> Result<(), SemanticError> {
        for rel in &self.relationships {
            if !self.entities.contains_key(&rel.from) {
                return Err(SemanticError::UnknownEntity(rel.from.clone()));
            }
            if !self.entities.contains_key(&rel.to) {
                return Err(SemanticError::UnknownEntity(rel.to.clone()));
            }
            if rel.join_keys.is_empty() {
                return Err(SemanticError::MissingJoinKeys {
                    from: rel.from.clone(),
                    to: rel.to.clone(),
                });
            }
        }
        Ok(())
    }
}

#[derive(Debug, thiserror::Error)]
pub enum SemanticError {
    #[error("entity '{0}' already exists")]
    DuplicateEntity(String),
    #[error("metric '{0}' already exists")]
    DuplicateMetric(String),
    #[error("unknown entity '{0}'")]
    UnknownEntity(String),
    #[error("relation {from}\u{2194}{to} requires at least one join key")]
    MissingJoinKeys { from: String, to: String },
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_entity(name: &str) -> SemanticEntity {
        SemanticEntity::new(name, format!("{}_table", name), format!("{} entity", name))
    }

    #[test]
    fn entity_new_and_with_column() {
        let col = ColumnSpec {
            name: "id".into(),
            kind: ColumnType::Integer,
            business_meaning: Some("primary key".into()),
            nullable: false,
        };
        let e = SemanticEntity::new("Order", "orders", "An order placed by a customer")
            .with_column(col);
        assert_eq!(e.name, "Order");
        assert_eq!(e.table, "orders");
        assert_eq!(e.columns.len(), 1);
        assert_eq!(e.columns[0].name, "id");
    }

    #[test]
    fn entity_column_hit_and_miss() {
        let col = ColumnSpec {
            name: "amount".into(),
            kind: ColumnType::Number,
            business_meaning: None,
            nullable: true,
        };
        let e = SemanticEntity::new("Order", "orders", "").with_column(col);
        assert!(e.column("amount").is_some());
        assert_eq!(e.column("amount").unwrap().kind, ColumnType::Number);
        assert!(e.column("nonexistent").is_none());
    }

    #[test]
    fn derived_metric_builder_chain() {
        let m = DerivedMetric::new("total_revenue", "SUM(revenue)", "Total revenue across orders")
            .group_by("region")
            .group_by("month")
            .with_filter("status = 'completed'");
        assert_eq!(m.name, "total_revenue");
        assert_eq!(m.formula, "SUM(revenue)");
        assert_eq!(m.grouping, vec!["region", "month"]);
        assert_eq!(m.filter.as_deref(), Some("status = 'completed'"));
    }

    #[test]
    fn derived_metric_no_filter_by_default() {
        let m = DerivedMetric::new("count_orders", "COUNT(*)", "Total orders");
        assert!(m.filter.is_none());
        assert!(m.grouping.is_empty());
    }

    #[test]
    fn entity_relation_on_appends_join_key() {
        let rel = EntityRelation::new("Order", "Customer", RelationKind::ManyToMany)
            .on("customer_id", "id")
            .on("region_id", "region_id");
        assert_eq!(rel.join_keys.len(), 2);
        assert_eq!(rel.join_keys[0], ("customer_id".to_string(), "id".to_string()));
        assert_eq!(rel.join_keys[1], ("region_id".to_string(), "region_id".to_string()));
    }

    #[test]
    fn semantic_model_new_is_empty() {
        let m = SemanticModel::new();
        assert!(m.entities.is_empty());
        assert!(m.metrics.is_empty());
        assert!(m.relationships.is_empty());
    }

    #[test]
    fn add_entity_success_and_duplicate_rejected() {
        let mut m = SemanticModel::new();
        assert!(m.add_entity(make_entity("Order")).is_ok());
        let err = m.add_entity(make_entity("Order")).unwrap_err();
        assert!(matches!(err, SemanticError::DuplicateEntity(n) if n == "Order"));
        assert_eq!(m.entities.len(), 1);
    }

    #[test]
    fn add_metric_success_and_duplicate_rejected() {
        let mut m = SemanticModel::new();
        let metric = DerivedMetric::new("rev", "SUM(revenue)", "revenue");
        assert!(m.add_metric(metric.clone()).is_ok());
        let err = m.add_metric(metric).unwrap_err();
        assert!(matches!(err, SemanticError::DuplicateMetric(n) if n == "rev"));
        assert_eq!(m.metrics.len(), 1);
    }

    #[test]
    fn add_relationship_appends() {
        let mut m = SemanticModel::new();
        let rel = EntityRelation::new("A", "B", RelationKind::OneToMany).on("a_id", "id");
        m.add_relationship(rel);
        assert_eq!(m.relationships.len(), 1);
    }

    #[test]
    fn validate_ok_for_valid_model() {
        let mut m = SemanticModel::new();
        m.add_entity(make_entity("Order")).unwrap();
        m.add_entity(make_entity("Customer")).unwrap();
        let rel = EntityRelation::new("Order", "Customer", RelationKind::ManyToMany)
            .on("customer_id", "id");
        m.add_relationship(rel);
        assert!(m.validate().is_ok());
    }

    #[test]
    fn validate_unknown_entity_from() {
        let mut m = SemanticModel::new();
        m.add_entity(make_entity("Customer")).unwrap();
        let rel = EntityRelation::new("Ghost", "Customer", RelationKind::OneToOne)
            .on("id", "id");
        m.add_relationship(rel);
        let err = m.validate().unwrap_err();
        assert!(matches!(err, SemanticError::UnknownEntity(n) if n == "Ghost"));
    }

    #[test]
    fn validate_unknown_entity_to() {
        let mut m = SemanticModel::new();
        m.add_entity(make_entity("Order")).unwrap();
        let rel = EntityRelation::new("Order", "Missing", RelationKind::OneToMany)
            .on("id", "id");
        m.add_relationship(rel);
        let err = m.validate().unwrap_err();
        assert!(matches!(err, SemanticError::UnknownEntity(n) if n == "Missing"));
    }

    #[test]
    fn validate_missing_join_keys() {
        let mut m = SemanticModel::new();
        m.add_entity(make_entity("Order")).unwrap();
        m.add_entity(make_entity("Customer")).unwrap();
        let rel = EntityRelation::new("Order", "Customer", RelationKind::OneToOne);
        m.add_relationship(rel);
        let err = m.validate().unwrap_err();
        assert!(matches!(err, SemanticError::MissingJoinKeys { from, to } if from == "Order" && to == "Customer"));
    }
}
