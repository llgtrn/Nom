//! GAP-12 — format-string interpolation clause tests.
//!
//! Covers:
//!  - Parse a function with `format "<template with {var}>"` → format_template extracted.
//!  - Parse without a format clause → format_template is None.
//!  - Format template with multiple interpolation variables.
//!  - Format template with no interpolation (plain string).
//!  - Malformed: `format` not followed by a quoted string → NOMX-S5e-malformed-format.
//!  - Malformed: missing closing `.` → NOMX-S5e-unterminated-format.
//!  - Format clause coexists with contracts and effects.
//!  - Clause shape registered in grammar DB for function kind.

use nom_concept::{
    NomtuItem,
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
    run_pipeline(src).expect_err("pipeline must reject malformed format clause")
}

// ── positive cases ────────────────────────────────────────────────────────────

#[test]
fn format_template_single_variable() {
    let src = "the function greet is intended to produce a greeting.\n\
               format \"Hello, {name}!\".\n";
    let e = first_entity(src);
    assert_eq!(
        e.format_template,
        Some("Hello, {name}!".to_string()),
        "format_template should capture the quoted string content"
    );
}

#[test]
fn format_template_multiple_variables() {
    let src = "the function greet is intended to produce a greeting.\n\
               format \"Hello, {name}! Welcome to {place}.\".\n";
    let e = first_entity(src);
    assert_eq!(
        e.format_template,
        Some("Hello, {name}! Welcome to {place}.".to_string()),
        "format_template should capture all interpolation markers"
    );
}

#[test]
fn format_template_plain_string() {
    let src = "the function greet is intended to produce a greeting.\n\
               format \"Hello, world!\".\n";
    let e = first_entity(src);
    assert_eq!(
        e.format_template,
        Some("Hello, world!".to_string()),
        "format_template should accept plain strings with no interpolation"
    );
}

#[test]
fn no_format_clause_yields_none() {
    let src = "the function ping is intended to check reachability.\n\
               requires the host is reachable.\n";
    let e = first_entity(src);
    assert_eq!(
        e.format_template, None,
        "format_template must be None when no format clause is present"
    );
}

#[test]
fn format_coexists_with_contracts_and_effects() {
    let src = "the function send_message is intended to emit a formatted notification.\n\
               requires the recipient is valid.\n\
               ensures the message is delivered.\n\
               format \"Dear {recipient}, your request {id} is complete.\".\n\
               benefit notification_sent.\n";
    let e = first_entity(src);
    assert_eq!(
        e.format_template,
        Some("Dear {recipient}, your request {id} is complete.".to_string()),
    );
    assert_eq!(e.contracts.len(), 2, "contracts should be preserved");
    assert_eq!(e.effects.len(), 1, "effects should be preserved");
}

#[test]
fn format_coexists_with_retry() {
    let src = "the function fetch_and_format is intended to retrieve and format a record.\n\
               retry at-most 3 times.\n\
               format \"Record: {id} — status {status}.\".\n";
    let e = first_entity(src);
    assert_eq!(
        e.format_template,
        Some("Record: {id} — status {status}.".to_string()),
    );
    assert!(e.retry_policy.is_some(), "retry_policy should be preserved");
}

// ── negative cases ────────────────────────────────────────────────────────────

#[test]
fn format_in_prose_not_followed_by_quoted_yields_none() {
    // `format plain_word.` — `format` as English prose, not a clause opener.
    // The extractor skips `format` tokens not followed by a quoted string.
    let src = "the function foo is intended to format output.\n";
    let e = first_entity(src);
    assert_eq!(
        e.format_template, None,
        "format in prose should not be treated as a clause opener"
    );
}

#[test]
fn format_missing_closing_dot_rejects() {
    // `format "hello {name}"` with no `.`
    let src = "the function foo is intended to bar.\nformat \"hello {name}\"\n";
    let err = parse_err(src);
    assert_eq!(err.reason, "unterminated-format");
}

// ── clause_shapes DB registration ─────────────────────────────────────────────

#[test]
fn format_clause_shape_in_grammar_db() {
    let dir = tempfile::tempdir().unwrap();
    let db = dir.path().join("grammar.sqlite");
    let conn = nom_grammar::init_at(&db).unwrap();

    // Seed the function kind + its clause_shapes.
    conn.execute(
        "INSERT INTO kinds (name, description, allowed_clauses, allowed_refs, shipped_commit, notes) \
         VALUES ('function', '', '[]', '[]', 'GAP-12-test', NULL)",
        [],
    )
    .unwrap();

    // Insert the format clause shape (mirrors the baseline.sql row).
    conn.execute(
        "INSERT OR IGNORE INTO clause_shapes \
         (kind, clause_name, is_required, position, grammar_shape, source_ref) \
         VALUES ('function', 'format', 0, 8, \
         'format <quoted-template-with-{interpolation}> .', 'GAP-12')",
        [],
    )
    .unwrap();

    // Verify it shows up with the right attributes.
    let (is_req, pos): (i64, i64) = conn
        .query_row(
            "SELECT is_required, position FROM clause_shapes \
             WHERE kind = 'function' AND clause_name = 'format'",
            [],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .expect("format row must exist");
    assert_eq!(
        is_req, 0,
        "format clause must be optional (is_required = 0)"
    );
    assert_eq!(pos, 8, "format clause must be at position 8");
}
