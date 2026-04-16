//! GAP-12 — `when` clause exhaustiveness checker tests.
//!
//! Covers:
//!  - Parse a function entity with `when <var> is <variant> then <result>.`
//!    clauses → `when_clauses` extracted.
//!  - Entity without any `when` clauses → `when_clauses` is `None`.
//!  - Multiple `when` arms for different variables are all captured.
//!  - `check_exhaustiveness` returns empty when all variants covered.
//!  - `check_exhaustiveness` returns one warning per missing variant.
//!  - `check_exhaustiveness` on empty `when_clauses` warns for every variant.
//!  - Partial coverage warns only for the missing variants.
//!  - Warning code and message content are correct.

use nom_concept::{
    NomtuItem, UnionVariants, WhenClause,
    exhaustiveness::check_exhaustiveness,
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

// ── extraction tests ─────────────────────────────────────────────────────────

#[test]
fn when_clauses_extracted_from_function() {
    let src = "the function describe_payment is\n\
               intended to describe a payment method.\n\
               given method of payment_method, returns text.\n\
               when method is credit_card then \"Credit Card\".\n\
               when method is debit_card then \"Debit Card\".\n\
               when method is bank_transfer then \"Bank Transfer\".\n";
    let e = first_entity(src);
    let clauses = e.when_clauses.expect("when_clauses must be Some");
    assert_eq!(clauses.len(), 3, "all three when arms must be captured");
    assert_eq!(clauses[0].variable, "method");
    assert_eq!(clauses[0].variant, "credit_card");
    assert_eq!(clauses[0].result, "\"Credit Card\"");
    assert_eq!(clauses[1].variant, "debit_card");
    assert_eq!(clauses[2].variant, "bank_transfer");
}

#[test]
fn no_when_clauses_yields_none() {
    let src = "the function simple_add is\n\
               intended to add two numbers.\n\
               given a of number, b of number, returns number.\n";
    let e = first_entity(src);
    assert_eq!(
        e.when_clauses, None,
        "when_clauses must be None when no when clauses present"
    );
}

#[test]
fn single_when_clause_extracted() {
    let src = "the function check_status is\n\
               intended to report a status.\n\
               given s of status_type, returns text.\n\
               when s is active then \"running\".\n";
    let e = first_entity(src);
    let clauses = e.when_clauses.expect("when_clauses must be Some");
    assert_eq!(clauses.len(), 1);
    assert_eq!(
        clauses[0],
        WhenClause {
            variable: "s".to_string(),
            variant: "active".to_string(),
            result: "\"running\"".to_string(),
        }
    );
}

// ── exhaustiveness checker tests ─────────────────────────────────────────────

#[test]
fn exhaustive_match_returns_no_warnings() {
    let whens = vec![
        WhenClause {
            variable: "m".to_string(),
            variant: "credit_card".to_string(),
            result: "\"Credit Card\"".to_string(),
        },
        WhenClause {
            variable: "m".to_string(),
            variant: "debit_card".to_string(),
            result: "\"Debit Card\"".to_string(),
        },
        WhenClause {
            variable: "m".to_string(),
            variant: "bank_transfer".to_string(),
            result: "\"Bank Transfer\"".to_string(),
        },
    ];
    let union = UnionVariants {
        variants: vec![
            "credit_card".to_string(),
            "debit_card".to_string(),
            "bank_transfer".to_string(),
        ],
    };
    let warnings = check_exhaustiveness(&whens, &union);
    assert!(warnings.is_empty(), "no warnings when all variants covered");
}

#[test]
fn missing_one_variant_produces_one_warning() {
    let whens = vec![
        WhenClause {
            variable: "m".to_string(),
            variant: "credit_card".to_string(),
            result: "\"Credit Card\"".to_string(),
        },
        WhenClause {
            variable: "m".to_string(),
            variant: "debit_card".to_string(),
            result: "\"Debit Card\"".to_string(),
        },
    ];
    let union = UnionVariants {
        variants: vec![
            "credit_card".to_string(),
            "debit_card".to_string(),
            "cryptocurrency".to_string(),
        ],
    };
    let warnings = check_exhaustiveness(&whens, &union);
    assert_eq!(warnings.len(), 1, "exactly one warning for missing variant");
    assert_eq!(warnings[0].missing_variant, "cryptocurrency");
}

#[test]
fn missing_multiple_variants_produces_multiple_warnings() {
    let whens = vec![WhenClause {
        variable: "method".to_string(),
        variant: "credit_card".to_string(),
        result: "\"Credit Card\"".to_string(),
    }];
    let union = UnionVariants {
        variants: vec![
            "credit_card".to_string(),
            "debit_card".to_string(),
            "bank_transfer".to_string(),
            "cryptocurrency".to_string(),
        ],
    };
    let warnings = check_exhaustiveness(&whens, &union);
    assert_eq!(
        warnings.len(),
        3,
        "three warnings for three missing variants"
    );
    let missing: Vec<&str> = warnings
        .iter()
        .map(|w| w.missing_variant.as_str())
        .collect();
    assert!(missing.contains(&"debit_card"));
    assert!(missing.contains(&"bank_transfer"));
    assert!(missing.contains(&"cryptocurrency"));
}

#[test]
fn empty_when_clauses_warns_for_every_variant() {
    let union = UnionVariants {
        variants: vec!["a".to_string(), "b".to_string(), "c".to_string()],
    };
    let warnings = check_exhaustiveness(&[], &union);
    assert_eq!(
        warnings.len(),
        3,
        "all three variants warned when no when clauses"
    );
}

#[test]
fn warning_has_correct_code_and_message() {
    let whens: Vec<WhenClause> = vec![];
    let union = UnionVariants {
        variants: vec!["missing_variant".to_string()],
    };
    let warnings = check_exhaustiveness(&whens, &union);
    assert_eq!(warnings.len(), 1);
    let w = &warnings[0];
    assert_eq!(w.code, "NOMX-GAP12-nonexhaustive");
    assert!(
        w.message.contains("missing_variant"),
        "message must name the missing variant"
    );
}

#[test]
fn extra_when_arms_not_in_union_are_ignored() {
    // When the when clause mentions a variant not in the union,
    // the checker only reports on union.variants; extra arms are harmless.
    let whens = vec![
        WhenClause {
            variable: "x".to_string(),
            variant: "known".to_string(),
            result: "ok".to_string(),
        },
        WhenClause {
            variable: "x".to_string(),
            variant: "unknown_extra".to_string(),
            result: "ok".to_string(),
        },
    ];
    let union = UnionVariants {
        variants: vec!["known".to_string()],
    };
    let warnings = check_exhaustiveness(&whens, &union);
    assert!(
        warnings.is_empty(),
        "no warnings when all union variants are covered (extra arms in when are fine)"
    );
}
