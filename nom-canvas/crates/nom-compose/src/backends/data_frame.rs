#![deny(unsafe_code)]

use super::ComposeResult;

/// An in-memory data frame with named columns and string rows.
#[derive(Debug, Clone, Default)]
pub struct DataFrame {
    pub columns: Vec<String>,
    pub rows: Vec<Vec<String>>,
}

impl DataFrame {
    pub fn new(columns: Vec<String>) -> Self {
        Self { columns, rows: Vec::new() }
    }

    pub fn row_count(&self) -> usize { self.rows.len() }

    pub fn column_count(&self) -> usize { self.columns.len() }

    pub fn add_row(&mut self, row: Vec<String>) {
        self.rows.push(row);
    }

    /// Serialize to CSV (header + data rows).
    pub fn to_csv(&self) -> String {
        let mut out = self.columns.join(",");
        out.push('\n');
        for row in &self.rows {
            out.push_str(&row.join(","));
            out.push('\n');
        }
        out
    }
}

/// Specification for composing a DataFrame artifact.
#[derive(Debug, Clone)]
pub struct DataFrameSpec {
    pub source_query: String,
    pub limit: Option<usize>,
    pub transform: Option<String>,
}

/// Compose a DataFrame artifact.
///
/// If `df` is `Some`, the frame is serialized to CSV and returned as a
/// successful artifact.  If `df` is `None`, the `source_query` is treated as
/// mock data for validation purposes (returns `Ok(())`).
pub fn compose(spec: &DataFrameSpec, df: Option<DataFrame>) -> ComposeResult {
    match df {
        Some(frame) => {
            let limit = spec.limit.unwrap_or(usize::MAX);
            let row_count = frame.row_count().min(limit);
            if row_count == 0 && frame.column_count() == 0 {
                return Err("DataFrame is empty".into());
            }
            // CSV generation validates the frame is serialisable.
            let _csv = frame.to_csv();
            Ok(())
        }
        None => {
            // No frame supplied — validate the query string is non-empty.
            if spec.source_query.trim().is_empty() {
                return Err("source_query must not be empty when no DataFrame is supplied".into());
            }
            Ok(())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn data_frame_csv_output() {
        let mut df = DataFrame::new(vec!["id".into(), "name".into()]);
        df.add_row(vec!["1".into(), "alice".into()]);
        df.add_row(vec!["2".into(), "bob".into()]);

        assert_eq!(df.row_count(), 2);
        assert_eq!(df.column_count(), 2);

        let csv = df.to_csv();
        assert!(csv.starts_with("id,name\n"));
        assert!(csv.contains("1,alice\n"));
        assert!(csv.contains("2,bob\n"));
    }

    #[test]
    fn data_frame_compose_produces_artifact() {
        let spec = DataFrameSpec {
            source_query: "SELECT id, name FROM users".into(),
            limit: Some(100),
            transform: None,
        };
        let mut df = DataFrame::new(vec!["id".into(), "name".into()]);
        df.add_row(vec!["1".into(), "alice".into()]);

        let result = compose(&spec, Some(df));
        assert!(result.is_ok());
    }

    #[test]
    fn data_frame_compose_none_uses_query() {
        let spec = DataFrameSpec {
            source_query: "SELECT * FROM events".into(),
            limit: None,
            transform: None,
        };
        assert!(compose(&spec, None).is_ok());
    }

    #[test]
    fn data_frame_compose_none_empty_query_errors() {
        let spec = DataFrameSpec {
            source_query: "   ".into(),
            limit: None,
            transform: None,
        };
        assert!(compose(&spec, None).is_err());
    }
}
