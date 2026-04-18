#![deny(unsafe_code)]

/// Data composition primitives: source registration, query dispatch, and result handling.

#[derive(Debug, Clone, PartialEq)]
pub enum DataSourceKind {
    Database,
    Api,
    File,
    Stream,
    Computed,
}

impl DataSourceKind {
    pub fn is_realtime(&self) -> bool {
        matches!(self, DataSourceKind::Stream | DataSourceKind::Api)
    }

    pub fn source_label(&self) -> &'static str {
        match self {
            DataSourceKind::Database => "db",
            DataSourceKind::Api => "api",
            DataSourceKind::File => "file",
            DataSourceKind::Stream => "stream",
            DataSourceKind::Computed => "computed",
        }
    }
}

#[derive(Debug, Clone)]
pub struct DataSource {
    pub id: u64,
    pub kind: DataSourceKind,
    pub uri: String,
    pub schema: Vec<String>,
}

impl DataSource {
    pub fn has_field(&self, name: &str) -> bool {
        self.schema.iter().any(|f| f == name)
    }

    pub fn field_count(&self) -> usize {
        self.schema.len()
    }
}

#[derive(Debug, Clone)]
pub struct DataQuery {
    pub source_id: u64,
    pub fields: Vec<String>,
    pub limit: Option<usize>,
}

impl DataQuery {
    pub fn is_selective(&self) -> bool {
        !self.fields.is_empty()
    }

    pub fn effective_limit(&self) -> usize {
        self.limit.unwrap_or(1000)
    }
}

#[derive(Debug, Clone)]
pub struct DataResult {
    pub row_count: u64,
    pub columns: Vec<String>,
    pub truncated: bool,
}

impl DataResult {
    pub fn is_empty(&self) -> bool {
        self.row_count == 0
    }

    pub fn column_count(&self) -> usize {
        self.columns.len()
    }
}

#[derive(Debug, Default)]
pub struct DataComposer {
    pub sources: Vec<DataSource>,
}

impl DataComposer {
    pub fn register(&mut self, s: DataSource) {
        self.sources.push(s);
    }

    pub fn find_source(&self, id: u64) -> Option<&DataSource> {
        self.sources.iter().find(|s| s.id == id)
    }

    pub fn compose(&self, query: &DataQuery) -> DataResult {
        let source = self.find_source(query.source_id);
        let columns = if query.is_selective() {
            query.fields.clone()
        } else {
            source.map(|s| s.schema.clone()).unwrap_or_default()
        };
        let row_count = query.effective_limit() as u64;
        let truncated = query.limit.is_some();
        DataResult { row_count, columns, truncated }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_source(id: u64) -> DataSource {
        DataSource {
            id,
            kind: DataSourceKind::Database,
            uri: "db://localhost/test".to_string(),
            schema: vec!["id".to_string(), "name".to_string(), "email".to_string()],
        }
    }

    // Test 1: DataSourceKind::Stream and Api are realtime; others are not.
    #[test]
    fn source_kind_is_realtime() {
        assert!(DataSourceKind::Stream.is_realtime());
        assert!(DataSourceKind::Api.is_realtime());
        assert!(!DataSourceKind::Database.is_realtime());
        assert!(!DataSourceKind::File.is_realtime());
        assert!(!DataSourceKind::Computed.is_realtime());
    }

    // Test 2: has_field returns true for existing schema fields only.
    #[test]
    fn source_has_field() {
        let src = make_source(1);
        assert!(src.has_field("name"));
        assert!(!src.has_field("missing"));
    }

    // Test 3: field_count matches schema length.
    #[test]
    fn source_field_count() {
        let src = make_source(1);
        assert_eq!(src.field_count(), 3);
    }

    // Test 4: is_selective is false when fields is empty, true otherwise.
    #[test]
    fn query_is_selective() {
        let empty_q = DataQuery { source_id: 1, fields: vec![], limit: None };
        assert!(!empty_q.is_selective());
        let sel_q = DataQuery { source_id: 1, fields: vec!["id".to_string()], limit: None };
        assert!(sel_q.is_selective());
    }

    // Test 5: effective_limit returns 1000 when limit is None.
    #[test]
    fn query_effective_limit_default() {
        let q = DataQuery { source_id: 1, fields: vec![], limit: None };
        assert_eq!(q.effective_limit(), 1000);
        let q2 = DataQuery { source_id: 1, fields: vec![], limit: Some(50) };
        assert_eq!(q2.effective_limit(), 50);
    }

    // Test 6: DataResult::is_empty is true only when row_count == 0.
    #[test]
    fn result_is_empty() {
        let empty = DataResult { row_count: 0, columns: vec![], truncated: false };
        assert!(empty.is_empty());
        let nonempty = DataResult { row_count: 5, columns: vec![], truncated: false };
        assert!(!nonempty.is_empty());
    }

    // Test 7: column_count matches columns length.
    #[test]
    fn result_column_count() {
        let r = DataResult {
            row_count: 1,
            columns: vec!["a".to_string(), "b".to_string()],
            truncated: false,
        };
        assert_eq!(r.column_count(), 2);
    }

    // Test 8: composer find_source returns the registered source by id.
    #[test]
    fn composer_find_source() {
        let mut composer = DataComposer::default();
        composer.register(make_source(42));
        assert!(composer.find_source(42).is_some());
        assert!(composer.find_source(99).is_none());
    }

    // Test 9: compose returns effective_limit as row_count stub.
    #[test]
    fn composer_compose_row_count() {
        let mut composer = DataComposer::default();
        composer.register(make_source(1));
        let q = DataQuery { source_id: 1, fields: vec![], limit: Some(25) };
        let result = composer.compose(&q);
        assert_eq!(result.row_count, 25);
    }
}
