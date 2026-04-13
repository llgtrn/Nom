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
    /// Phase 3b: edges emitted per EdgeType variant name, sorted.
    pub edges_written: Vec<(String, usize)>,
    /// Phase 3b: edges whose (word, variant) endpoints couldn't be
    /// resolved against word_variant_index (missing node or ambiguous
    /// multi-kind match). Skipped, not fatal — count is reported so
    /// callers can log / warn.
    pub edges_skipped: usize,
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

    // Edges CSVs — one per EdgeType variant used by this graph. Phase 3b.
    let (edges_written, edges_skipped) =
        write_edges_csvs(graph, out_dir, &mut files_written)?;

    // Import script (schema + LOAD FROM for nodes + all emitted edge kinds).
    let import_path = out_dir.join("import.cypher");
    write_import_cypher(&import_path, &edges_written)?;
    files_written.push(import_path);

    Ok(ExportSummary {
        nodes_written: graph.uid_nodes.len(),
        files_written,
        edges_written,
        edges_skipped,
    })
}

/// Build `(word, variant) -> uid` resolver from the uid-addressed
/// storage. Returns `None` on ambiguous matches (same word+variant
/// maps to multiple kinds → caller skips that edge).
fn build_endpoint_resolver(
    graph: &NomtuGraph,
) -> std::collections::HashMap<(String, Option<String>), Option<String>> {
    let mut resolver: std::collections::HashMap<
        (String, Option<String>),
        Option<String>,
    > = std::collections::HashMap::new();
    for ((word, _kind, variant), uid) in &graph.word_variant_index {
        let key = (word.clone(), variant.clone());
        resolver
            .entry(key)
            .and_modify(|existing| {
                // If we already saw a uid for this (word, variant), flag
                // ambiguous by setting to None. Callers filter these out.
                if existing.as_deref() != Some(uid.as_str()) {
                    *existing = None;
                }
            })
            .or_insert_with(|| Some(uid.clone()));
    }
    resolver
}

fn write_edges_csvs(
    graph: &NomtuGraph,
    out_dir: &Path,
    files_written: &mut Vec<PathBuf>,
) -> Result<(Vec<(String, usize)>, usize), ExportError> {
    use crate::EdgeType;

    let resolver = build_endpoint_resolver(graph);
    let mut by_type: std::collections::BTreeMap<String, Vec<(String, String, f64)>> =
        std::collections::BTreeMap::new();
    let mut skipped = 0usize;

    for edge in graph.edges() {
        let from_key = (edge.from_word.clone(), edge.from_variant.clone());
        let to_key = (edge.to_word.clone(), edge.to_variant.clone());
        let from_uid = match resolver.get(&from_key).and_then(|v| v.clone()) {
            Some(u) => u,
            None => {
                skipped += 1;
                continue;
            }
        };
        let to_uid = match resolver.get(&to_key).and_then(|v| v.clone()) {
            Some(u) => u,
            None => {
                skipped += 1;
                continue;
            }
        };
        let type_name = edge_type_name(edge.edge_type);
        by_type
            .entry(type_name)
            .or_default()
            .push((from_uid, to_uid, edge.confidence));
    }

    // Emit one CSV per edge type with rows, deterministic ordering.
    let mut summary: Vec<(String, usize)> = Vec::new();
    for (type_name, mut rows) in by_type {
        rows.sort_by(|a, b| (&a.0, &a.1).cmp(&(&b.0, &b.1)));
        let path = out_dir.join(format!("edges_{type_name}.csv"));
        let file = fs::File::create(&path).map_err(|e| ExportError::Io {
            path: path.clone(),
            source: e,
        })?;
        let mut w = io::BufWriter::new(file);
        writeln!(w, "from_uid,to_uid,confidence").map_err(|e| ExportError::Io {
            path: path.clone(),
            source: e,
        })?;
        for (from, to, conf) in &rows {
            writeln!(w, "{},{},{:.6}", csv_escape(from), csv_escape(to), conf).map_err(
                |e| ExportError::Io { path: path.clone(), source: e },
            )?;
        }
        w.flush()
            .map_err(|e| ExportError::Io { path: path.clone(), source: e })?;
        summary.push((type_name, rows.len()));
        files_written.push(path);
    }

    Ok((summary, skipped))
}

fn edge_type_name(t: crate::EdgeType) -> String {
    format!("{t:?}")
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

fn write_import_cypher(
    path: &Path,
    edges: &[(String, usize)],
) -> Result<(), ExportError> {
    let mut script = String::from(
        r#"// nom-graph export — LadybugDB LOAD FROM script (Phase 3a+3b).
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
"#,
    );

    // Phase 3b: REL TABLE per edge kind that actually has rows, then COPY FROM.
    if !edges.is_empty() {
        script.push_str("\n// Edge rel tables + loads (Phase 3b).\n");
        for (type_name, _count) in edges {
            script.push_str(&format!(
                "CREATE REL TABLE IF NOT EXISTS {type_name}(FROM NomtuNode TO NomtuNode, confidence DOUBLE);\n"
            ));
        }
        for (type_name, _count) in edges {
            script.push_str(&format!(
                "COPY {type_name} FROM \"edges_{type_name}.csv\" (HEADER=true);\n"
            ));
        }
    }

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
        assert!(summary.edges_written.is_empty());
        assert_eq!(summary.edges_skipped, 0);
        let csv = fs::read_to_string(dir.join("nodes_NomtuNode.csv")).unwrap();
        assert_eq!(csv.trim(), "uid,word,variant,language,kind,body_hash");
        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn export_emits_edges_csv_with_resolved_endpoints() {
        let mut g = NomtuGraph::new();
        // Seed nodes via upsert so word_variant_index knows about them.
        let a = g
            .upsert_entry(&mk("add", "function", Some("h1")))
            .current_uid()
            .clone();
        let m = g
            .upsert_entry(&mk("mul", "function", Some("h2")))
            .current_uid()
            .clone();
        // Inject an edge directly into the legacy Vec<NomtuEdge>. build_call_edges
        // path would normally populate this; the export only cares that the edge
        // exists with matching (word, variant) endpoints.
        g.add_edge(crate::NomtuEdge {
            from_word: "add".into(),
            from_variant: None,
            to_word: "mul".into(),
            to_variant: None,
            edge_type: crate::EdgeType::Calls,
            confidence: 1.0,
        });

        let dir = tmp_dir("edges");
        let summary = export_to_dir(&g, &dir, false).unwrap();
        assert_eq!(summary.edges_skipped, 0);
        assert_eq!(summary.edges_written, vec![("Calls".to_string(), 1)]);

        let csv = fs::read_to_string(dir.join("edges_Calls.csv")).unwrap();
        let lines: Vec<&str> = csv.lines().collect();
        assert_eq!(lines[0], "from_uid,to_uid,confidence");
        assert!(lines[1].starts_with(&format!("{a},{m},")));
        assert!(lines[1].ends_with("1.000000"));
        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn export_skips_edges_with_unknown_endpoints() {
        let mut g = NomtuGraph::new();
        g.upsert_entry(&mk("add", "function", Some("h1")));
        // Edge references a `mul` that was NEVER upserted → endpoint unresolved.
        g.add_edge(crate::NomtuEdge {
            from_word: "add".into(),
            from_variant: None,
            to_word: "mul_nowhere".into(),
            to_variant: None,
            edge_type: crate::EdgeType::Calls,
            confidence: 0.8,
        });
        let dir = tmp_dir("skipped");
        let summary = export_to_dir(&g, &dir, false).unwrap();
        assert_eq!(summary.edges_skipped, 1);
        assert!(summary.edges_written.is_empty(), "no CSV for unresolved-only edge type");
        // edges_Calls.csv must NOT exist (no rows emitted).
        assert!(!dir.join("edges_Calls.csv").exists());
        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn import_cypher_declares_rel_tables_for_emitted_edges() {
        let mut g = NomtuGraph::new();
        g.upsert_entry(&mk("x", "function", Some("h1")));
        g.upsert_entry(&mk("y", "function", Some("h2")));
        g.add_edge(crate::NomtuEdge {
            from_word: "x".into(),
            from_variant: None,
            to_word: "y".into(),
            to_variant: None,
            edge_type: crate::EdgeType::Imports,
            confidence: 0.9,
        });
        let dir = tmp_dir("rel_tables");
        export_to_dir(&g, &dir, false).unwrap();
        let cypher = fs::read_to_string(dir.join("import.cypher")).unwrap();
        assert!(cypher.contains("CREATE REL TABLE IF NOT EXISTS Imports"));
        assert!(cypher.contains("COPY Imports FROM \"edges_Imports.csv\""));
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
