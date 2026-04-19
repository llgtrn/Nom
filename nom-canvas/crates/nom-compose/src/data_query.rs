#![deny(unsafe_code)]

use polars::prelude::*;

/// Source of data for a lazy query frame.
#[derive(Debug, Clone)]
pub enum DataSource {
    Sqlite { path: String, table: String },
    Csv { path: String },
    Json { path: String },
    Parquet { path: String },
    Memory { df: DataFrame },
}

/// A lazy query frame that accumulates operations until collected.
#[derive(Clone)]
pub struct NomQueryFrame {
    inner: LazyFrame,
    source: DataSource,
    pending_group_by: Option<Vec<String>>,
}

impl NomQueryFrame {
    /// Load from an on-disk SQLite table.
    pub fn from_sqlite(path: &str, table: &str) -> Result<Self, String> {
        if !is_safe_identifier(table) {
            return Err(format!("unsafe table name: {table}"));
        }

        use rusqlite::{types::Value as SqlValue, Connection};

        let conn = Connection::open(path).map_err(|e| format!("sqlite open: {e}"))?;
        let sql = format!("SELECT * FROM {table}");
        let mut stmt = conn.prepare(&sql).map_err(|e| format!("sqlite prepare: {e}"))?;

        let col_names: Vec<String> = stmt.column_names().iter().map(|s| s.to_string()).collect();
        let ncols = col_names.len();

        let mut col_strs: Vec<Vec<String>> = vec![Vec::new(); ncols];
        let mut rows = stmt.query([]).map_err(|e| format!("sqlite query: {e}"))?;

        while let Some(row) = rows.next().map_err(|e| format!("sqlite row: {e}"))? {
            for i in 0..ncols {
                let val: SqlValue = row.get(i).unwrap_or(SqlValue::Null);
                col_strs[i].push(match val {
                    SqlValue::Integer(v) => v.to_string(),
                    SqlValue::Real(v) => v.to_string(),
                    SqlValue::Text(v) => v,
                    SqlValue::Blob(v) => String::from_utf8_lossy(&v).to_string(),
                    SqlValue::Null => String::new(),
                });
            }
        }

        let series: Vec<Series> = col_names
            .iter()
            .enumerate()
            .map(|(i, name)| Series::new(name.as_str().into(), col_strs[i].clone()))
            .collect();

        let df = DataFrame::new(series).map_err(|e| e.to_string())?;
        Ok(Self {
            inner: df.lazy(),
            source: DataSource::Sqlite {
                path: path.to_string(),
                table: table.to_string(),
            },
            pending_group_by: None,
        })
    }

    /// Load from a CSV file.
    pub fn from_csv(path: &str) -> Result<Self, String> {
        let lf = LazyCsvReader::new(path)
            .finish()
            .map_err(|e| e.to_string())?;
        Ok(Self {
            inner: lf,
            source: DataSource::Csv {
                path: path.to_string(),
            },
            pending_group_by: None,
        })
    }

    /// Load from a JSON-lines file.
    pub fn from_json(path: &str) -> Result<Self, String> {
        let file = std::fs::File::open(path).map_err(|e| e.to_string())?;
        let df = JsonLineReader::new(file)
            .finish()
            .map_err(|e| e.to_string())?;
        Ok(Self {
            inner: df.lazy(),
            source: DataSource::Json {
                path: path.to_string(),
            },
            pending_group_by: None,
        })
    }

    /// Load from a Parquet file.
    pub fn from_parquet(path: &str) -> Result<Self, String> {
        let lf = LazyFrame::scan_parquet(path, ScanArgsParquet::default())
            .map_err(|e| e.to_string())?;
        Ok(Self {
            inner: lf,
            source: DataSource::Parquet {
                path: path.to_string(),
            },
            pending_group_by: None,
        })
    }

    /// Create from an in-memory frame.
    pub fn from_memory(df: DataFrame) -> Self {
        Self {
            inner: df.lazy(),
            source: DataSource::Memory {
                df: DataFrame::default(),
            },
            pending_group_by: None,
        }
    }

    /// Apply a predicate filter expressed as a SQL-like expression.
    pub fn filter(&mut self, predicate: &str) -> Result<&mut Self, String> {
        use polars::sql::SQLContext;
        let mut ctx = SQLContext::new();
        ctx.register("__tmp", self.inner.clone());
        let sql = format!("SELECT * FROM __tmp WHERE {}", predicate);
        let lf = ctx.execute(&sql).map_err(|e| format!("filter: {e}"))?;
        self.inner = lf;
        Ok(self)
    }

    /// Select a subset of columns.
    pub fn select(&mut self, columns: &[&str]) -> &mut Self {
        let exprs: Vec<Expr> = columns.iter().map(|c| col(*c)).collect();
        self.inner = self.inner.clone().select(exprs);
        self
    }

    /// Declare grouping columns (used by the next aggregate call).
    pub fn group_by(&mut self, columns: &[&str]) -> &mut Self {
        self.pending_group_by = Some(columns.iter().map(|c| c.to_string()).collect());
        self
    }

    /// Apply aggregations. If group_by was called, aggregates within groups.
    pub fn aggregate(&mut self, aggs: &[(&str, &str)]) -> Result<&mut Self, String> {
        let exprs: Vec<Expr> = aggs
            .iter()
            .map(|(column, op)| match *op {
                "sum" => Ok(sum(column)),
                "count" => Ok(col(column).count()),
                "mean" => Ok(mean(column)),
                "min" => Ok(min(column)),
                "max" => Ok(max(column)),
                "first" => Ok(col(column).first()),
                "last" => Ok(col(column).last()),
                _ => Err(format!("unsupported aggregation: {op}")),
            })
            .collect::<Result<Vec<_>, _>>()?;

        if let Some(ref group_cols) = self.pending_group_by {
            let group_exprs: Vec<Expr> = group_cols.iter().map(|c| col(c)).collect();
            self.inner = self.inner.clone().group_by(group_exprs).agg(exprs);
            self.pending_group_by = None;
        } else {
            self.inner = self.inner.clone().select(exprs);
        }
        Ok(self)
    }

    /// Sort by a column.
    pub fn sort(&mut self, column: &str, descending: bool) -> &mut Self {
        self.inner = self
            .inner
            .clone()
            .sort([column], SortMultipleOptions::new().with_order_descending(descending));
        self
    }

    /// Limit the number of rows.
    pub fn limit(&mut self, n: usize) -> &mut Self {
        self.inner = self.inner.clone().limit(n as u32);
        self
    }

    /// Join with another frame.
    pub fn join(
        &mut self,
        other: NomQueryFrame,
        on: &[&str],
        how: JoinType,
    ) -> Result<&mut Self, String> {
        let left_on: Vec<Expr> = on.iter().map(|c| col(*c)).collect();
        let right_on: Vec<Expr> = on.iter().map(|c| col(*c)).collect();
        self.inner = self
            .inner
            .clone()
            .join(other.inner, left_on, right_on, JoinArgs::new(how));
        Ok(self)
    }

    /// Execute the plan and return a materialized frame.
    pub fn collect(&self) -> Result<DataFrame, String> {
        self.inner.clone().collect().map_err(|e| e.to_string())
    }

    /// Return the optimized plan as a string.
    pub fn explain(&self) -> Result<String, String> {
        self.inner
            .clone()
            .describe_optimized_plan()
            .map_err(|e| e.to_string())
    }
}

/// Returns true if the identifier contains only safe characters.
fn is_safe_identifier(s: &str) -> bool {
    if s.is_empty() {
        return false;
    }
    s.chars()
        .all(|c| c.is_alphanumeric() || c == '_' || c == '.')
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_df() -> DataFrame {
        df! {
            "name" => ["a", "b", "c", "d"],
            "value" => [1i64, 2, 3, 4],
            "category" => ["x", "y", "x", "y"],
        }
        .unwrap()
    }

    #[test]
    fn from_memory_creates_frame() {
        let df = sample_df();
        let qf = NomQueryFrame::from_memory(df);
        assert!(matches!(qf.source, DataSource::Memory { .. }));
    }

    #[test]
    fn select_reduces_columns() {
        let df = sample_df();
        let mut qf = NomQueryFrame::from_memory(df);
        qf.select(&["name", "value"]);
        let out = qf.collect().unwrap();
        assert_eq!(out.width(), 2);
        let names: Vec<&str> = out.get_column_names().iter().map(|s| *s).collect();
        assert!(names.contains(&"name"));
        assert!(names.contains(&"value"));
        assert!(!names.contains(&"category"));
    }

    #[test]
    fn filter_predicate_works() {
        let df = sample_df();
        let mut qf = NomQueryFrame::from_memory(df);
        qf.filter("value > 2").unwrap();
        let out = qf.collect().unwrap();
        assert_eq!(out.height(), 2);
    }

    #[test]
    fn sort_descending() {
        let df = sample_df();
        let mut qf = NomQueryFrame::from_memory(df);
        qf.sort("value", true);
        let out = qf.collect().unwrap();
        let values: Vec<i64> = out
            .column("value")
            .unwrap()
            .i64()
            .unwrap()
            .into_no_null_iter()
            .collect();
        assert_eq!(values, vec![4, 3, 2, 1]);
    }

    #[test]
    fn limit_reduces_rows() {
        let df = sample_df();
        let mut qf = NomQueryFrame::from_memory(df);
        qf.limit(2);
        let out = qf.collect().unwrap();
        assert_eq!(out.height(), 2);
    }

    #[test]
    fn group_by_aggregate_sum() {
        let df = sample_df();
        let mut qf = NomQueryFrame::from_memory(df);
        qf.group_by(&["category"])
            .aggregate(&[("value", "sum")])
            .unwrap();
        let out = qf.collect().unwrap();
        assert_eq!(out.height(), 2);
    }

    #[test]
    fn explain_returns_plan() {
        let df = sample_df();
        let qf = NomQueryFrame::from_memory(df);
        let plan = qf.explain().unwrap();
        assert!(!plan.is_empty());
    }

    #[test]
    fn join_two_frames() {
        let left = df! {
            "key" => ["a", "b"],
            "lval" => [1i64, 2],
        }
        .unwrap();
        let right = df! {
            "key" => ["a", "b"],
            "rval" => [10i64, 20],
        }
        .unwrap();
        let mut qf = NomQueryFrame::from_memory(left);
        qf.join(NomQueryFrame::from_memory(right), &["key"], JoinType::Inner)
            .unwrap();
        let out = qf.collect().unwrap();
        assert_eq!(out.height(), 2);
        assert!(out.get_column_names().contains(&"rval"));
    }

    #[test]
    fn aggregate_without_group_by() {
        let df = sample_df();
        let mut qf = NomQueryFrame::from_memory(df);
        qf.aggregate(&[("value", "sum")]).unwrap();
        let out = qf.collect().unwrap();
        assert_eq!(out.height(), 1);
    }

    #[test]
    fn from_sqlite_reads_table() {
        use rusqlite::Connection;
        let dir = std::env::temp_dir();
        let path = dir.join("nom_test_sqlite.db");
        let _ = std::fs::remove_file(&path);
        {
            let conn = Connection::open(&path).unwrap();
            conn.execute(
                "CREATE TABLE t (id INTEGER PRIMARY KEY, name TEXT)",
                [],
            )
            .unwrap();
            conn.execute("INSERT INTO t (name) VALUES ('alice')", [])
                .unwrap();
            conn.execute("INSERT INTO t (name) VALUES ('bob')", [])
                .unwrap();
        }
        let qf = NomQueryFrame::from_sqlite(path.to_str().unwrap(), "t").unwrap();
        let out = qf.collect().unwrap();
        assert_eq!(out.height(), 2);
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn from_csv_reads_file() {
        let dir = std::env::temp_dir();
        let path = dir.join("nom_test.csv");
        std::fs::write(&path, "a,b\n1,2\n3,4\n").unwrap();
        let qf = NomQueryFrame::from_csv(path.to_str().unwrap()).unwrap();
        let out = qf.collect().unwrap();
        assert_eq!(out.height(), 2);
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn from_json_reads_file() {
        let dir = std::env::temp_dir();
        let path = dir.join("nom_test.json");
        std::fs::write(&path, "{\"a\":1,\"b\":2}\n{\"a\":3,\"b\":4}\n").unwrap();
        let qf = NomQueryFrame::from_json(path.to_str().unwrap()).unwrap();
        let out = qf.collect().unwrap();
        assert_eq!(out.height(), 2);
        let _ = std::fs::remove_file(&path);
    }
}
