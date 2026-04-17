#![deny(unsafe_code)]
use crate::block_schema::{BlockSchema, Role};
use crate::flavour::{NOTE, SURFACE};
use crate::block_model::FractionalIndex;
use crate::media::BlobId;

#[derive(Clone, Debug, PartialEq)]
pub struct DataBlockProps {
    pub rows: usize,
    pub columns: usize,
    pub column_types: Vec<String>,
    pub sort_column: Option<String>,
    pub filter_expr: Option<String>,
    pub source_id: BlobId,
    pub index: FractionalIndex,
}

impl DataBlockProps {
    pub fn new(source_id: BlobId, rows: usize, columns: usize) -> Self {
        Self {
            source_id,
            rows,
            columns,
            column_types: Vec::new(),
            sort_column: None,
            filter_expr: None,
            index: "a0".to_owned(),
        }
    }

    pub fn with_sort_column(mut self, col: impl Into<String>) -> Self {
        self.sort_column = Some(col.into());
        self
    }

    pub fn with_filter(mut self, expr: impl Into<String>) -> Self {
        self.filter_expr = Some(expr.into());
        self
    }
}

pub fn data_block_schema() -> BlockSchema {
    BlockSchema {
        flavour: crate::compose::COMPOSE_DATA,
        version: 1,
        role: Role::Content,
        parents: &[NOTE, SURFACE],
        children: &[],
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::block_schema::Role;

    #[test]
    fn empty_rows_and_cols() {
        let d = DataBlockProps::new("blob-data".to_owned(), 0, 0);
        assert_eq!(d.rows, 0);
        assert_eq!(d.columns, 0);
        assert!(d.column_types.is_empty());
        assert!(d.sort_column.is_none());
        assert!(d.filter_expr.is_none());
    }

    #[test]
    fn with_sort_column_sets_sort() {
        let d = DataBlockProps::new("b".to_owned(), 10, 3).with_sort_column("date");
        assert_eq!(d.sort_column.as_deref(), Some("date"));
    }

    #[test]
    fn with_filter_sets_expr() {
        let d = DataBlockProps::new("b".to_owned(), 5, 2).with_filter("age > 18");
        assert_eq!(d.filter_expr.as_deref(), Some("age > 18"));
    }

    #[test]
    fn schema_role_content() {
        assert_eq!(data_block_schema().role, Role::Content);
    }
}
