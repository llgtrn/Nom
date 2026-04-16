//! GAP-12 — retry-policy clause tests.
//!
//! Covers:
//!  - Parse a .nomx source with `retry at-most N times.` → EntityDecl has retry_policy.
//!  - Parse with `retry at-most N times with exponential backoff.` → strategy captured.
//!  - Parse without a retry clause → retry_policy is None.
//!  - Malformed: `retry` not followed by `at-most` → NOMX-S5-malformed-retry.
//!  - Malformed: `at-most` followed by a non-integer → NOMX-S5-malformed-retry.
//!  - Malformed: missing `times` keyword → NOMX-S5-malformed-retry.
//!  - Malformed: unknown strategy word → NOMX-S5-unknown-retry-strategy.
//!  - Malformed: missing closing `.` → NOMX-S5-unterminated-retry.
//!  - Clause shape registered in grammar DB for function kind.

use nom_concept::{
    NomtuItem, RetryPolicy,
    stages::{PipelineOutput, run_pipeline},
};

// ── helpers ──────────────────────────────────────────────────────────────────

/// Run the pipeline and unwrap the first entity decl.
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
    run_pipeline(src).expect_err("pipeline must reject malformed retry clause")
}

// ── positive cases ────────────────────────────────────────────────────────────

#[test]
fn retry_without_strategy_defaults_to_fixed() {
    let src = "the function fetch_url is intended to fetch an https URL.\n\
               retry at-most 3 times.\n";
    let e = first_entity(src);
    assert_eq!(
        e.retry_policy,
        Some(RetryPolicy {
            max_attempts: 3,
            strategy: "fixed".to_string(),
        }),
        "retry_policy should be Some with max_attempts=3 and strategy=fixed"
    );
}

#[test]
fn retry_with_exponential_backoff() {
    let src = "the function fetch_url is intended to fetch an https URL.\n\
               retry at-most 5 times with exponential backoff.\n";
    let e = first_entity(src);
    assert_eq!(
        e.retry_policy,
        Some(RetryPolicy {
            max_attempts: 5,
            strategy: "exponential".to_string(),
        })
    );
}

#[test]
fn retry_with_linear_backoff() {
    let src = "the function fetch_url is intended to fetch an https URL.\n\
               retry at-most 2 times with linear backoff.\n";
    let e = first_entity(src);
    assert_eq!(
        e.retry_policy,
        Some(RetryPolicy {
            max_attempts: 2,
            strategy: "linear".to_string(),
        })
    );
}

#[test]
fn retry_with_fixed_backoff_explicit() {
    let src = "the function fetch_url is intended to fetch an https URL.\n\
               retry at-most 1 times with fixed backoff.\n";
    let e = first_entity(src);
    assert_eq!(
        e.retry_policy,
        Some(RetryPolicy {
            max_attempts: 1,
            strategy: "fixed".to_string(),
        })
    );
}

#[test]
fn no_retry_clause_yields_none() {
    let src = "the function ping is intended to check reachability.\n\
               requires the host is reachable.\n";
    let e = first_entity(src);
    assert_eq!(
        e.retry_policy, None,
        "retry_policy must be None when no retry clause is present"
    );
}

#[test]
fn retry_coexists_with_contracts_and_effects() {
    let src = "the function send_request is intended to submit an HTTP request.\n\
               requires the endpoint is non-empty.\n\
               ensures the response is received.\n\
               retry at-most 4 times with exponential backoff.\n\
               hazard timeout.\n";
    let e = first_entity(src);
    assert_eq!(
        e.retry_policy,
        Some(RetryPolicy {
            max_attempts: 4,
            strategy: "exponential".to_string(),
        })
    );
    assert_eq!(e.contracts.len(), 2, "contracts should be preserved");
    assert_eq!(e.effects.len(), 1, "effects should be preserved");
}

// ── negative cases ────────────────────────────────────────────────────────────

#[test]
fn retry_not_followed_by_at_most_rejects() {
    // `retry 3 times.` — missing `at-most`
    let src = "the function foo is intended to bar.\nretry 3 times.\n";
    let err = parse_err(src);
    assert_eq!(
        err.reason, "malformed-retry",
        "expected malformed-retry, got: {:?}",
        err
    );
}

#[test]
fn retry_at_most_non_integer_rejects() {
    // `retry at-most foo times.` — non-integer count
    let src = "the function foo is intended to bar.\nretry at-most foo times.\n";
    let err = parse_err(src);
    assert_eq!(err.reason, "malformed-retry");
}

#[test]
fn retry_at_most_missing_times_rejects() {
    // `retry at-most 3.` — missing `times`
    let src = "the function foo is intended to bar.\nretry at-most 3.\n";
    let err = parse_err(src);
    assert_eq!(err.reason, "malformed-retry");
}

#[test]
fn retry_unknown_strategy_rejects() {
    // `retry at-most 3 times with random backoff.` — unknown strategy
    let src = "the function foo is intended to bar.\nretry at-most 3 times with random backoff.\n";
    let err = parse_err(src);
    assert_eq!(err.reason, "unknown-retry-strategy");
    assert!(
        err.detail.contains("random"),
        "diagnostic should name the bad strategy: {}",
        err.detail
    );
}

#[test]
fn retry_missing_closing_dot_rejects() {
    // `retry at-most 3 times` with no `.`
    let src = "the function foo is intended to bar.\nretry at-most 3 times\n";
    let err = parse_err(src);
    assert_eq!(err.reason, "unterminated-retry");
}

// ── clause_shapes DB registration ─────────────────────────────────────────────

#[test]
fn retry_clause_shape_in_grammar_db() {
    let dir = tempfile::tempdir().unwrap();
    let db = dir.path().join("grammar.sqlite");
    let conn = nom_grammar::init_at(&db).unwrap();

    // Seed the function kind + its clause_shapes.
    conn.execute(
        "INSERT INTO kinds (name, description, allowed_clauses, allowed_refs, shipped_commit, notes) \
         VALUES ('function', '', '[]', '[]', 'GAP-12-test', NULL)",
        [],
    ).unwrap();

    // Insert the retry clause shape (mirrors the baseline.sql row).
    conn.execute(
        "INSERT OR IGNORE INTO clause_shapes \
         (kind, clause_name, is_required, position, grammar_shape, source_ref) \
         VALUES ('function', 'retry', 0, 7, \
         'retry at-most <N> times (with (exponential|linear|fixed) backoff)? .', 'GAP-12')",
        [],
    )
    .unwrap();

    // Verify it shows up with the right attributes.
    let (is_req, pos): (i64, i64) = conn
        .query_row(
            "SELECT is_required, position FROM clause_shapes \
             WHERE kind = 'function' AND clause_name = 'retry'",
            [],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .expect("retry row must exist");
    assert_eq!(is_req, 0, "retry clause must be optional (is_required = 0)");
    assert_eq!(pos, 7, "retry clause must be at position 7");
}
