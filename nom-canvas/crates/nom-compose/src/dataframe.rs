#![deny(unsafe_code)]

/// A single value in a typed column.
#[derive(Debug, Clone, PartialEq)]
pub enum ColumnValue {
    Float(f64),
    Int(i64),
    Str(String),
    Null,
}

/// A named column holding typed values.
#[derive(Debug, Clone)]
pub struct Column {
    pub name: String,
    pub values: Vec<ColumnValue>,
}

impl Column {
    /// Create a new column with the given name and no values.
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            values: Vec::new(),
        }
    }

    /// Sum all `Float` values in this column; non-float values are skipped.
    pub fn float_sum(&self) -> f64 {
        self.values.iter().fold(0.0, |acc, v| {
            if let ColumnValue::Float(f) = v {
                acc + f
            } else {
                acc
            }
        })
    }

    /// Mean of all `Float` values; returns 0.0 when there are no float values.
    pub fn float_mean(&self) -> f64 {
        let floats: Vec<f64> = self
            .values
            .iter()
            .filter_map(|v| {
                if let ColumnValue::Float(f) = v {
                    Some(*f)
                } else {
                    None
                }
            })
            .collect();
        if floats.is_empty() {
            0.0
        } else {
            floats.iter().sum::<f64>() / floats.len() as f64
        }
    }
}

/// An in-memory data frame composed of typed columns.
#[derive(Debug, Clone, Default)]
pub struct DataFrame {
    pub columns: Vec<Column>,
}

impl DataFrame {
    /// Create an empty `DataFrame`.
    pub fn new() -> Self {
        Self::default()
    }

    /// Append a column, returning `self` for chaining.
    pub fn with_column(mut self, col: Column) -> Self {
        self.columns.push(col);
        self
    }

    /// Number of rows — defined as the minimum column length.
    /// Returns 0 when there are no columns.
    pub fn row_count(&self) -> usize {
        self.columns
            .iter()
            .map(|c| c.values.len())
            .min()
            .unwrap_or(0)
    }

    /// Number of columns.
    pub fn col_count(&self) -> usize {
        self.columns.len()
    }

    /// Look up a column by name.
    pub fn get_column(&self, name: &str) -> Option<&Column> {
        self.columns.iter().find(|c| c.name == name)
    }

    /// Return a new `DataFrame` containing only rows where the named float
    /// column exceeds `threshold`.  Rows where the value is not `Float` are
    /// dropped.  Panics if the column does not exist.
    pub fn filter_float_gt(&self, col_name: &str, threshold: f64) -> Self {
        let row_count = self.row_count();
        let filter_col_idx = self
            .columns
            .iter()
            .position(|c| c.name == col_name)
            .expect("filter column not found");

        // Collect which row indices pass the filter.
        let passing: Vec<usize> = (0..row_count)
            .filter(|&i| {
                matches!(self.columns[filter_col_idx].values.get(i),
                    Some(ColumnValue::Float(f)) if *f > threshold)
            })
            .collect();

        let new_columns: Vec<Column> = self
            .columns
            .iter()
            .map(|col| Column {
                name: col.name.clone(),
                values: passing
                    .iter()
                    .filter_map(|&i| col.values.get(i).cloned())
                    .collect(),
            })
            .collect();

        DataFrame {
            columns: new_columns,
        }
    }

    /// Project to a subset of columns by name, preserving order of `col_names`.
    /// Columns not found in the frame are silently skipped.
    pub fn select(&self, col_names: &[&str]) -> Self {
        let new_columns: Vec<Column> = col_names
            .iter()
            .filter_map(|&name| self.get_column(name).cloned())
            .collect();
        DataFrame {
            columns: new_columns,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dataframe_new_is_empty() {
        let df = DataFrame::new();
        assert_eq!(df.row_count(), 0);
        assert_eq!(df.col_count(), 0);
    }

    #[test]
    fn dataframe_with_column_builds_frame() {
        let col = Column {
            name: "score".to_string(),
            values: vec![
                ColumnValue::Float(1.0),
                ColumnValue::Float(2.0),
                ColumnValue::Float(3.0),
            ],
        };
        let df = DataFrame::new().with_column(col);
        assert_eq!(df.col_count(), 1);
        assert_eq!(df.row_count(), 3);
        assert!(df.get_column("score").is_some());
        assert!(df.get_column("missing").is_none());
    }

    #[test]
    fn dataframe_row_count_uses_min_column_length() {
        let short = Column {
            name: "a".to_string(),
            values: vec![ColumnValue::Int(1), ColumnValue::Int(2)],
        };
        let long = Column {
            name: "b".to_string(),
            values: vec![
                ColumnValue::Int(10),
                ColumnValue::Int(20),
                ColumnValue::Int(30),
            ],
        };
        let df = DataFrame::new().with_column(short).with_column(long);
        assert_eq!(df.row_count(), 2, "row_count must be the minimum column length");
    }

    #[test]
    fn dataframe_filter_float_gt_keeps_passing_rows() {
        let scores = Column {
            name: "score".to_string(),
            values: vec![
                ColumnValue::Float(1.5),
                ColumnValue::Float(3.0),
                ColumnValue::Float(0.5),
                ColumnValue::Float(4.2),
            ],
        };
        let labels = Column {
            name: "label".to_string(),
            values: vec![
                ColumnValue::Str("low".to_string()),
                ColumnValue::Str("mid".to_string()),
                ColumnValue::Str("very-low".to_string()),
                ColumnValue::Str("high".to_string()),
            ],
        };
        let df = DataFrame::new().with_column(scores).with_column(labels);
        let filtered = df.filter_float_gt("score", 2.0);
        assert_eq!(filtered.row_count(), 2, "only rows with score > 2.0 must remain");
        let score_col = filtered.get_column("score").unwrap();
        assert!(
            score_col.values.iter().all(|v| matches!(v, ColumnValue::Float(f) if *f > 2.0)),
            "all remaining scores must exceed the threshold"
        );
    }

    #[test]
    fn dataframe_select_projects_columns() {
        let a = Column {
            name: "a".to_string(),
            values: vec![ColumnValue::Int(1)],
        };
        let b = Column {
            name: "b".to_string(),
            values: vec![ColumnValue::Int(2)],
        };
        let c = Column {
            name: "c".to_string(),
            values: vec![ColumnValue::Int(3)],
        };
        let df = DataFrame::new()
            .with_column(a)
            .with_column(b)
            .with_column(c);
        let projected = df.select(&["a", "c"]);
        assert_eq!(projected.col_count(), 2);
        assert!(projected.get_column("a").is_some());
        assert!(projected.get_column("c").is_some());
        assert!(projected.get_column("b").is_none());
    }

    #[test]
    fn column_float_sum_and_mean() {
        let col = Column {
            name: "values".to_string(),
            values: vec![
                ColumnValue::Float(2.0),
                ColumnValue::Float(4.0),
                ColumnValue::Float(6.0),
                ColumnValue::Null,      // skipped
                ColumnValue::Int(99),   // skipped
            ],
        };
        assert!((col.float_sum() - 12.0).abs() < f64::EPSILON, "sum must be 12.0");
        assert!((col.float_mean() - 4.0).abs() < f64::EPSILON, "mean must be 4.0");

        let empty = Column {
            name: "empty".to_string(),
            values: vec![ColumnValue::Null],
        };
        assert_eq!(empty.float_mean(), 0.0, "mean of no floats must be 0.0");
    }
}
