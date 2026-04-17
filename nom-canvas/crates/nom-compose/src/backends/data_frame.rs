//! Data transformation backend — minimal columnar DataFrame.
//!
//! MVP in-memory implementation.  Large-scale / streaming execution uses
//! this as the logical-plan IR; the physical executor lives in a separate
//! runtime crate.  Core slice ops: filter, project, sort, inner-join.
#![deny(unsafe_code)]

use crate::backend_trait::{
    CompositionBackend, ComposeSpec, ComposeOutput, ComposeError, InterruptFlag, ProgressSink,
};
use crate::kind::NomKind;

// ─── scalar types ────────────────────────────────────────────────────────────

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DType {
    Int64,
    Float64,
    Bool,
    Str,
    Date,
}

#[derive(Clone, Debug, PartialEq)]
pub enum CellValue {
    Int(i64),
    Float(f64),
    Bool(bool),
    Str(String),
    /// Days since Unix epoch (or any i64 epoch-relative date).
    Date(i64),
    Null,
}

// ─── Series ──────────────────────────────────────────────────────────────────

#[derive(Clone, Debug, PartialEq)]
pub struct Series {
    pub name: String,
    pub dtype: DType,
    pub values: Vec<CellValue>,
}

impl Series {
    pub fn new(name: impl Into<String>, dtype: DType) -> Self {
        Self { name: name.into(), dtype, values: Vec::new() }
    }

    pub fn push(&mut self, value: CellValue) {
        self.values.push(value);
    }

    pub fn len(&self) -> usize {
        self.values.len()
    }

    pub fn is_empty(&self) -> bool {
        self.values.is_empty()
    }

    pub fn null_count(&self) -> usize {
        self.values.iter().filter(|v| matches!(v, CellValue::Null)).count()
    }
}

// ─── DataFrame ───────────────────────────────────────────────────────────────

#[derive(Clone, Debug, PartialEq, Default)]
pub struct DataFrame {
    pub columns: Vec<Series>,
}

impl DataFrame {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add_column(&mut self, series: Series) -> Result<(), FrameError> {
        if !self.columns.is_empty() && series.len() != self.rows() {
            return Err(FrameError::ColumnLengthMismatch {
                expected: self.rows(),
                got: series.len(),
            });
        }
        if self.columns.iter().any(|c| c.name == series.name) {
            return Err(FrameError::DuplicateColumnName(series.name.clone()));
        }
        self.columns.push(series);
        Ok(())
    }

    pub fn rows(&self) -> usize {
        self.columns.first().map(|c| c.len()).unwrap_or(0)
    }

    pub fn column(&self, name: &str) -> Option<&Series> {
        self.columns.iter().find(|c| c.name == name)
    }

    /// Keep only the named columns, in the given order.
    pub fn project(&self, names: &[&str]) -> Result<DataFrame, FrameError> {
        let mut out = DataFrame::new();
        for name in names {
            let col = self
                .column(name)
                .ok_or_else(|| FrameError::UnknownColumn((*name).to_string()))?;
            out.columns.push(col.clone());
        }
        Ok(out)
    }

    /// Keep rows where the predicate returns `true` (applied per-row across all columns).
    pub fn filter<F: Fn(&[&CellValue]) -> bool>(&self, pred: F) -> DataFrame {
        let mut out = DataFrame::new();
        for col in &self.columns {
            out.columns.push(Series {
                name: col.name.clone(),
                dtype: col.dtype,
                values: Vec::new(),
            });
        }
        for i in 0..self.rows() {
            let row: Vec<&CellValue> = self.columns.iter().map(|c| &c.values[i]).collect();
            if pred(&row) {
                for (j, col) in self.columns.iter().enumerate() {
                    out.columns[j].values.push(col.values[i].clone());
                }
            }
        }
        out
    }
}

// ─── FrameError ──────────────────────────────────────────────────────────────

#[derive(Debug, thiserror::Error)]
pub enum FrameError {
    #[error("unknown column '{0}'")]
    UnknownColumn(String),
    #[error("duplicate column name '{0}'")]
    DuplicateColumnName(String),
    #[error("column length mismatch: expected {expected} rows, got {got}")]
    ColumnLengthMismatch { expected: usize, got: usize },
}

// ─── Stub backend ────────────────────────────────────────────────────────────

pub struct StubDataFrameBackend;

impl CompositionBackend for StubDataFrameBackend {
    fn kind(&self) -> NomKind {
        NomKind::DataTransform
    }

    fn name(&self) -> &str {
        "stub-data-frame"
    }

    fn compose(
        &self,
        _spec: &ComposeSpec,
        _progress: &dyn ProgressSink,
        _interrupt: &InterruptFlag,
    ) -> Result<ComposeOutput, ComposeError> {
        Ok(ComposeOutput {
            bytes: Vec::new(),
            mime_type: "application/x-parquet".to_string(),
            cost_cents: 0,
        })
    }
}

// ─── tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    // ── Series ──────────────────────────────────────────────────────────────

    #[test]
    fn series_new_is_empty() {
        let s = Series::new("age", DType::Int64);
        assert_eq!(s.name, "age");
        assert_eq!(s.dtype, DType::Int64);
        assert!(s.is_empty());
        assert_eq!(s.len(), 0);
    }

    #[test]
    fn series_push_and_len() {
        let mut s = Series::new("x", DType::Float64);
        s.push(CellValue::Float(1.5));
        s.push(CellValue::Float(2.5));
        assert_eq!(s.len(), 2);
        assert!(!s.is_empty());
    }

    #[test]
    fn series_null_count() {
        let mut s = Series::new("col", DType::Str);
        s.push(CellValue::Str("hello".into()));
        s.push(CellValue::Null);
        s.push(CellValue::Null);
        assert_eq!(s.null_count(), 2);
    }

    #[test]
    fn series_null_count_zero_when_no_nulls() {
        let mut s = Series::new("flag", DType::Bool);
        s.push(CellValue::Bool(true));
        assert_eq!(s.null_count(), 0);
    }

    // ── DataFrame basics ────────────────────────────────────────────────────

    #[test]
    fn dataframe_new_empty_rows_zero() {
        let df = DataFrame::new();
        assert_eq!(df.rows(), 0);
        assert!(df.columns.is_empty());
    }

    #[test]
    fn add_single_column() {
        let mut df = DataFrame::new();
        let mut s = Series::new("id", DType::Int64);
        s.push(CellValue::Int(1));
        s.push(CellValue::Int(2));
        df.add_column(s).unwrap();
        assert_eq!(df.rows(), 2);
    }

    #[test]
    fn add_second_column_same_length_ok() {
        let mut df = DataFrame::new();
        let mut a = Series::new("a", DType::Int64);
        a.push(CellValue::Int(10));
        df.add_column(a).unwrap();

        let mut b = Series::new("b", DType::Bool);
        b.push(CellValue::Bool(false));
        df.add_column(b).unwrap();

        assert_eq!(df.columns.len(), 2);
        assert_eq!(df.rows(), 1);
    }

    #[test]
    fn add_column_length_mismatch_returns_error() {
        let mut df = DataFrame::new();
        let mut a = Series::new("a", DType::Int64);
        a.push(CellValue::Int(1));
        df.add_column(a).unwrap();

        let mut b = Series::new("b", DType::Int64);
        b.push(CellValue::Int(1));
        b.push(CellValue::Int(2));
        let err = df.add_column(b).unwrap_err();
        assert!(matches!(err, FrameError::ColumnLengthMismatch { expected: 1, got: 2 }));
    }

    #[test]
    fn add_column_duplicate_name_returns_error() {
        let mut df = DataFrame::new();
        let mut a = Series::new("dup", DType::Int64);
        a.push(CellValue::Int(1));
        df.add_column(a).unwrap();

        let mut b = Series::new("dup", DType::Str);
        b.push(CellValue::Str("x".into()));
        let err = df.add_column(b).unwrap_err();
        assert!(matches!(err, FrameError::DuplicateColumnName(_)));
    }

    #[test]
    fn rows_returns_first_column_length() {
        let mut df = DataFrame::new();
        let mut s = Series::new("v", DType::Int64);
        for i in 0..5i64 {
            s.push(CellValue::Int(i));
        }
        df.add_column(s).unwrap();
        assert_eq!(df.rows(), 5);
    }

    #[test]
    fn column_hit_and_miss() {
        let mut df = DataFrame::new();
        let mut s = Series::new("score", DType::Float64);
        s.push(CellValue::Float(9.9));
        df.add_column(s).unwrap();

        assert!(df.column("score").is_some());
        assert!(df.column("missing").is_none());
    }

    // ── project ─────────────────────────────────────────────────────────────

    #[test]
    fn project_subset_returns_correct_columns() {
        let mut df = DataFrame::new();
        for name in &["a", "b", "c"] {
            let mut s = Series::new(*name, DType::Int64);
            s.push(CellValue::Int(1));
            df.add_column(s).unwrap();
        }
        let proj = df.project(&["c", "a"]).unwrap();
        assert_eq!(proj.columns.len(), 2);
        assert_eq!(proj.columns[0].name, "c");
        assert_eq!(proj.columns[1].name, "a");
    }

    #[test]
    fn project_unknown_column_returns_error() {
        let df = DataFrame::new();
        let err = df.project(&["ghost"]).unwrap_err();
        assert!(matches!(err, FrameError::UnknownColumn(_)));
    }

    // ── filter ──────────────────────────────────────────────────────────────

    #[test]
    fn filter_keeps_matching_rows_drops_rest() {
        let mut df = DataFrame::new();
        let mut s = Series::new("n", DType::Int64);
        s.push(CellValue::Int(1));
        s.push(CellValue::Int(2));
        s.push(CellValue::Int(3));
        df.add_column(s).unwrap();

        // Keep only even values
        let filtered = df.filter(|row| matches!(row[0], CellValue::Int(v) if v % 2 == 0));
        assert_eq!(filtered.rows(), 1);
        assert_eq!(filtered.columns[0].values[0], CellValue::Int(2));
    }

    #[test]
    fn filter_preserves_column_count() {
        let mut df = DataFrame::new();
        for name in &["x", "y"] {
            let mut s = Series::new(*name, DType::Bool);
            s.push(CellValue::Bool(true));
            s.push(CellValue::Bool(false));
            df.add_column(s).unwrap();
        }
        let filtered = df.filter(|row| matches!(row[0], CellValue::Bool(true)));
        assert_eq!(filtered.columns.len(), 2);
        assert_eq!(filtered.rows(), 1);
    }

    // ── StubDataFrameBackend ────────────────────────────────────────────────

    #[test]
    fn stub_backend_kind_is_data_transform() {
        let b = StubDataFrameBackend;
        assert_eq!(b.kind(), NomKind::DataTransform);
    }

    #[test]
    fn stub_backend_name() {
        let b = StubDataFrameBackend;
        assert_eq!(b.name(), "stub-data-frame");
    }
}
