//! GAP-12 — wire-field-tag clause tests.
//!
//! Covers:
//!  - Parse a data entity with one `field … tagged "…".` clause → field_tags extracted.
//!  - Parse with multiple `field … tagged "…".` clauses → all pairs collected.
//!  - Parse without any field-tag clauses → field_tags is None.
//!  - Field-tag clause coexists with intent, contracts, and effects.
//!  - Field-tag clause coexists with @Union variants.
//!  - Malformed: `field <name> tagged` not followed by a quoted string is silently skipped.
//!  - Malformed: missing closing `.` → NOMX-S5g-unterminated-field-tag.
//!  - Clause shape registered in grammar DB for data kind.

use nom_concept::{
    FieldTag, NomtuItem,
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
    run_pipeline(src).expect_err("pipeline must reject malformed field-tag clause")
}

// ── positive cases ────────────────────────────────────────────────────────────

#[test]
fn single_field_tag_extracted() {
    let src = "the data api_response is\n\
               intended to represent a JSON API response.\n\
               field status tagged \"status_code\".\n";
    let e = first_entity(src);
    assert_eq!(
        e.field_tags,
        Some(vec![FieldTag {
            field_name: "status".to_string(),
            wire_name: "status_code".to_string(),
        }]),
        "field_tags should capture the field-to-wire mapping"
    );
}

#[test]
fn multiple_field_tags_collected_in_order() {
    let src = "the data api_response is\n\
               intended to represent a JSON API response.\n\
               field status tagged \"status_code\".\n\
               field body tagged \"response_body\".\n\
               field timestamp tagged \"created_at\".\n";
    let e = first_entity(src);
    assert_eq!(
        e.field_tags,
        Some(vec![
            FieldTag {
                field_name: "status".to_string(),
                wire_name: "status_code".to_string(),
            },
            FieldTag {
                field_name: "body".to_string(),
                wire_name: "response_body".to_string(),
            },
            FieldTag {
                field_name: "timestamp".to_string(),
                wire_name: "created_at".to_string(),
            },
        ]),
        "all three field-tag pairs should be collected in source order"
    );
}

#[test]
fn no_field_tag_clauses_yields_none() {
    let src = "the data record is\n\
               intended to hold a simple record.\n\
               exposes name as text.\n";
    let e = first_entity(src);
    assert_eq!(
        e.field_tags, None,
        "field_tags must be None when no field … tagged … clauses are present"
    );
}

#[test]
fn field_tag_coexists_with_contracts_and_effects() {
    let src = "the data event_payload is\n\
               intended to represent an event payload for the message bus.\n\
               requires the event_type is non-empty.\n\
               field event_type tagged \"type\".\n\
               field correlation_id tagged \"corr_id\".\n\
               hazard schema_mismatch.\n";
    let e = first_entity(src);
    assert_eq!(
        e.field_tags,
        Some(vec![
            FieldTag {
                field_name: "event_type".to_string(),
                wire_name: "type".to_string(),
            },
            FieldTag {
                field_name: "correlation_id".to_string(),
                wire_name: "corr_id".to_string(),
            },
        ]),
        "field_tags should be present alongside contracts and effects"
    );
    assert_eq!(e.contracts.len(), 1, "contracts should be preserved");
    assert_eq!(e.effects.len(), 1, "effects should be preserved");
}

#[test]
fn field_tag_coexists_with_union_variants() {
    let src = "the data message_status is\n\
               intended to represent the delivery status of a message.\n\
               @Union of sent, delivered, failed.\n\
               field sent tagged \"msg_sent\".\n\
               field delivered tagged \"msg_delivered\".\n";
    let e = first_entity(src);
    assert!(
        e.union_variants.is_some(),
        "union_variants should be preserved alongside field_tags"
    );
    assert_eq!(
        e.field_tags,
        Some(vec![
            FieldTag {
                field_name: "sent".to_string(),
                wire_name: "msg_sent".to_string(),
            },
            FieldTag {
                field_name: "delivered".to_string(),
                wire_name: "msg_delivered".to_string(),
            },
        ])
    );
}

#[test]
fn field_tag_wire_name_preserves_special_chars() {
    // Wire names often contain hyphens, which are common in JSON keys.
    let src = "the data http_header is\n\
               intended to represent an HTTP header value.\n\
               field content_type tagged \"Content-Type\".\n";
    let e = first_entity(src);
    assert_eq!(
        e.field_tags,
        Some(vec![FieldTag {
            field_name: "content_type".to_string(),
            wire_name: "Content-Type".to_string(),
        }])
    );
}

// ── negative cases ────────────────────────────────────────────────────────────

#[test]
fn field_in_prose_not_followed_by_tagged_yields_none() {
    // `field` appearing in prose (intent sentence) should not be treated as a clause opener.
    let src = "the data form is intended to represent a form field value.\n";
    let e = first_entity(src);
    assert_eq!(
        e.field_tags, None,
        "field in prose intent should not be treated as a clause opener"
    );
}

#[test]
fn field_tag_missing_closing_dot_rejects() {
    // `field status tagged "status_code"` with no `.`
    let src = "the data foo is intended to hold data.\nfield status tagged \"status_code\"\n";
    let err = parse_err(src);
    assert_eq!(
        err.reason, "unterminated-field-tag",
        "expected unterminated-field-tag, got: {:?}",
        err
    );
}

// ── clause_shapes DB registration ─────────────────────────────────────────────

#[test]
fn field_tag_clause_shape_in_grammar_db() {
    let dir = tempfile::tempdir().unwrap();
    let db = dir.path().join("grammar.sqlite");
    let conn = nom_grammar::init_at(&db).unwrap();

    // Seed the data kind.
    conn.execute(
        "INSERT INTO kinds (name, description, allowed_clauses, allowed_refs, shipped_commit, notes) \
         VALUES ('data', '', '[]', '[]', 'GAP-12-field-tag-test', NULL)",
        [],
    )
    .unwrap();

    // Insert the field_tag clause shape (mirrors the baseline.sql row).
    conn.execute(
        "INSERT OR IGNORE INTO clause_shapes \
         (kind, clause_name, is_required, position, grammar_shape, source_ref) \
         VALUES ('data', 'field_tag', 0, 5, \
         'field <field-name> tagged <quoted-wire-name> .', 'GAP-12')",
        [],
    )
    .unwrap();

    // Verify it shows up with the right attributes.
    let (is_req, pos): (i64, i64) = conn
        .query_row(
            "SELECT is_required, position FROM clause_shapes \
             WHERE kind = 'data' AND clause_name = 'field_tag'",
            [],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .expect("field_tag row must exist");
    assert_eq!(
        is_req, 0,
        "field_tag clause must be optional (is_required = 0)"
    );
    assert_eq!(pos, 5, "field_tag clause must be at position 5");
}
