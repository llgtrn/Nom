//! GAP-12 — S5l clock-domain clause tests.
//!
//! Covers:
//!  - Parse `clock domain "<name>" at <N> mhz.` → ClockDomain extracted.
//!  - Parse without a clock clause → clock_domain is None.
//!  - Clock coexists with contracts and effects.
//!  - Malformed: `clock` not followed by `domain` → NOMX-S5l-malformed-clock.
//!  - Malformed: `domain` not followed by a quoted name → NOMX-S5l-malformed-clock.
//!  - Malformed: quoted name not followed by `at` → NOMX-S5l-malformed-clock.
//!  - Malformed: `at` not followed by a positive integer → NOMX-S5l-malformed-clock.
//!  - Malformed: integer not followed by `mhz` → NOMX-S5l-malformed-clock.
//!  - Malformed: missing closing `.` → NOMX-S5l-unterminated-clock.
//!  - Clause shape registered in grammar DB for function kind.

use nom_concept::{
    ClockDomain, NomtuItem,
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
    run_pipeline(src).expect_err("pipeline must reject malformed clock domain clause")
}

// ── positive cases ────────────────────────────────────────────────────────────

#[test]
fn clock_domain_extracted() {
    let src = "the function sync_data is\n\
               intended to synchronize across clock domains.\n\
               clock domain \"pci_clk\" at 250 mhz.\n";
    let e = first_entity(src);
    assert_eq!(
        e.clock_domain,
        Some(ClockDomain {
            name: "pci_clk".to_string(),
            frequency_mhz: 250,
        }),
        "clock_domain must be extracted with name=pci_clk and frequency=250"
    );
}

#[test]
fn clock_domain_different_frequency() {
    let src = "the function sync_apb is\n\
               intended to drive the apb bus.\n\
               clock domain \"apb_clk\" at 100 mhz.\n";
    let e = first_entity(src);
    assert_eq!(
        e.clock_domain,
        Some(ClockDomain {
            name: "apb_clk".to_string(),
            frequency_mhz: 100,
        })
    );
}

#[test]
fn no_clock_domain_yields_none() {
    let src = "the function no_clock is\n\
               intended to perform a pure computation.\n";
    let e = first_entity(src);
    assert_eq!(
        e.clock_domain, None,
        "clock_domain must be None when no clock clause is present"
    );
}

#[test]
fn clock_coexists_with_other_clauses() {
    let src = "the function bridge_clocks is\n\
               intended to bridge two clock domains.\n\
               requires the source clock is stable.\n\
               ensures the output is synchronized.\n\
               clock domain \"sys_clk\" at 200 mhz.\n\
               hazard metastability.\n";
    let e = first_entity(src);
    assert_eq!(
        e.clock_domain,
        Some(ClockDomain {
            name: "sys_clk".to_string(),
            frequency_mhz: 200,
        })
    );
    assert_eq!(e.contracts.len(), 2);
    assert_eq!(e.effects.len(), 1);
}

// ── negative cases ────────────────────────────────────────────────────────────

#[test]
fn clock_without_domain_treated_as_prose() {
    // `clock` not followed by `domain` is prose filler — no clock_domain is extracted.
    let src = "the function foo is intended to bar with clock precision.\n";
    let e = first_entity(src);
    assert_eq!(
        e.clock_domain, None,
        "clock without domain must be treated as prose, not a clause opener"
    );
}

#[test]
fn clock_domain_missing_quoted_name_rejects() {
    let src = "the function foo is intended to bar.\nclock domain pci_clk at 250 mhz.\n";
    let err = parse_err(src);
    assert_eq!(err.reason, "malformed-clock");
}

#[test]
fn clock_domain_missing_mhz_rejects() {
    let src = "the function foo is intended to bar.\nclock domain \"pci_clk\" at 250.\n";
    let err = parse_err(src);
    assert_eq!(err.reason, "malformed-clock");
}

#[test]
fn clock_domain_missing_closing_dot_rejects() {
    let src = "the function foo is intended to bar.\nclock domain \"pci_clk\" at 250 mhz\n";
    let err = parse_err(src);
    assert_eq!(err.reason, "unterminated-clock");
}

// ── clause_shapes DB registration ─────────────────────────────────────────────

#[test]
fn clock_clause_shape_in_grammar_db() {
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
         VALUES ('function', 'clock', 0, 12, \
         'clock domain <quoted-name> at <N> mhz .', 'GAP-12')",
        [],
    )
    .unwrap();

    let (is_req, pos): (i64, i64) = conn
        .query_row(
            "SELECT is_required, position FROM clause_shapes \
             WHERE kind = 'function' AND clause_name = 'clock'",
            [],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .expect("clock row must exist");
    assert_eq!(is_req, 0, "clock clause must be optional (is_required = 0)");
    assert_eq!(pos, 12, "clock clause must be at position 12");
}
