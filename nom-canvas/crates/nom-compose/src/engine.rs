#![deny(unsafe_code)]

#[cfg(feature = "polars")]
use polars::prelude::*;
#[cfg(feature = "polars")]
use polars::sql::SQLContext;
use serde_json::Value;

/// Swappable query engine trait.
pub trait QueryEngine: Send + Sync {
    /// Prepare a query plan from a SQL string.
    fn plan(&self, sql: &str) -> Result<Box<dyn QueryPlan>, String>;
    /// Execute a prepared plan and return output.
    fn execute(&self, plan: &dyn QueryPlan) -> Result<QueryOutput, String>;
}

/// Abstract query plan produced by an engine.
pub trait QueryPlan: Send + Sync {
    /// Human-readable description of the plan.
    fn explain(&self) -> String;
    /// Downcast helper for engine-specific plan types.
    fn as_any(&self) -> &dyn std::any::Any;
}

/// Result variants from executing a query plan.
pub enum QueryOutput {
    #[cfg(feature = "polars")]
    DataFrame(DataFrame),
    Rows(Vec<Vec<Value>>),
    Scalar(Value),
}

/// Lazy-frame-backed engine.
#[cfg(feature = "polars")]
pub struct PolarsEngine {
    tables: std::sync::Mutex<std::collections::HashMap<String, LazyFrame>>,
}

#[cfg(feature = "polars")]
impl PolarsEngine {
    /// Create a new empty engine.
    pub fn new() -> Self {
        Self {
            tables: std::sync::Mutex::new(std::collections::HashMap::new()),
        }
    }

    /// Register a CSV file as a named table.
    pub fn register_csv(&self, name: &str, path: &str) -> Result<(), String> {
        let lf = LazyCsvReader::new(path)
            .finish()
            .map_err(|e| e.to_string())?;
        self.tables.lock().unwrap().insert(name.to_string(), lf);
        Ok(())
    }

    /// Register a JSON-lines file as a named table.
    pub fn register_json(&self, name: &str, path: &str) -> Result<(), String> {
        let file = std::fs::File::open(path).map_err(|e| e.to_string())?;
        let df = JsonLineReader::new(file)
            .finish()
            .map_err(|e| e.to_string())?;
        self.tables.lock().unwrap().insert(name.to_string(), df.lazy());
        Ok(())
    }

    /// Register a Parquet file as a named table.
    pub fn register_parquet(&self, name: &str, path: &str) -> Result<(), String> {
        let lf = LazyFrame::scan_parquet(path, ScanArgsParquet::default())
            .map_err(|e| e.to_string())?;
        self.tables.lock().unwrap().insert(name.to_string(), lf);
        Ok(())
    }
}

#[cfg(feature = "polars")]
impl QueryEngine for PolarsEngine {
    fn plan(&self, sql: &str) -> Result<Box<dyn QueryPlan>, String> {
        let mut ctx = SQLContext::new();
        let tables = self.tables.lock().unwrap();
        for (name, lf) in tables.iter() {
            ctx.register(name, lf.clone());
        }
        drop(tables);
        let lf = ctx.execute(sql).map_err(|e| e.to_string())?;
        Ok(Box::new(PolarsPlan {
            lf,
            sql: sql.to_string(),
        }))
    }

    fn execute(&self, plan: &dyn QueryPlan) -> Result<QueryOutput, String> {
        let plan = plan
            .as_any()
            .downcast_ref::<PolarsPlan>()
            .ok_or_else(|| "invalid plan type for PolarsEngine".to_string())?;
        let df = plan.lf.clone().collect().map_err(|e| e.to_string())?;
        Ok(QueryOutput::DataFrame(df))
    }
}

#[cfg(feature = "polars")]
struct PolarsPlan {
    lf: LazyFrame,
    sql: String,
}

#[cfg(feature = "polars")]
impl QueryPlan for PolarsPlan {
    fn explain(&self) -> String {
        format!("PolarsPlan: {}", self.sql)
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

/// SQLite-backed engine.
pub struct SqliteEngine {
    path: String,
}

impl SqliteEngine {
    /// Create a new engine backed by a SQLite file.
    pub fn new(path: &str) -> Self {
        Self {
            path: path.to_string(),
        }
    }
}

impl QueryEngine for SqliteEngine {
    fn plan(&self, sql: &str) -> Result<Box<dyn QueryPlan>, String> {
        Ok(Box::new(SqlitePlan {
            sql: sql.to_string(),
        }))
    }

    fn execute(&self, plan: &dyn QueryPlan) -> Result<QueryOutput, String> {
        use rusqlite::types::ValueRef;
        use rusqlite::Connection;

        let sql = plan.explain();
        let actual_sql = sql.strip_prefix("SqlitePlan: ").unwrap_or(&sql);

        let conn = Connection::open(&self.path).map_err(|e| e.to_string())?;
        let mut stmt = conn.prepare(actual_sql).map_err(|e| e.to_string())?;

        let ncols = stmt.column_count();
        if ncols == 0 {
            // Non-query statement (INSERT, CREATE, UPDATE, etc.)
            stmt.execute([]).map_err(|e| e.to_string())?;
            return Ok(QueryOutput::Rows(Vec::new()));
        }

        let mut rows_out: Vec<Vec<Value>> = Vec::new();

        let mut rows = stmt.query([]).map_err(|e| e.to_string())?;
        while let Some(row) = rows.next().map_err(|e| e.to_string())? {
            let mut row_vec = Vec::with_capacity(ncols);
            for i in 0..ncols {
                let val = match row.get_ref(i).map_err(|e| e.to_string())? {
                    ValueRef::Null => Value::Null,
                    ValueRef::Integer(i) => Value::Number(i.into()),
                    ValueRef::Real(f) => {
                        Value::Number(serde_json::Number::from_f64(f).unwrap_or_else(|| 0.into()))
                    }
                    ValueRef::Text(t) => Value::String(String::from_utf8_lossy(t).to_string()),
                    ValueRef::Blob(b) => Value::String(format!("<{} bytes>", b.len())),
                };
                row_vec.push(val);
            }
            rows_out.push(row_vec);
        }

        Ok(QueryOutput::Rows(rows_out))
    }
}

struct SqlitePlan {
    sql: String,
}

impl QueryPlan for SqlitePlan {
    fn explain(&self) -> String {
        format!("SqlitePlan: {}", self.sql)
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_db_path() -> String {
        let path = std::env::temp_dir().join(format!("nom_engine_test_{}.db", std::process::id()));
        let _ = std::fs::remove_file(&path);
        path.to_string_lossy().to_string()
    }

    #[test]
    #[cfg(feature = "polars")]
    fn polars_engine_plan_ok() {
        let engine = PolarsEngine::new();
        let plan = engine.plan("SELECT 1");
        assert!(plan.is_ok());
    }

    #[test]
    fn sqlite_engine_plan_ok() {
        let engine = SqliteEngine::new(":memory:");
        let plan = engine.plan("SELECT 1");
        assert!(plan.is_ok());
    }

    #[test]
    #[cfg(feature = "polars")]
    fn polars_plan_explain_contains_sql() {
        let engine = PolarsEngine::new();
        let plan = engine.plan("SELECT 1").unwrap();
        assert!(plan.explain().contains("SELECT 1"));
    }

    #[test]
    fn sqlite_plan_explain_contains_sql() {
        let engine = SqliteEngine::new(&temp_db_path());
        let plan = engine.plan("SELECT 1").unwrap();
        assert!(plan.explain().contains("SELECT 1"));
    }

    #[test]
    #[cfg(feature = "polars")]
    fn polars_engine_execute_returns_dataframe() {
        let engine = PolarsEngine::new();
        let plan = engine.plan("SELECT 1").unwrap();
        let out = engine.execute(plan.as_ref()).unwrap();
        assert!(matches!(out, QueryOutput::DataFrame(_)));
    }

    #[test]
    fn sqlite_engine_execute_select_scalar() {
        let engine = SqliteEngine::new(&temp_db_path());
        let plan = engine.plan("SELECT 1 as x").unwrap();
        let out = engine.execute(plan.as_ref()).unwrap();
        match out {
            QueryOutput::Rows(rows) => {
                assert_eq!(rows.len(), 1);
                assert_eq!(rows[0].len(), 1);
                assert_eq!(rows[0][0], Value::Number(1i64.into()));
            }
            _ => panic!("expected Rows output"),
        }
    }

    #[test]
    fn sqlite_engine_execute_create_and_query() {
        let engine = SqliteEngine::new(&temp_db_path());
        let create_plan = engine
            .plan("CREATE TABLE t (id INTEGER PRIMARY KEY, name TEXT)")
            .unwrap();
        assert!(engine.execute(create_plan.as_ref()).is_ok());

        let insert_plan = engine
            .plan("INSERT INTO t (name) VALUES ('alice')")
            .unwrap();
        assert!(engine.execute(insert_plan.as_ref()).is_ok());

        let select_plan = engine.plan("SELECT * FROM t").unwrap();
        let out = engine.execute(select_plan.as_ref()).unwrap();
        match out {
            QueryOutput::Rows(rows) => {
                assert_eq!(rows.len(), 1);
                assert_eq!(rows[0].len(), 2);
            }
            _ => panic!("expected Rows output"),
        }
    }

    #[test]
    fn query_output_scalar_variant() {
        let scalar = QueryOutput::Scalar(Value::Bool(true));
        assert!(matches!(scalar, QueryOutput::Scalar(_)));
    }

    #[test]
    fn sqlite_engine_execute_empty_result() {
        let engine = SqliteEngine::new(&temp_db_path());
        let plan = engine.plan("SELECT 1 WHERE 0").unwrap();
        let out = engine.execute(plan.as_ref()).unwrap();
        match out {
            QueryOutput::Rows(rows) => assert!(rows.is_empty()),
            _ => panic!("expected Rows output"),
        }
    }

    #[test]
    fn sqlite_engine_multiple_rows() {
        let engine = SqliteEngine::new(&temp_db_path());
        let create = engine
            .plan("CREATE TABLE nums (n INTEGER)")
            .unwrap();
        engine.execute(create.as_ref()).unwrap();

        let insert = engine.plan("INSERT INTO nums VALUES (1),(2),(3)").unwrap();
        engine.execute(insert.as_ref()).unwrap();

        let select = engine.plan("SELECT * FROM nums").unwrap();
        let out = engine.execute(select.as_ref()).unwrap();
        match out {
            QueryOutput::Rows(rows) => {
                assert_eq!(rows.len(), 3);
            }
            _ => panic!("expected Rows output"),
        }
    }

    #[test]
    #[cfg(feature = "polars")]
    fn polars_engine_execute_dataframe_has_data() {
        let engine = PolarsEngine::new();
        let plan = engine.plan("SELECT 1").unwrap();
        let out = engine.execute(plan.as_ref()).unwrap();
        match out {
            QueryOutput::DataFrame(df) => {
                assert_eq!(df.height(), 1);
            }
            _ => panic!("expected DataFrame output"),
        }
    }

    #[test]
    fn sqlite_engine_null_value() {
        let engine = SqliteEngine::new(&temp_db_path());
        let plan = engine.plan("SELECT NULL as n").unwrap();
        let out = engine.execute(plan.as_ref()).unwrap();
        match out {
            QueryOutput::Rows(rows) => {
                assert_eq!(rows[0][0], Value::Null);
            }
            _ => panic!("expected Rows output"),
        }
    }

    #[test]
    #[cfg(feature = "polars")]
    fn polars_engine_csv_roundtrip() {
        let dir = std::env::temp_dir();
        let path = dir.join(format!("nom_polars_engine_csv_{}.csv", std::process::id()));
        std::fs::write(&path, "name,value\nalice,1\nbob,2\n").unwrap();

        let engine = PolarsEngine::new();
        engine.register_csv("t", path.to_str().unwrap()).unwrap();
        let plan = engine.plan("SELECT * FROM t WHERE value > 1").unwrap();
        let out = engine.execute(plan.as_ref()).unwrap();
        match out {
            QueryOutput::DataFrame(df) => {
                assert_eq!(df.height(), 1);
            }
            _ => panic!("expected DataFrame output"),
        }

        let _ = std::fs::remove_file(&path);
    }

    #[test]
    #[cfg(feature = "polars")]
    fn polars_engine_json_roundtrip() {
        let dir = std::env::temp_dir();
        let path = dir.join(format!("nom_polars_engine_json_{}.json", std::process::id()));
        std::fs::write(&path, "{\"name\":\"alice\",\"value\":1}\n{\"name\":\"bob\",\"value\":2}\n").unwrap();

        let engine = PolarsEngine::new();
        engine.register_json("t", path.to_str().unwrap()).unwrap();
        let plan = engine.plan("SELECT name FROM t").unwrap();
        let out = engine.execute(plan.as_ref()).unwrap();
        match out {
            QueryOutput::DataFrame(df) => {
                assert_eq!(df.height(), 2);
                assert!(df.get_column_names().contains(&"name"));
            }
            _ => panic!("expected DataFrame output"),
        }

        let _ = std::fs::remove_file(&path);
    }

    #[test]
    #[cfg(feature = "polars")]
    fn polars_engine_parquet_roundtrip() {
        let dir = std::env::temp_dir();
        let path = dir.join(format!("nom_polars_engine_parquet_{}.parquet", std::process::id()));

        // Write a small parquet file first
        let df = df! {
            "name" => ["alice", "bob"],
            "value" => [1i64, 2],
        }
        .unwrap();
        let mut file = std::fs::File::create(&path).unwrap();
        #[cfg(feature = "polars")]
        {
            use polars::prelude::ParquetWriter;
            ParquetWriter::new(&mut file)
                .finish(&mut df.clone())
                .unwrap();
        }

        let engine = PolarsEngine::new();
        engine.register_parquet("t", path.to_str().unwrap()).unwrap();
        let plan = engine.plan("SELECT * FROM t WHERE value = 2").unwrap();
        let out = engine.execute(plan.as_ref()).unwrap();
        match out {
            QueryOutput::DataFrame(df) => {
                assert_eq!(df.height(), 1);
            }
            _ => panic!("expected DataFrame output"),
        }

        let _ = std::fs::remove_file(&path);
    }
}
