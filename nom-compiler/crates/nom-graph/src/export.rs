//! Phase 3a: Cypher-compatible CSV export for uid-addressed nodes.
//!
//! Emits LadybugDB's `LOAD FROM` CSV dump shape so a `nom-graph` dump
//! can be roundtripped through GitNexus (`gitnexus cypher < import.cypher`)
//! or any Cypher-speaking graph DB. Reads the uid_nodes HashMap shipped
//! in Phase 2b; edges are exported in Phase 3b after Phase 2c migrates
//! the Vec<NomtuEdge> storage to be uid-addressed.
//!
//! Spec: `docs/superpowers/specs/2026-04-14-graph-durability-design.md`
//! (Phase 3). Shape: one `nodes_<Label>.csv` per node label, one
//! `edges_<Type>.csv` per edge type, one `import.cypher` LOAD FROM
//! script that ingests them.
//!
//! ## Phase 3a scope
//!
//! - `export_to_dir(&NomtuGraph, &Path) -> Result<ExportSummary, _>`
//! - `nodes_NomtuNode.csv` with header `uid,word,variant,language,kind,body_hash`
//! - `import.cypher` declaring schema + LOAD FROM for nodes
//! - Deterministic ordering (sorted by uid) so reruns are byte-identical
//! - Field escaping: RFC 4180 quoting for any value containing `,"\\n`
//!
//! ## Phase 3b (deferred)
//!
//! - `edges_*.csv` per `EdgeType` variant (requires uid-addressed
//!   edges from Phase 2c; today's `Vec<NomtuEdge>` uses (word, variant)
//!   pairs, not uids, so we'd have to synthesize uids at export time
//!   which is error-prone for edges touching retired nodes)
//! - Roundtrip gate test (`gitnexus cypher < import.cypher` must
//!   report the same `count(n)` as Nom's own count) — pending until
//!   edges land, otherwise the gate just counts nodes which is trivial

use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};

use thiserror::Error;

use crate::NomtuGraph;

#[derive(Debug, Error)]
pub enum ExportError {
    #[error("output dir {0:?} is not empty; pass force=true to clobber")]
    NonEmptyOutDir(PathBuf),
    #[error("io error on {path:?}: {source}")]
    Io {
        path: PathBuf,
        #[source]
        source: io::Error,
    },
}

/// Summary of what was written; callers can assert + log.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExportSummary {
    pub nodes_written: usize,
    pub files_written: Vec<PathBuf>,
}

/// Export the graph's uid-addressed nodes to LadybugDB CSV dump shape.
///
/// If `out_dir` exists and is non-empty, returns `ExportError::NonEmptyOutDir`
/// unless `force=true`. Creates `out_dir` if it doesn't exist.
pub fn export_to_dir(
    graph: &NomtuGraph,
    out_dir: &Path,
    force: bool,
) -> Result<ExportSummary, ExportError> {
    if out_dir.exists() {
        let non_empty = fs::read_dir(out_dir)
            .map_err(|e| ExportError::Io { path: out_dir.into(), source: e })?
            .next()
            .is_some();
        if non_empty && !force {
            return Err(ExportError::NonEmptyOutDir(out_dir.into()));
        }
    } else {
        fs::create_dir_all(out_dir).map_err(|e| ExportError::Io {
            path: out_dir.into(),
            source: e,
        })?;
    }

    let mut files_written = Vec::new();

    // Nodes CSV — deterministic uid-sorted order.
    let nodes_path = out_dir.join("nodes_NomtuNode.csv");
    write_nodes_csv(graph, &nodes_path)?;
    files_written.push(nodes_path);

    // Import script.
    let import_path = out_dir.join("import.cypher");
    write_import_cypher(&import_path)?;
    files_written.push(import_path);

    Ok(ExportSummary {
        nodes_written: graph.uid_nodes.len(),
        files_written,
    })
}

fn write_nodes_csv(graph: &NomtuGraph, path: &Path) -> Result<(), ExportError> {
    let file = fs::File::create(path).map_err(|e| ExportError::Io {
        path: path.into(),
        source: e,
    })?;
    let mut w = io::BufWriter::new(file);
    writeln!(w, "uid,word,variant,language,kind,body_hash").map_err(|e| ExportError::Io {
        path: path.into(),
        source: e,
    })?;
    // Sort by uid for deterministic output (byte-identical across runs).
    let mut uids: Vec<&String> = graph.uid_nodes.keys().collect();
    uids.sort();
    for uid in uids {
        let node = &graph.uid_nodes[uid];
        writeln!(
            w,
            "{},{},{},{},{},{}",
            csv_escape(uid),
            csv_escape(&node.word),
            csv_escape_opt(node.variant.as_deref()),
            csv_escape(&node.language),
            csv_escape(&node.kind),
            csv_escape_opt(node.body_hash.as_deref()),
        )
        .map_err(|e| ExportError::Io { path: path.into(), source: e })?;
    }
    w.flush()
        .map_err(|e| ExportError::Io { path: path.into(), source: e })?;
    Ok(())
}

fn write_import_cypher(path: &Path) -> Result<(), ExportError> {
    let script = r#"// nom-graph export — LadybugDB LOAD FROM script (Phase 3a, nodes-only).
// Edges land in Phase 3b once uid-addressed edges ship.
//
// Usage:  gitnexus cypher < import.cypher
// Or:     npx kuzu-cli --init import.cypher <graph.db>

// Schema — idempotent (CREATE IF NOT EXISTS not in standard Cypher,
// but LadybugDB accepts this idiom).
CREATE NODE TABLE IF NOT EXISTS NomtuNode(
    uid       STRING,
    word      STRING,
    variant   STRING,
    language  STRING,
    kind      STRING,
    body_hash STRING,
    PRIMARY KEY (uid)
);

// Load nodes.
COPY NomtuNode FROM "nodes_NomtuNode.csv" (HEADER=true);
"#;
    fs::write(path, script).map_err(|e| ExportError::Io {
        path: path.into(),
        source: e,
    })?;
    Ok(())
}

/// RFC 4180 CSV escape: wrap in double quotes and double any internal
/// double quotes, but only if the value contains a `,`, `"`, or newline.
fn csv_escape(s: &str) -> String {
    if s.contains(',') || s.contains('"') || s.contains('\n') || s.contains('\r') {
        let inner = s.replace('"', "\"\"");
        format!("\"{inner}\"")
    } else {
        s.to_string()
    }
}

fn csv_escape_opt(s: Option<&str>) -> String {
    s.map(csv_escape).unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;
    use nom_types::NomtuEntry;

    fn mk(word: &str, kind: &str, body: Option<&str>) -> NomtuEntry {
        NomtuEntry {
            word: word.into(),
            kind: kind.into(),
            body_hash: body.map(|s| s.into()),
            language: "rust".into(),
            ..Default::default()
        }
    }

    fn tmp_dir(label: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!(
            "nom_export_{}_{}",
            label,
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&d);
        d
    }

    #[test]
    fn export_creates_expected_files_for_populated_graph() {
        let mut g = NomtuGraph::new();
        g.upsert_entry(&mk("add", "function", Some("h1")));
        g.upsert_entry(&mk("mul", "function", Some("h2")));
        let dir = tmp_dir("populated");
        let summary = export_to_dir(&g, &dir, false).unwrap();

        assert_eq!(summary.nodes_written, 2);
        assert!(dir.join("nodes_NomtuNode.csv").exists());
        assert!(dir.join("import.cypher").exists());

        let csv = fs::read_to_string(dir.join("nodes_NomtuNode.csv")).unwrap();
        let lines: Vec<&str> = csv.lines().collect();
        assert_eq!(lines[0], "uid,word,variant,language,kind,body_hash");
        assert_eq!(lines.len(), 3, "1 header + 2 data rows");
        // Rows sorted by uid.
        assert!(lines[1] < lines[2]);

        let cypher = fs::read_to_string(dir.join("import.cypher")).unwrap();
        assert!(cypher.contains("CREATE NODE TABLE IF NOT EXISTS NomtuNode"));
        assert!(cypher.contains("COPY NomtuNode FROM \"nodes_NomtuNode.csv\""));

        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn export_empty_graph_emits_headers_only() {
        let g = NomtuGraph::new();
        let dir = tmp_dir("empty");
        let summary = export_to_dir(&g, &dir, false).unwrap();
        assert_eq!(summary.nodes_written, 0);
        let csv = fs::read_to_string(dir.join("nodes_NomtuNode.csv")).unwrap();
        assert_eq!(csv.trim(), "uid,word,variant,language,kind,body_hash");
        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn export_is_deterministic_across_runs() {
        let mut g = NomtuGraph::new();
        for w in &["alpha", "gamma", "beta", "delta"] {
            g.upsert_entry(&mk(w, "function", Some("h")));
        }
        let dir1 = tmp_dir("deterministic1");
        let dir2 = tmp_dir("deterministic2");
        export_to_dir(&g, &dir1, false).unwrap();
        export_to_dir(&g, &dir2, false).unwrap();
        let a = fs::read_to_string(dir1.join("nodes_NomtuNode.csv")).unwrap();
        let b = fs::read_to_string(dir2.join("nodes_NomtuNode.csv")).unwrap();
        assert_eq!(a, b, "two exports must be byte-identical");
        fs::remove_dir_all(&dir1).ok();
        fs::remove_dir_all(&dir2).ok();
    }

    #[test]
    fn export_refuses_non_empty_dir_without_force() {
        let mut g = NomtuGraph::new();
        g.upsert_entry(&mk("x", "function", Some("h")));
        let dir = tmp_dir("clobber_guard");
        fs::create_dir_all(&dir).unwrap();
        fs::write(dir.join("stale.txt"), "pre-existing").unwrap();
        let err = export_to_dir(&g, &dir, false).expect_err("must reject non-empty dir");
        assert!(matches!(err, ExportError::NonEmptyOutDir(_)));
        // Second call with force=true succeeds.
        let ok = export_to_dir(&g, &dir, true);
        assert!(ok.is_ok());
        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn csv_escape_wraps_values_with_commas_and_quotes() {
        assert_eq!(csv_escape("plain"), "plain");
        assert_eq!(csv_escape("has,comma"), "\"has,comma\"");
        assert_eq!(csv_escape(r#"has"quote"#), r#""has""quote""#);
        assert_eq!(csv_escape("multi\nline"), "\"multi\nline\"");
        assert_eq!(csv_escape_opt(None), "");
        assert_eq!(csv_escape_opt(Some("x")), "x");
    }
}
