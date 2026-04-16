//! GAP-12 — S5k window-aggregation clause tests.
//!
//! Covers:
//!  - Parse `window tumbling <N> seconds.` → WindowClause with kind=tumbling.
//!  - Parse `window sliding <N> seconds.` → kind=sliding.
//!  - Parse `window session <N> seconds.` → kind=session.
//!  - Parse without a window clause → window is None.
//!  - Window coexists with contracts and effects.
//!  - Malformed: unknown window kind → NOMX-S5k-unknown-window-kind.
//!  - Malformed: missing duration → NOMX-S5k-malformed-window.
//!  - Malformed: missing `seconds` → NOMX-S5k-malformed-window.
//!  - Malformed: missing closing `.` → NOMX-S5k-unterminated-window.
//!  - Clause shape registered in grammar DB for function kind.

use nom_concept::{
    NomtuItem, WindowClause,
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
    run_pipeline(src).expect_err("pipeline must reject malformed window clause")
}

// ── positive cases ────────────────────────────────────────────────────────────

#[test]
fn window_tumbling_extracted() {
    let src = "the function count_clicks is\n\
               intended to count clicks per window.\n\
               window tumbling 60 seconds.\n";
    let e = first_entity(src);
    assert_eq!(
        e.window,
        Some(WindowClause {
            kind: "tumbling".to_string(),
            duration_seconds: 60,
        }),
        "window must be extracted with kind=tumbling and duration=60"
    );
}

#[test]
fn window_sliding_extracted() {
    let src = "the function compute_avg is\n\
               intended to compute a sliding average.\n\
               window sliding 30 seconds.\n";
    let e = first_entity(src);
    assert_eq!(
        e.window,
        Some(WindowClause {
            kind: "sliding".to_string(),
            duration_seconds: 30,
        })
    );
}

#[test]
fn window_session_extracted() {
    let src = "the function group_sessions is\n\
               intended to group user activity into sessions.\n\
               window session 300 seconds.\n";
    let e = first_entity(src);
    assert_eq!(
        e.window,
        Some(WindowClause {
            kind: "session".to_string(),
            duration_seconds: 300,
        })
    );
}

#[test]
fn no_window_clause_yields_none() {
    let src = "the function no_window is\n\
               intended to process events without windowing.\n";
    let e = first_entity(src);
    assert_eq!(
        e.window, None,
        "window must be None when no window clause is present"
    );
}

#[test]
fn window_coexists_with_other_clauses() {
    let src = "the function aggregate_clicks is\n\
               intended to aggregate click events per tumbling window.\n\
               requires the stream is connected.\n\
               ensures the output rate is bounded.\n\
               window tumbling 10 seconds.\n\
               hazard late_events.\n";
    let e = first_entity(src);
    assert_eq!(
        e.window,
        Some(WindowClause {
            kind: "tumbling".to_string(),
            duration_seconds: 10,
        })
    );
    assert_eq!(e.contracts.len(), 2);
    assert_eq!(e.effects.len(), 1);
}

// ── negative cases ────────────────────────────────────────────────────────────

#[test]
fn window_unknown_kind_rejects() {
    let src = "the function foo is intended to bar.\nwindow hopping 60 seconds.\n";
    let err = parse_err(src);
    assert_eq!(
        err.reason, "unknown-window-kind",
        "expected unknown-window-kind, got: {:?}",
        err
    );
    assert!(
        err.detail.contains("hopping"),
        "diagnostic should name the bad kind: {}",
        err.detail
    );
}

#[test]
fn window_missing_seconds_rejects() {
    let src = "the function foo is intended to bar.\nwindow tumbling 60.\n";
    let err = parse_err(src);
    assert_eq!(err.reason, "malformed-window");
}

#[test]
fn window_missing_duration_rejects() {
    let src = "the function foo is intended to bar.\nwindow tumbling seconds.\n";
    let err = parse_err(src);
    assert_eq!(err.reason, "malformed-window");
}

#[test]
fn window_missing_closing_dot_rejects() {
    let src = "the function foo is intended to bar.\nwindow tumbling 60 seconds\n";
    let err = parse_err(src);
    assert_eq!(err.reason, "unterminated-window");
}

// ── clause_shapes DB registration ─────────────────────────────────────────────

#[test]
fn window_clause_shape_in_grammar_db() {
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
         VALUES ('function', 'window', 0, 11, \
         'window (tumbling|sliding|session) <N> seconds .', 'GAP-12')",
        [],
    )
    .unwrap();

    let (is_req, pos): (i64, i64) = conn
        .query_row(
            "SELECT is_required, position FROM clause_shapes \
             WHERE kind = 'function' AND clause_name = 'window'",
            [],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .expect("window row must exist");
    assert_eq!(
        is_req, 0,
        "window clause must be optional (is_required = 0)"
    );
    assert_eq!(pos, 11, "window clause must be at position 11");
}
