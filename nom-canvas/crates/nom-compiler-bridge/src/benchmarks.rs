#![deny(unsafe_code)]
// Typed side-tables: entry_benchmarks + flow_steps
// Only compiled when the `compiler` feature is active (rusqlite available).

#[cfg(feature = "compiler")]
use rusqlite::params;

/// Performance measurement for a single nomtu entry run.
pub struct EntryBenchmark {
    pub run_id: String,
    pub entry_hash: String,
    pub platform: String,
    pub compiler_hash: String,
    pub workload_key: String,
    pub wall_ns: i64,
    pub cpu_ns: i64,
    pub mem_bytes: i64,
    /// Optional JSON blob for custom counters.
    pub custom_counters: Option<String>,
}

/// One step in a recorded execution flow artifact.
pub struct FlowStep {
    pub artifact_id: String,
    pub step_index: i32,
    pub entry_hash: String,
    pub started_ns: i64,
    pub ended_ns: i64,
    pub input_hash: Option<String>,
    pub output_hash: Option<String>,
}

/// Insert a benchmark row. Idempotent on (run_id, entry_hash, platform).
#[cfg(feature = "compiler")]
pub fn insert_benchmark(conn: &rusqlite::Connection, b: &EntryBenchmark) -> rusqlite::Result<()> {
    conn.execute(
        "INSERT OR REPLACE INTO entry_benchmarks \
         (run_id, entry_hash, platform, compiler_hash, workload_key, \
          wall_ns, cpu_ns, mem_bytes, custom_counters) \
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
        params![
            b.run_id,
            b.entry_hash,
            b.platform,
            b.compiler_hash,
            b.workload_key,
            b.wall_ns,
            b.cpu_ns,
            b.mem_bytes,
            b.custom_counters,
        ],
    )?;
    Ok(())
}

/// Insert a flow step row. Idempotent on (artifact_id, step_index).
#[cfg(feature = "compiler")]
pub fn insert_flow_step(conn: &rusqlite::Connection, s: &FlowStep) -> rusqlite::Result<()> {
    conn.execute(
        "INSERT OR REPLACE INTO flow_steps \
         (artifact_id, step_index, entry_hash, started_ns, ended_ns, \
          input_hash, output_hash) \
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
        params![
            s.artifact_id,
            s.step_index,
            s.entry_hash,
            s.started_ns,
            s.ended_ns,
            s.input_hash,
            s.output_hash,
        ],
    )?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn entry_benchmark_fields_set_correctly() {
        let b = EntryBenchmark {
            run_id: "run-001".into(),
            entry_hash: "abc123".into(),
            platform: "x86_64-linux".into(),
            compiler_hash: "comp-v1".into(),
            workload_key: "sort-1k".into(),
            wall_ns: 1_000_000,
            cpu_ns: 980_000,
            mem_bytes: 4096,
            custom_counters: Some(r#"{"cache_misses":12}"#.into()),
        };
        assert_eq!(b.run_id, "run-001");
        assert_eq!(b.entry_hash, "abc123");
        assert_eq!(b.platform, "x86_64-linux");
        assert_eq!(b.compiler_hash, "comp-v1");
        assert_eq!(b.workload_key, "sort-1k");
        assert_eq!(b.wall_ns, 1_000_000);
        assert_eq!(b.cpu_ns, 980_000);
        assert_eq!(b.mem_bytes, 4096);
        assert!(b.custom_counters.is_some());
    }

    #[test]
    fn entry_benchmark_optional_custom_counters_none() {
        let b = EntryBenchmark {
            run_id: "run-002".into(),
            entry_hash: "def456".into(),
            platform: "aarch64-macos".into(),
            compiler_hash: "comp-v2".into(),
            workload_key: "fibonacci-40".into(),
            wall_ns: 500_000,
            cpu_ns: 490_000,
            mem_bytes: 2048,
            custom_counters: None,
        };
        assert!(b.custom_counters.is_none());
        assert_eq!(b.wall_ns, 500_000);
    }

    #[test]
    fn flow_step_fields_set_correctly() {
        let s = FlowStep {
            artifact_id: "artifact-001".into(),
            step_index: 0,
            entry_hash: "abc123".into(),
            started_ns: 1_000_000_000,
            ended_ns: 1_001_000_000,
            input_hash: Some("in-hash".into()),
            output_hash: Some("out-hash".into()),
        };
        assert_eq!(s.artifact_id, "artifact-001");
        assert_eq!(s.step_index, 0);
        assert_eq!(s.entry_hash, "abc123");
        assert_eq!(s.started_ns, 1_000_000_000);
        assert_eq!(s.ended_ns, 1_001_000_000);
        assert_eq!(s.input_hash.as_deref(), Some("in-hash"));
        assert_eq!(s.output_hash.as_deref(), Some("out-hash"));
    }

    #[test]
    fn flow_step_optional_hashes_none() {
        let s = FlowStep {
            artifact_id: "artifact-002".into(),
            step_index: 3,
            entry_hash: "xyz789".into(),
            started_ns: 2_000_000_000,
            ended_ns: 2_002_000_000,
            input_hash: None,
            output_hash: None,
        };
        assert!(s.input_hash.is_none());
        assert!(s.output_hash.is_none());
        assert_eq!(s.step_index, 3);
    }

    #[cfg(feature = "compiler")]
    mod with_sqlite {
        use super::super::*;
        use rusqlite::Connection;

        fn in_memory_db() -> Connection {
            let conn = Connection::open_in_memory().unwrap();
            conn.execute_batch(
                "CREATE TABLE entries (
                     id TEXT PRIMARY KEY,
                     word TEXT NOT NULL,
                     kind TEXT NOT NULL,
                     language TEXT NOT NULL,
                     status TEXT NOT NULL
                 );
                 CREATE TABLE entry_benchmarks (
                     run_id        TEXT NOT NULL,
                     entry_hash    TEXT NOT NULL,
                     platform      TEXT NOT NULL,
                     compiler_hash TEXT NOT NULL,
                     workload_key  TEXT NOT NULL,
                     wall_ns       INTEGER NOT NULL,
                     cpu_ns        INTEGER NOT NULL,
                     mem_bytes     INTEGER NOT NULL,
                     custom_counters TEXT,
                     recorded_at   TEXT NOT NULL DEFAULT (datetime('now')),
                     PRIMARY KEY (run_id, entry_hash, platform)
                 );
                 CREATE TABLE flow_steps (
                     artifact_id TEXT NOT NULL,
                     step_index  INTEGER NOT NULL,
                     entry_hash  TEXT NOT NULL,
                     started_ns  INTEGER NOT NULL,
                     ended_ns    INTEGER NOT NULL,
                     input_hash  TEXT,
                     output_hash TEXT,
                     PRIMARY KEY (artifact_id, step_index)
                 );",
            )
            .unwrap();
            // Insert a seed entries row so FK references resolve.
            conn.execute(
                "INSERT INTO entries (id, word, kind, language, status) VALUES ('abc123', 'sort', 'function', 'rust', 'complete')",
                [],
            ).unwrap();
            conn
        }

        #[test]
        fn insert_benchmark_round_trips() {
            let conn = in_memory_db();
            let b = EntryBenchmark {
                run_id: "run-rt".into(),
                entry_hash: "abc123".into(),
                platform: "x86_64".into(),
                compiler_hash: "c-v1".into(),
                workload_key: "wk-1".into(),
                wall_ns: 100,
                cpu_ns: 90,
                mem_bytes: 512,
                custom_counters: None,
            };
            insert_benchmark(&conn, &b).unwrap();
            let count: i64 = conn
                .query_row("SELECT COUNT(*) FROM entry_benchmarks", [], |r| r.get(0))
                .unwrap();
            assert_eq!(count, 1);
        }

        #[test]
        fn insert_flow_step_round_trips() {
            let conn = in_memory_db();
            let s = FlowStep {
                artifact_id: "art-rt".into(),
                step_index: 0,
                entry_hash: "abc123".into(),
                started_ns: 1000,
                ended_ns: 2000,
                input_hash: Some("in".into()),
                output_hash: Some("out".into()),
            };
            insert_flow_step(&conn, &s).unwrap();
            let count: i64 = conn
                .query_row("SELECT COUNT(*) FROM flow_steps", [], |r| r.get(0))
                .unwrap();
            assert_eq!(count, 1);
        }
    }
}
