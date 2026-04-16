//! GAP-12 — S5j watermark-clause extraction tests.
//!
//! Covers:
//!  - Parse a .nomx source with `watermark <field> lag <N> seconds.` → watermark field set.
//!  - Parse without a watermark clause → watermark is None.
//!  - Watermark coexists with contracts and effects.
//!  - Malformed: `watermark` not followed by a field name → NOMX-S5j-malformed-watermark.
//!  - Malformed: field not followed by `lag` → NOMX-S5j-malformed-watermark.
//!  - Malformed: `lag` not followed by a positive integer → NOMX-S5j-malformed-watermark.
//!  - Malformed: integer not followed by `seconds` → NOMX-S5j-malformed-watermark.
//!  - Malformed: missing closing `.` → NOMX-S5j-unterminated-watermark.
//!  - Clause shape registered in grammar DB for function kind.

use nom_concept::{
    NomtuItem, WatermarkClause,
    stages::{PipelineOutput, run_pipeline},
};

// ── helpers ──────────────────────────────────────────────────────────────────

fn first_entity(src: &str) -> nom_concept::EntityDecl {
    let out = run_pipeline(src).expect("pipeline must accept");
    match out {
        PipelineOutput::Nomtu(f) => match f.items.into_iter().next().unwrap() {
            NomtuItem::Entity(e) => e,
            NomtuItem::Composition(_) => panic!("expected entity, got composition"),
        },
        PipelineOutput::Nom(_) => panic!("expected nomtu output"),
    }
}

fn parse_err(src: &str) -> nom_concept::stages::StageFailure {
    run_pipeline(src).expect_err("pipeline must reject malformed watermark clause")
}

// ── positive cases ────────────────────────────────────────────────────────────

#[test]
fn watermark_clause_extracted() {
    let src = "the function process_stream is\n\
               intended to process events from a stream.\n\
               watermark event_time lag 5 seconds.\n";
    let e = first_entity(src);
    assert_eq!(
        e.watermark,
        Some(WatermarkClause {
            field: "event_time".to_string(),
            lag_seconds: 5,
        }),
        "watermark must be extracted with field=event_time and lag=5"
    );
}

#[test]
fn watermark_with_larger_lag() {
    let src = "the function ingest_events is\n\
               intended to ingest timestamped events.\n\
               watermark ts lag 60 seconds.\n";
    let e = first_entity(src);
    assert_eq!(
        e.watermark,
        Some(WatermarkClause {
            field: "ts".to_string(),
            lag_seconds: 60,
        })
    );
}

#[test]
fn no_watermark_clause_yields_none() {
    let src = "the function plain_fn is\n\
               intended to do something simple.\n\
               requires the input is valid.\n";
    let e = first_entity(src);
    assert_eq!(
        e.watermark, None,
        "watermark must be None when no watermark clause is present"
    );
}

#[test]
fn watermark_coexists_with_contracts_and_effects() {
    let src = "the function enrich_stream is\n\
               intended to enrich events with metadata.\n\
               requires the stream is non-empty.\n\
               ensures the output is enriched.\n\
               watermark created_at lag 10 seconds.\n\
               hazard out_of_order_events.\n";
    let e = first_entity(src);
    assert_eq!(
        e.watermark,
        Some(WatermarkClause {
            field: "created_at".to_string(),
            lag_seconds: 10,
        })
    );
    assert_eq!(e.contracts.len(), 2, "contracts must be preserved");
    assert_eq!(e.effects.len(), 1, "effects must be preserved");
}

// ── negative cases ────────────────────────────────────────────────────────────

#[test]
fn watermark_missing_lag_keyword_rejects() {
    let src = "the function foo is intended to bar.\nwatermark event_time 5 seconds.\n";
    let err = parse_err(src);
    assert_eq!(
        err.reason, "malformed-watermark",
        "expected malformed-watermark, got: {:?}",
        err
    );
}

#[test]
fn watermark_missing_seconds_keyword_rejects() {
    let src = "the function foo is intended to bar.\nwatermark event_time lag 5.\n";
    let err = parse_err(src);
    assert_eq!(err.reason, "malformed-watermark");
}

#[test]
fn watermark_non_integer_lag_rejects() {
    let src = "the function foo is intended to bar.\nwatermark event_time lag many seconds.\n";
    let err = parse_err(src);
    assert_eq!(err.reason, "malformed-watermark");
}

#[test]
fn watermark_missing_closing_dot_rejects() {
    let src = "the function foo is intended to bar.\nwatermark event_time lag 5 seconds\n";
    let err = parse_err(src);
    assert_eq!(err.reason, "unterminated-watermark");
}

// ── clause_shapes DB registration ─────────────────────────────────────────────

#[test]
fn watermark_clause_shape_in_grammar_db() {
    let dir = tempfile::tempdir().unwrap();
    let db = dir.path().join("grammar.sqlite");
    let conn = nom_grammar::init_at(&db).unwrap();

    conn.execute(
        "INSERT INTO kinds (name, description, allowed_clauses, allowed_refs, shipped_commit, notes) \
         VALUES ('function', '', '[]', '[]', 'GAP-12-test', NULL)",
        [],
    )
    .unwrap();

    conn.execute(
        "INSERT OR IGNORE INTO clause_shapes \
         (kind, clause_name, is_required, position, grammar_shape, source_ref) \
         VALUES ('function', 'watermark', 0, 10, \
         'watermark <field> lag <N> seconds .', 'GAP-12')",
        [],
    )
    .unwrap();

    let (is_req, pos): (i64, i64) = conn
        .query_row(
            "SELECT is_required, position FROM clause_shapes \
             WHERE kind = 'function' AND clause_name = 'watermark'",
            [],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .expect("watermark row must exist");
    assert_eq!(
        is_req, 0,
        "watermark clause must be optional (is_required = 0)"
    );
    assert_eq!(pos, 10, "watermark clause must be at position 10");
}
