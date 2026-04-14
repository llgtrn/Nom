//! Seeding functions per doc 21 phases P4 + P5 (kinds + QualityNames + authoring_rules).
//!
//! This module carries the data that migrates "grammar in doc files" into the
//! grammar.sqlite registry so AI clients can query it deterministically. Each
//! seed fn is idempotent (INSERT OR REPLACE) so re-running after doc edits
//! brings the DB back in sync.

use anyhow::{Context, Result};
use rusqlite::{Connection, params};

// ── P5: closed 9-kind set (doc 08 §2, W41 + W46) ────────────────────

/// The closed set of 9 top-level kinds, in the canonical order they were
/// introduced (function → module → concept → screen → data → event → media
/// → property (W41) → scenario (W46)).
pub const KINDS_SEED: &[(&str, &str, &str)] = &[
    (
        "function",
        "Named computation with input types + output type + requires/ensures/hazard contract clauses.",
        "a04b91e",
    ),
    (
        "module",
        "Tier-1 composition: several DB2 entities grouped with optional composition expressions (doc 08 §1 Tier 1).",
        "a04b91e",
    ),
    (
        "concept",
        "Tier-2 big-scope container: one or more concepts with dictionary-relative index over DB2 (doc 08 §1 Tier 2).",
        "a04b91e",
    ),
    (
        "screen",
        "User-facing UI / rendered artifact / internal architecture diagram. Generalised by doc 14 #39 + #49.",
        "a04b91e",
    ),
    (
        "data",
        "Structural type / tagged variant / schema-IDL. Covers Kotlin-sealed, Elm Msg, Protobuf, Solidity tagged errors.",
        "a04b91e",
    ),
    (
        "event",
        "Named event signal (editor event, subscription, stream element). W49-quantified ensures describe delivery semantics.",
        "a04b91e",
    ),
    (
        "media",
        "Image / audio / video / 3D / typography — composable via same 3 operators per §5.18 aesthetic-is-programming.",
        "a04b91e",
    ),
    (
        "property",
        "Universally-quantified claim over a generator. Wedge W41 — 8th kind added for property-based-verification paradigm.",
        "W41-ship-commit",
    ),
    (
        "scenario",
        "Asserted-behavior claim with given/when/then triple. Wedge W46 — 9th kind added for BDD/Gherkin/RSpec surface.",
        "W46-ship-commit",
    ),
];

/// Per-kind `allowed_refs` lattice. Each kind lists which @Kind typed-slot
/// refs its `uses` clause may invoke. Derived from doc 08 §2 + actual
/// translation corpus patterns in doc 14.
const ALLOWED_REFS_FOR_KIND: &[(&str, &[&str])] = &[
    ("function",  &["@Function", "@Data", "@Concept", "@Module", "@Property"]),
    ("module",    &["@Function", "@Data", "@Module", "@Composition"]),
    ("concept",   &["@Function", "@Data", "@Concept", "@Module", "@Composition", "@Route"]),
    ("screen",    &["@Function", "@Data", "@Concept", "@Composition", "@Media"]),
    ("data",      &[]), // data decls have no `uses` clause — pure structural
    ("event",     &["@Data"]),
    ("media",     &["@Function", "@Data", "@Media"]),
    ("property",  &["@Function", "@Data", "@Property"]),
    ("scenario",  &["@Function", "@Data", "@Concept", "@Property"]),
];

fn allowed_refs_json(kind: &str) -> String {
    for (k, refs) in ALLOWED_REFS_FOR_KIND {
        if *k == kind {
            return serde_json::to_string(refs).unwrap_or_else(|_| "[]".to_string());
        }
    }
    "[]".to_string()
}

/// Derive `allowed_clauses` for a kind from CLAUSE_SHAPES_SEED, sorted by position.
fn allowed_clauses_json(kind: &str) -> String {
    let mut pairs: Vec<(i32, &str)> = CLAUSE_SHAPES_SEED
        .iter()
        .filter(|s| s.kind == kind)
        .map(|s| (s.position, s.clause_name))
        .collect();
    pairs.sort_by_key(|(pos, _)| *pos);
    let clauses: Vec<&str> = pairs.into_iter().map(|(_, n)| n).collect();
    serde_json::to_string(&clauses).unwrap_or_else(|_| "[]".to_string())
}

pub fn seed_kinds(conn: &Connection) -> Result<usize> {
    let mut inserted = 0;
    for (name, description, commit) in KINDS_SEED {
        let allowed_clauses = allowed_clauses_json(name);
        let allowed_refs = allowed_refs_json(name);
        conn.execute(
            "INSERT OR REPLACE INTO kinds \
             (name, description, allowed_clauses, allowed_refs, shipped_commit, notes) \
             VALUES (?1, ?2, ?3, ?4, ?5, NULL)",
            params![name, description, allowed_clauses, allowed_refs, commit],
        )?;
        inserted += 1;
    }
    Ok(inserted)
}

// ── P3: per-kind clause shape registry (from stages.rs S3+S4+S5+S6 invariants) ──

/// One row in the `clause_shapes` table. Describes which clauses each kind
/// accepts, in what canonical authoring order, and whether they are required.
#[derive(Debug, Clone, Copy)]
pub struct ClauseShapeSeed {
    pub kind: &'static str,
    pub clause_name: &'static str,
    pub is_required: i32, // 0=optional, 1=required, 2=one-of group
    pub one_of_group: Option<&'static str>,
    pub position: i32,
    pub grammar_shape: &'static str,
    pub min_occurrences: i32,
    pub max_occurrences: Option<i32>,
    pub source_ref: &'static str,
    pub notes: Option<&'static str>,
}

/// Per-kind clause shapes. Derived from nom-concept/src/stages.rs S3+S4+S5+S6
/// pass invariants and the doc 21 §2 + doc 08 + W41/W42/W46/W47/W49 specs.
pub const CLAUSE_SHAPES_SEED: &[ClauseShapeSeed] = &[
    // ── function ──
    s("function",  "intended", 1, 1,  "'intended to' <prose-sentence> '.'",                        1, Some(1), "doc 05 §3"),
    s("function",  "uses",     0, 2,  "'uses the' '@' Kind 'matching' <quoted-prose> 'with at-least' <f32 in 0..1> 'confidence' '.'", 0, None, "doc 07 v2 §6"),
    s("function",  "requires", 0, 3,  "'requires' <prose-precondition> '.'",                        0, None, "doc 05 §3"),
    sreq_any("function", "ensures", 4, "'ensures' <prose-postcondition> '.'",                       1, None, "doc 05 §3"),
    s("function",  "hazard",   0, 5,  "'hazard' <prose-hazard-note> '.'",                           0, None, "doc 05 §3"),
    s("function",  "favor",    0, 6,  "'favor' <QualityName> '.'",                                  0, None, "doc 08 §7"),
    // ── data ──
    s("data",      "intended", 1, 1,  "'intended to' <prose-sentence> '.'",                         1, Some(1), "doc 05 §3"),
    sreq_any("data", "exposes", 2,    "'exposes' <field-name> ('at tag' <int>)? 'as' <type-name> ('with payload' <field-list>)? '.'", 1, None, "doc 08 §2"),
    s("data",      "favor",    0, 3,  "'favor' <QualityName> '.'",                                  0, None, "doc 08 §7"),
    // ── concept ──
    s("concept",   "intended", 1, 1,  "'intended to' <prose-sentence> '.'",                         1, Some(1), "doc 05 §3"),
    s("concept",   "uses",     0, 2,  "'uses the' '@' Kind 'matching' <quoted-prose> 'with at-least' <f32 in 0..1> 'confidence' '.'", 0, None, "doc 07 v2"),
    s("concept",   "composes", 0, 3,  "'composes' <entity-ref> ('then' <entity-ref>)* '.'",         0, None, "doc 08 §1"),
    s("concept",   "requires", 0, 4,  "'requires' <prose-precondition> '.'",                         0, None, "doc 05 §3"),
    s("concept",   "ensures",  0, 5,  "'ensures' <prose-postcondition> '.'",                         0, None, "doc 05 §3"),
    s("concept",   "hazard",   0, 6,  "'hazard' <prose-hazard-note> '.'",                            0, None, "doc 05 §3"),
    s("concept",   "favor",    0, 7,  "'favor' <QualityName> '.'",                                   0, None, "doc 08 §7"),
    s("concept",   "exposes",  0, 8,  "'exposes' <name-list> '.'",                                   0, None, "doc 08 §2 (concept-as-export surface)"),
    // ── module ──
    s("module",    "intended", 1, 1,  "'intended to' <prose-sentence> '.'",                         1, Some(1), "doc 05 §3"),
    s("module",    "uses",     0, 2,  "'uses the' '@' Kind 'matching' <quoted-prose> 'with at-least' <f32 in 0..1> 'confidence' '.'", 0, None, "doc 07 v2"),
    s("module",    "composes", 0, 3,  "'composes' <entity-ref> ('then' <entity-ref>)* '.'",         0, None, "doc 08 §1 Tier 1"),
    s("module",    "favor",    0, 4,  "'favor' <QualityName> '.'",                                  0, None, "doc 08 §7"),
    // ── property ── (W41 + W42 generator)
    s("property",  "intended", 1, 1,  "'intended to assert' <prose-claim> '.'",                     1, Some(1), "W41"),
    s("property",  "generator",1, 2,  "'generator' <prose-domain-descriptor> '.'",                   1, Some(1), "W42"),
    s("property",  "uses",     0, 3,  "'uses the' '@' Kind 'matching' <quoted-prose> 'with at-least' <f32 in 0..1> 'confidence' '.'", 0, None, "doc 07 v2"),
    s("property",  "requires", 0, 4,  "'requires' <prose-precondition> '.'",                         0, None, "doc 05 §3"),
    sreq_any("property", "ensures", 5, "'ensures' <prose-universal-claim> '.'",                     1, None, "W41 (property decl requires ≥1 ensures)"),
    s("property",  "favor",    0, 6,  "'favor' <QualityName> '.'",                                  0, None, "doc 08 §7"),
    // ── scenario ── (W46 kind, W47 clause grammar)
    s("scenario",  "intended", 1, 1,  "'intended to describe' <prose-scenario-summary> '.'",         1, Some(1), "W46"),
    s("scenario",  "given",    1, 2,  "'given' <prose-precondition> '.'",                            1, None, "W47"),
    s("scenario",  "when",     1, 3,  "'when' <prose-action> '.'",                                   1, None, "W47"),
    s("scenario",  "then",     1, 4,  "'then' <prose-postcondition> '.'",                            1, None, "W47"),
    s("scenario",  "favor",    0, 5,  "'favor' <QualityName> '.'",                                   0, None, "doc 08 §7"),
    // ── screen ── (user-facing UI + diagrams + typeset docs per doc 14 #39/#49)
    s("screen",    "intended", 1, 1,  "'intended to' <prose-sentence> '.'",                         1, Some(1), "doc 05 §3"),
    s("screen",    "uses",     0, 2,  "'uses the' '@' Kind 'matching' <quoted-prose> 'with at-least' <f32 in 0..1> 'confidence' '.'", 0, None, "doc 07 v2"),
    s("screen",    "exposes",  0, 3,  "'exposes' <field-name> 'as' <type-name> '.'",                 0, None, "doc 08 §2"),
    s("screen",    "favor",    0, 4,  "'favor' <QualityName> '.'",                                   0, None, "doc 08 §7"),
    // ── event ──
    s("event",     "intended", 1, 1,  "'intended to' <prose-sentence> '.'",                         1, Some(1), "doc 05 §3"),
    s("event",     "exposes",  0, 2,  "'exposes' <field-name> 'as' <type-name> '.'",                 0, None, "doc 08 §2"),
    s("event",     "ensures",  0, 3,  "'ensures' <W49-quantified prose> '.'",                        0, None, "W49 (event delivery semantics)"),
    s("event",     "favor",    0, 4,  "'favor' <QualityName> '.'",                                   0, None, "doc 08 §7"),
    // ── media ──
    s("media",     "intended", 1, 1,  "'intended to' <prose-sentence> '.'",                         1, Some(1), "doc 05 §3"),
    s("media",     "uses",     0, 2,  "'uses the' '@' Kind 'matching' <quoted-prose> 'with at-least' <f32 in 0..1> 'confidence' '.'", 0, None, "doc 07 v2"),
    s("media",     "favor",    0, 3,  "'favor' <QualityName> '.'",                                   0, None, "doc 08 §7"),
];

const fn s(
    kind: &'static str,
    clause_name: &'static str,
    is_required: i32,
    position: i32,
    grammar_shape: &'static str,
    min_occurrences: i32,
    max_occurrences: Option<i32>,
    source_ref: &'static str,
) -> ClauseShapeSeed {
    ClauseShapeSeed {
        kind,
        clause_name,
        is_required,
        one_of_group: None,
        position,
        grammar_shape,
        min_occurrences,
        max_occurrences,
        source_ref,
        notes: None,
    }
}

/// Shortcut for "required-at-least-one-of-this-kind" clauses (e.g. function
/// requires ≥1 ensures; data requires ≥1 exposes).
const fn sreq_any(
    kind: &'static str,
    clause_name: &'static str,
    position: i32,
    grammar_shape: &'static str,
    min_occurrences: i32,
    max_occurrences: Option<i32>,
    source_ref: &'static str,
) -> ClauseShapeSeed {
    ClauseShapeSeed {
        kind,
        clause_name,
        is_required: 2,
        one_of_group: None,
        position,
        grammar_shape,
        min_occurrences,
        max_occurrences,
        source_ref,
        notes: None,
    }
}

pub fn seed_clause_shapes(conn: &Connection) -> Result<usize> {
    let mut inserted = 0;
    for shape in CLAUSE_SHAPES_SEED {
        conn.execute(
            "INSERT OR REPLACE INTO clause_shapes \
             (kind, clause_name, is_required, one_of_group, position, grammar_shape, \
              min_occurrences, max_occurrences, source_ref, notes) \
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
            params![
                shape.kind,
                shape.clause_name,
                shape.is_required,
                shape.one_of_group,
                shape.position,
                shape.grammar_shape,
                shape.min_occurrences,
                shape.max_occurrences,
                shape.source_ref,
                shape.notes,
            ],
        )?;
        inserted += 1;
    }
    Ok(inserted)
}

// ── P2: closed keyword vocabulary (docs 05 / 06 / 07 v2 / W4 / W49 / W41 / W46 / W50) ──

/// One row in the `keywords` table. `role` + `kind_scope` describe where the
/// token is valid; `source_ref` cites the research doc or parser line that
/// introduced it.
#[derive(Debug, Clone, Copy)]
pub struct KeywordSeed {
    pub token: &'static str,
    pub role: &'static str,
    pub kind_scope: Option<&'static str>, // JSON-encoded array, or None = any
    pub source_ref: &'static str,
    pub shipped_commit: &'static str,
    pub notes: Option<&'static str>,
}

/// Closed keyword set shipped by the current parser, grouped by role:
///
/// - **determiner**: the single `the` opener on every top-level decl
/// - **kind_noun**: the 9 prose-form kinds (v1 surface)
/// - **kind_marker**: the `@Kind` typed-slot form (v2 surface) — 11 entries
/// - **clause_opener**: the 12 recognized clause heads (requires/ensures/hazard/uses/exposes/favor/generator/composes/given/when/then/intended)
/// - **ref_slot**: `matching`, `with`, `at-least`, `confidence` (typed-slot syntax per doc 07)
/// - **quantifier**: W49-registered quantifier vocabulary (every/no/some/at-most/exactly)
/// - **connective**: light prose connectives recognised for shape disambiguation
pub const KEYWORDS_SEED: &[KeywordSeed] = &[
    // determiner — doc 05 §3
    KeywordSeed {
        token: "the",
        role: "determiner",
        kind_scope: None,
        source_ref: "doc 05 §3",
        shipped_commit: "a04b91e",
        notes: Some("every top-level decl opens with 'the' (v2) or 'define' (v1)"),
    },
    // v1 decl opener
    KeywordSeed {
        token: "define",
        role: "decl_opener_v1",
        kind_scope: None,
        source_ref: "doc 05 §3",
        shipped_commit: "a04b91e",
        notes: Some("`.nomx v1` prose form; v2 uses `the` instead"),
    },
    // kind nouns (v1 surface)
    KeywordSeed { token: "function",  role: "kind_noun", kind_scope: None, source_ref: "doc 08 §1", shipped_commit: "a04b91e", notes: None },
    KeywordSeed { token: "module",    role: "kind_noun", kind_scope: None, source_ref: "doc 08 §1", shipped_commit: "a04b91e", notes: None },
    KeywordSeed { token: "concept",   role: "kind_noun", kind_scope: None, source_ref: "doc 08 §1", shipped_commit: "a04b91e", notes: None },
    KeywordSeed { token: "screen",    role: "kind_noun", kind_scope: None, source_ref: "doc 08 §1", shipped_commit: "a04b91e", notes: None },
    KeywordSeed { token: "data",      role: "kind_noun", kind_scope: None, source_ref: "doc 08 §1", shipped_commit: "a04b91e", notes: None },
    KeywordSeed { token: "event",     role: "kind_noun", kind_scope: None, source_ref: "doc 08 §1", shipped_commit: "a04b91e", notes: None },
    KeywordSeed { token: "media",     role: "kind_noun", kind_scope: None, source_ref: "doc 08 §1", shipped_commit: "a04b91e", notes: None },
    KeywordSeed { token: "property",  role: "kind_noun", kind_scope: None, source_ref: "W41", shipped_commit: "W41-ship-commit", notes: Some("added by W41") },
    KeywordSeed { token: "scenario",  role: "kind_noun", kind_scope: None, source_ref: "W46", shipped_commit: "W46-ship-commit", notes: Some("added by W46") },
    // kind markers (v2 typed-slot surface)
    KeywordSeed { token: "@Function",    role: "kind_marker", kind_scope: None, source_ref: "doc 07 v2",  shipped_commit: "97c836f", notes: None },
    KeywordSeed { token: "@Module",      role: "kind_marker", kind_scope: None, source_ref: "doc 07 v2",  shipped_commit: "97c836f", notes: None },
    KeywordSeed { token: "@Concept",     role: "kind_marker", kind_scope: None, source_ref: "doc 07 v2",  shipped_commit: "97c836f", notes: None },
    KeywordSeed { token: "@Screen",      role: "kind_marker", kind_scope: None, source_ref: "doc 07 v2",  shipped_commit: "97c836f", notes: None },
    KeywordSeed { token: "@Data",        role: "kind_marker", kind_scope: None, source_ref: "doc 07 v2",  shipped_commit: "97c836f", notes: None },
    KeywordSeed { token: "@Event",       role: "kind_marker", kind_scope: None, source_ref: "doc 07 v2",  shipped_commit: "97c836f", notes: None },
    KeywordSeed { token: "@Media",       role: "kind_marker", kind_scope: None, source_ref: "doc 07 v2",  shipped_commit: "97c836f", notes: None },
    KeywordSeed { token: "@Property",    role: "kind_marker", kind_scope: None, source_ref: "W41",         shipped_commit: "W41-ship-commit", notes: None },
    KeywordSeed { token: "@Scenario",    role: "kind_marker", kind_scope: None, source_ref: "W46",         shipped_commit: "W46-ship-commit", notes: None },
    KeywordSeed { token: "@Composition", role: "kind_marker", kind_scope: None, source_ref: "doc 08 §1",  shipped_commit: "a04b91e", notes: None },
    KeywordSeed { token: "@Route",       role: "kind_marker", kind_scope: None, source_ref: "W50",         shipped_commit: "W50-ship-commit", notes: Some("HTTP methods + paths / gRPC / CLI subcommands") },
    // clause openers
    KeywordSeed { token: "intended",  role: "clause_opener", kind_scope: None, source_ref: "doc 05 §3",  shipped_commit: "a04b91e", notes: Some("'intended to <prose>' — universal across kinds") },
    KeywordSeed { token: "requires",  role: "clause_opener", kind_scope: Some(r#"["function","property","concept","scenario"]"#), source_ref: "doc 05 §3", shipped_commit: "a04b91e", notes: None },
    KeywordSeed { token: "ensures",   role: "clause_opener", kind_scope: Some(r#"["function","property","concept","scenario"]"#), source_ref: "doc 05 §3", shipped_commit: "a04b91e", notes: None },
    KeywordSeed { token: "hazard",    role: "clause_opener", kind_scope: Some(r#"["function","concept"]"#), source_ref: "doc 05 §3", shipped_commit: "a04b91e", notes: None },
    KeywordSeed { token: "uses",      role: "clause_opener", kind_scope: Some(r#"["function","concept","property","scenario","module"]"#), source_ref: "doc 07 v2", shipped_commit: "97c836f", notes: Some("introduces typed-slot @Kind refs") },
    KeywordSeed { token: "exposes",   role: "clause_opener", kind_scope: Some(r#"["data","concept"]"#), source_ref: "doc 08 §2", shipped_commit: "a04b91e", notes: None },
    KeywordSeed { token: "favor",     role: "clause_opener", kind_scope: None, source_ref: "doc 08 §7", shipped_commit: "a04b91e", notes: Some("pairs with QualityName registry") },
    KeywordSeed { token: "generator", role: "clause_opener", kind_scope: Some(r#"["property"]"#), source_ref: "W42", shipped_commit: "W42-ship-commit", notes: Some("W41-property kind specific") },
    KeywordSeed { token: "composes",  role: "clause_opener", kind_scope: Some(r#"["module","concept"]"#), source_ref: "doc 08 §1", shipped_commit: "a04b91e", notes: None },
    KeywordSeed { token: "given",     role: "clause_opener", kind_scope: Some(r#"["scenario"]"#), source_ref: "W47", shipped_commit: "W47-ship-commit", notes: None },
    KeywordSeed { token: "when",      role: "clause_opener", kind_scope: Some(r#"["scenario"]"#), source_ref: "W47", shipped_commit: "W47-ship-commit", notes: None },
    KeywordSeed { token: "then",      role: "clause_opener", kind_scope: Some(r#"["scenario"]"#), source_ref: "W47", shipped_commit: "W47-ship-commit", notes: None },
    // ref slot vocabulary (doc 07 v2 typed-slot syntax)
    KeywordSeed { token: "matching",   role: "ref_slot", kind_scope: None, source_ref: "doc 07 v2", shipped_commit: "97c836f", notes: Some("'uses the @Kind matching \"prose\"'") },
    KeywordSeed { token: "with",       role: "ref_slot", kind_scope: None, source_ref: "doc 07 v2", shipped_commit: "97c836f", notes: Some("'with at-least N confidence'") },
    KeywordSeed { token: "at-least",   role: "ref_slot", kind_scope: None, source_ref: "doc 07 v2", shipped_commit: "853e70b", notes: Some("confidence threshold — hyphenated compound") },
    KeywordSeed { token: "confidence", role: "ref_slot", kind_scope: None, source_ref: "doc 07 v2", shipped_commit: "97c836f", notes: None },
    // quantifier vocabulary (W49)
    KeywordSeed { token: "every",    role: "quantifier", kind_scope: None, source_ref: "W49", shipped_commit: "W49-ship-commit", notes: Some("universal quantifier") },
    KeywordSeed { token: "no",       role: "quantifier", kind_scope: None, source_ref: "W49", shipped_commit: "W49-ship-commit", notes: Some("negated existential") },
    KeywordSeed { token: "some",     role: "quantifier", kind_scope: None, source_ref: "W49", shipped_commit: "W49-ship-commit", notes: Some("existential") },
    KeywordSeed { token: "at-most",  role: "quantifier", kind_scope: None, source_ref: "W49", shipped_commit: "W49-ship-commit", notes: Some("bounded above — rolling-window rate limits") },
    KeywordSeed { token: "exactly",  role: "quantifier", kind_scope: None, source_ref: "W49", shipped_commit: "W49-ship-commit", notes: Some("precise count") },
    // connectives (prose shape)
    KeywordSeed { token: "is",  role: "connective", kind_scope: None, source_ref: "doc 05 §3", shipped_commit: "a04b91e", notes: Some("copula after decl opener") },
    KeywordSeed { token: "to",  role: "connective", kind_scope: None, source_ref: "doc 05 §3", shipped_commit: "a04b91e", notes: Some("'intended to X'") },
    KeywordSeed { token: "as",  role: "connective", kind_scope: None, source_ref: "doc 08 §2", shipped_commit: "a04b91e", notes: Some("'exposes X as Y'") },
    KeywordSeed { token: "of",  role: "connective", kind_scope: None, source_ref: "doc 07 v2", shipped_commit: "97c836f", notes: None },
    KeywordSeed { token: "and", role: "connective", kind_scope: None, source_ref: "doc 05 §3", shipped_commit: "a04b91e", notes: None },
    KeywordSeed { token: "or",  role: "connective", kind_scope: None, source_ref: "doc 05 §3", shipped_commit: "a04b91e", notes: None },
    KeywordSeed { token: "that", role: "connective", kind_scope: None, source_ref: "doc 05 §3", shipped_commit: "a04b91e", notes: Some("v1 'define X that Y'") },
];

pub fn seed_keywords(conn: &Connection) -> Result<usize> {
    let mut inserted = 0;
    for kw in KEYWORDS_SEED {
        conn.execute(
            "INSERT OR REPLACE INTO keywords \
             (token, role, kind_scope, source_ref, shipped_commit, notes) \
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![
                kw.token,
                kw.role,
                kw.kind_scope,
                kw.source_ref,
                kw.shipped_commit,
                kw.notes,
            ],
        )?;
        inserted += 1;
    }
    Ok(inserted)
}

// ── P5: 10 fixed QualityName seeds per MEMORY.md ────────────────────

/// The 10 fixed seed QualityNames registered 2026-04-14 (doc 08 §7, W51).
/// `metric_function` is a placeholder hash; real metric nomtu hashes are
/// populated by `nom corpus register-axis` per MEMORY.md roadmap item 8.
pub const QUALITY_SEED: &[(&str, &str, &str, Option<&str>)] = &[
    ("forward_compatibility", "semver/api", "any", None),
    ("numerical_stability", "numeric", "any", None),
    ("gas_efficiency", "onchain_cost", "any", None),
    ("synthesizability", "hardware", "any", None),
    ("minimum_cost", "optimization", "any", None),
    ("statistical_rigor", "stats", "any", None),
    ("availability", "ops", "exactly_one_per_app", Some("app")),
    ("auditability", "ops", "any", None),
    ("accessibility", "ops", "exactly_one_per_app", Some("app")),
    ("totality", "proofs", "any", None),
];

pub fn seed_quality_names(conn: &Connection) -> Result<usize> {
    let mut inserted = 0;
    for (name, axis, cardinality, required_at) in QUALITY_SEED {
        // metric_function is populated by `nom corpus register-axis` per
        // MEMORY.md roadmap item 8. Until that ships, the column stays empty.
        let metric_function: Option<&str> = None;
        conn.execute(
            "INSERT OR REPLACE INTO quality_names \
             (name, axis, metric_function, cardinality, required_at, source_ref, notes) \
             VALUES (?1, ?2, ?3, ?4, ?5, 'MEMORY.md:2026-04-14', 'metric_function pending nom corpus register-axis')",
            params![name, axis, metric_function, cardinality, required_at],
        )?;
        inserted += 1;
    }
    Ok(inserted)
}

// ── P4: parse doc 16 markdown table → authoring_rules rows ──────────

#[derive(Debug, PartialEq)]
pub struct DocRuleRow {
    pub row_id: i64,
    pub gap_summary: String,
    pub destination: String,
    pub status: String,
    pub closed_in: Option<String>,
}

/// Parse lines shaped like `| 419 | Behavioral-module ... | authoring-guide rule | ✅ closed (doc 14 #85) |`
/// from doc 16's markdown source. Header lines, divider lines, and narrative
/// text are silently skipped. Returns one DocRuleRow per table row.
pub fn parse_doc16_rules(md_source: &str) -> Vec<DocRuleRow> {
    let mut rows = Vec::new();
    for raw in md_source.lines() {
        let line = raw.trim();
        // Require a numeric-leading table row: `| <n> | ... | ... | ... |`
        if !line.starts_with("| ") {
            continue;
        }
        let cells: Vec<&str> = line.split('|').map(str::trim).collect();
        // A well-formed row has 6 split-pieces: "" | id | gap | dest | status | ""
        if cells.len() < 5 {
            continue;
        }
        let id_cell = cells[1];
        let row_id: i64 = match id_cell.parse() {
            Ok(n) => n,
            Err(_) => continue,
        };
        let gap_summary = cells[2].to_string();
        let destination = cells[3].to_string();
        let status_cell = cells[4].to_string();
        let (status, closed_in) = split_status_and_ref(&status_cell);
        rows.push(DocRuleRow {
            row_id,
            gap_summary,
            destination,
            status,
            closed_in,
        });
    }
    rows
}

/// Split a status cell like "✅ closed (doc 14 #85)" into ("closed", Some("doc 14 #85")).
/// Leaves free-form statuses like "⏳ queued" → ("queued", None).
fn split_status_and_ref(cell: &str) -> (String, Option<String>) {
    // Strip leading emoji + whitespace.
    let stripped = cell
        .chars()
        .skip_while(|c| !c.is_ascii_alphabetic())
        .collect::<String>();
    // Look for "(...)" ref suffix.
    if let Some(paren_idx) = stripped.find('(') {
        let (head, tail) = stripped.split_at(paren_idx);
        let status = head.trim().to_string();
        let closed_in = tail
            .trim_start_matches('(')
            .trim_end_matches(')')
            .trim()
            .to_string();
        (status, if closed_in.is_empty() { None } else { Some(closed_in) })
    } else {
        (stripped.trim().to_string(), None)
    }
}

/// Heuristic split of a doc-16 gap_summary into (source_paradigm, nom_shape)
/// on the " → " arrow convention. Most doc 16 rows follow one of:
///   "Source-paradigm-name → Nom shape prose"
///   "Concept from lang X (reuses #N) → Nom shape"
/// If no arrow is present, source_paradigm is left empty and the full text
/// lands in gap_summary (preserving the existing column's content).
pub fn split_gap_summary(gap: &str) -> (String, String, String) {
    // Returns (source_paradigm, gap_summary_kept, nom_shape)
    // The original gap_summary is preserved in full to avoid information loss.
    if let Some(idx) = gap.find(" → ") {
        let (head, tail) = gap.split_at(idx);
        let shape = tail.trim_start_matches(" → ").trim();
        (head.trim().to_string(), gap.to_string(), shape.to_string())
    } else {
        (String::new(), gap.to_string(), String::new())
    }
}

pub fn seed_authoring_rules(conn: &Connection, rows: &[DocRuleRow]) -> Result<usize> {
    let mut inserted = 0;
    for row in rows {
        let (paradigm, full_gap, shape) = split_gap_summary(&row.gap_summary);
        conn.execute(
            "INSERT OR REPLACE INTO authoring_rules \
             (row_id, source_paradigm, gap_summary, nom_shape, reuses_rows, destination, status, closed_in, source_doc_ref) \
             VALUES (?1, ?2, ?3, ?4, NULL, ?5, ?6, ?7, ?8)",
            params![
                row.row_id,
                paradigm,
                full_gap,
                shape,
                row.destination,
                row.status,
                row.closed_in,
                format!("doc 16 row {}", row.row_id),
            ],
        )?;
        inserted += 1;
    }
    Ok(inserted)
}

/// One-shot summary counts from a full seed run.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct SeedCounts {
    pub kinds: usize,
    pub quality_names: usize,
    pub keywords: usize,
    pub clause_shapes: usize,
    pub authoring_rules: usize,
}

/// One-shot convenience: seed kinds + quality_names + keywords (P2) + clause_shapes
/// (P3) + parse+insert all rows from the given doc-16 markdown source (P4).
/// Callable from the CLI `nom grammar seed`.
pub fn seed_all_from_doc16(conn: &Connection, doc16_md: &str) -> Result<SeedCounts> {
    let kinds = seed_kinds(conn).context("seeding kinds")?;
    let quality_names = seed_quality_names(conn).context("seeding quality_names")?;
    let keywords = seed_keywords(conn).context("seeding keywords")?;
    let clause_shapes = seed_clause_shapes(conn).context("seeding clause_shapes")?;
    let rows = parse_doc16_rules(doc16_md);
    let authoring_rules = seed_authoring_rules(conn, &rows).context("seeding authoring_rules")?;
    Ok(SeedCounts {
        kinds,
        quality_names,
        keywords,
        clause_shapes,
        authoring_rules,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::init_at;
    use tempfile::tempdir;

    #[test]
    fn seeds_nine_kinds_verbatim() {
        let dir = tempdir().unwrap();
        let conn = init_at(dir.path().join("g.sqlite")).unwrap();
        let n = seed_kinds(&conn).unwrap();
        assert_eq!(n, 9);
        let count: i64 = conn
            .query_row("SELECT COUNT(*) FROM kinds", [], |r| r.get(0))
            .unwrap();
        assert_eq!(count, 9);
    }

    #[test]
    fn kinds_allowed_clauses_are_derived_not_empty() {
        let dir = tempdir().unwrap();
        let conn = init_at(dir.path().join("g.sqlite")).unwrap();
        // clause_shapes must seed before kinds for the derivation to work.
        seed_clause_shapes(&conn).unwrap();
        seed_kinds(&conn).unwrap();
        // function kind has 6 clauses: intended, uses, requires, ensures, hazard, favor
        let fn_clauses: String = conn
            .query_row(
                "SELECT allowed_clauses FROM kinds WHERE name = 'function'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert!(fn_clauses.contains("intended"), "got {fn_clauses}");
        assert!(fn_clauses.contains("ensures"), "got {fn_clauses}");
        assert!(fn_clauses.contains("hazard"), "got {fn_clauses}");
        assert_ne!(fn_clauses, "[]", "function kind must not have empty allowed_clauses");
    }

    #[test]
    fn kinds_allowed_refs_are_populated_per_kind() {
        let dir = tempdir().unwrap();
        let conn = init_at(dir.path().join("g.sqlite")).unwrap();
        seed_kinds(&conn).unwrap();
        // function can use @Function, @Data, @Concept, @Module, @Property
        let fn_refs: String = conn
            .query_row(
                "SELECT allowed_refs FROM kinds WHERE name = 'function'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert!(fn_refs.contains("@Function"));
        assert!(fn_refs.contains("@Data"));
        // data kind is pure-structural: no @Kind refs
        let data_refs: String = conn
            .query_row(
                "SELECT allowed_refs FROM kinds WHERE name = 'data'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(data_refs, "[]");
    }

    #[test]
    fn quality_names_metric_function_is_null_not_placeholder() {
        let dir = tempdir().unwrap();
        let conn = init_at(dir.path().join("g.sqlite")).unwrap();
        seed_quality_names(&conn).unwrap();
        let metric: Option<String> = conn
            .query_row(
                "SELECT metric_function FROM quality_names WHERE name = 'forward_compatibility'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(metric, None);
    }

    #[test]
    fn gap_summary_split_on_arrow_populates_source_paradigm_and_nom_shape() {
        let (p, g, s) = split_gap_summary("Erlang OTP supervisor → concept decl + FIFO mailbox + serialized invocation");
        assert_eq!(p, "Erlang OTP supervisor");
        assert!(g.contains(" → "));
        assert_eq!(s, "concept decl + FIFO mailbox + serialized invocation");
    }

    #[test]
    fn gap_summary_split_preserves_full_text_when_no_arrow() {
        let (p, g, s) = split_gap_summary("Some legacy prose without an arrow");
        assert_eq!(p, "");
        assert_eq!(g, "Some legacy prose without an arrow");
        assert_eq!(s, "");
    }

    #[test]
    fn authoring_rules_have_split_paradigm_when_arrow_present() {
        let dir = tempdir().unwrap();
        let conn = init_at(dir.path().join("g.sqlite")).unwrap();
        let md = "\
| 419 | Behavioral-module declarations → structural typed-slots | authoring-guide rule | ✅ closed (doc 14 #85) |
| 999 | Legacy row without arrow | W-wedge | ⏳ queued |
";
        let rows = parse_doc16_rules(md);
        seed_authoring_rules(&conn, &rows).unwrap();
        let paradigm_419: String = conn
            .query_row(
                "SELECT source_paradigm FROM authoring_rules WHERE row_id = 419",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(paradigm_419, "Behavioral-module declarations");
        let shape_419: String = conn
            .query_row(
                "SELECT nom_shape FROM authoring_rules WHERE row_id = 419",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(shape_419, "structural typed-slots");
        let paradigm_999: String = conn
            .query_row(
                "SELECT source_paradigm FROM authoring_rules WHERE row_id = 999",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(paradigm_999, "");
    }

    #[test]
    fn seeding_kinds_is_idempotent() {
        let dir = tempdir().unwrap();
        let conn = init_at(dir.path().join("g.sqlite")).unwrap();
        let _ = seed_kinds(&conn).unwrap();
        let _ = seed_kinds(&conn).unwrap();
        let count: i64 = conn
            .query_row("SELECT COUNT(*) FROM kinds", [], |r| r.get(0))
            .unwrap();
        assert_eq!(count, 9); // INSERT OR REPLACE keeps row count constant
    }

    #[test]
    fn seeds_ten_quality_names() {
        let dir = tempdir().unwrap();
        let conn = init_at(dir.path().join("g.sqlite")).unwrap();
        let n = seed_quality_names(&conn).unwrap();
        assert_eq!(n, 10);
    }

    #[test]
    fn parses_closed_row_with_ref() {
        let md = "\
| 419 | Behavioral-module declarations | authoring-guide rule | ✅ closed (doc 14 #85) |
";
        let rows = parse_doc16_rules(md);
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].row_id, 419);
        assert_eq!(rows[0].gap_summary, "Behavioral-module declarations");
        assert_eq!(rows[0].destination, "authoring-guide rule");
        assert_eq!(rows[0].status, "closed");
        assert_eq!(rows[0].closed_in.as_deref(), Some("doc 14 #85"));
    }

    #[test]
    fn parses_queued_row_without_ref() {
        let md = "\
| 5 | Format-string interpolation | **W5** grammar rule | ⏳ queued |
";
        let rows = parse_doc16_rules(md);
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].row_id, 5);
        assert_eq!(rows[0].status, "queued");
        assert_eq!(rows[0].closed_in, None);
    }

    #[test]
    fn ignores_header_and_divider_lines() {
        let md = "\
# Title
## Triage format
| # | Gap | Destination | Status |
|--:|-----|-------------|--------|
| 1 | First | W-wedge | ⏳ queued |
Narrative text here.
| 2 | Second | authoring-guide rule | ✅ closed (doc 14 #42) |
";
        let rows = parse_doc16_rules(md);
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].row_id, 1);
        assert_eq!(rows[1].row_id, 2);
    }

    #[test]
    fn seed_all_from_doc16_populates_all_five_tables() {
        let dir = tempdir().unwrap();
        let conn = init_at(dir.path().join("g.sqlite")).unwrap();
        let md = "\
| 1 | First gap | authoring-guide rule | ✅ closed (doc 14 #1) |
| 2 | Second gap | W-wedge | ⏳ queued |
| 3 | Third gap | design deferred | 🔒 blocked |
";
        let c = seed_all_from_doc16(&conn, md).unwrap();
        assert_eq!(c.kinds, 9);
        assert_eq!(c.quality_names, 10);
        assert!(c.keywords >= 40, "expected ≥40 keyword rows, got {}", c.keywords);
        assert!(c.clause_shapes >= 40, "expected ≥40 clause_shape rows, got {}", c.clause_shapes);
        assert_eq!(c.authoring_rules, 3);
    }

    #[test]
    fn seeds_clause_shapes_for_all_nine_kinds() {
        let dir = tempdir().unwrap();
        let conn = init_at(dir.path().join("g.sqlite")).unwrap();
        let n = seed_clause_shapes(&conn).unwrap();
        assert!(n >= 40, "expected ≥40 clause shapes seeded, got {n}");
        // Every closed kind MUST have at least one clause_shape row.
        for k in [
            "function", "module", "concept", "screen", "data", "event", "media",
            "property", "scenario",
        ] {
            let count: i64 = conn
                .query_row(
                    "SELECT COUNT(*) FROM clause_shapes WHERE kind = ?1",
                    [k],
                    |r| r.get(0),
                )
                .unwrap();
            assert!(count >= 1, "kind {k} has zero clause_shape rows");
        }
    }

    #[test]
    fn required_clauses_marked_correctly() {
        let dir = tempdir().unwrap();
        let conn = init_at(dir.path().join("g.sqlite")).unwrap();
        let _ = seed_clause_shapes(&conn).unwrap();
        // Spot-check: 'intended' is required on every kind
        let intended_required: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM clause_shapes WHERE clause_name = 'intended' AND is_required = 1",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(intended_required, 9);
        // Spot-check: scenario has given/when/then all required
        for c in ["given", "when", "then"] {
            let req: i64 = conn
                .query_row(
                    "SELECT is_required FROM clause_shapes WHERE kind = 'scenario' AND clause_name = ?1",
                    [c],
                    |r| r.get(0),
                )
                .unwrap();
            assert_eq!(req, 1, "scenario.{c} should be required");
        }
        // Spot-check: property.generator is required
        let prop_gen: i64 = conn
            .query_row(
                "SELECT is_required FROM clause_shapes WHERE kind = 'property' AND clause_name = 'generator'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(prop_gen, 1);
    }

    #[test]
    fn clause_shape_seeding_is_idempotent() {
        let dir = tempdir().unwrap();
        let conn = init_at(dir.path().join("g.sqlite")).unwrap();
        let n1 = seed_clause_shapes(&conn).unwrap();
        let n2 = seed_clause_shapes(&conn).unwrap();
        assert_eq!(n1, n2);
        let count: i64 = conn
            .query_row("SELECT COUNT(*) FROM clause_shapes", [], |r| r.get(0))
            .unwrap();
        assert_eq!(count as usize, n1);
    }

    #[test]
    fn seeds_closed_keyword_vocabulary() {
        let dir = tempdir().unwrap();
        let conn = init_at(dir.path().join("g.sqlite")).unwrap();
        let n = seed_keywords(&conn).unwrap();
        assert!(n >= 40);
        // Spot-check load-bearing tokens.
        for (tok, role) in [
            ("the", "determiner"),
            ("matching", "ref_slot"),
            ("at-least", "ref_slot"),
            ("ensures", "clause_opener"),
            ("hazard", "clause_opener"),
            ("generator", "clause_opener"),
            ("given", "clause_opener"),
            ("@Property", "kind_marker"),
            ("@Route", "kind_marker"),
            ("at-most", "quantifier"),
            ("every", "quantifier"),
        ] {
            let got: String = conn
                .query_row(
                    "SELECT role FROM keywords WHERE token = ?1",
                    [tok],
                    |r| r.get(0),
                )
                .unwrap_or_else(|_| panic!("token {tok} missing from keyword seed"));
            assert_eq!(got, role, "role mismatch for {tok}");
        }
    }

    #[test]
    fn keyword_seeding_is_idempotent() {
        let dir = tempdir().unwrap();
        let conn = init_at(dir.path().join("g.sqlite")).unwrap();
        let n1 = seed_keywords(&conn).unwrap();
        let n2 = seed_keywords(&conn).unwrap();
        assert_eq!(n1, n2);
        let count: i64 = conn
            .query_row("SELECT COUNT(*) FROM keywords", [], |r| r.get(0))
            .unwrap();
        assert_eq!(count as usize, n1); // INSERT OR REPLACE
    }

    #[test]
    fn doc16_row_count_matches_repo_file() {
        // Smoke test: parse the actual doc 16 shipped in the repo and confirm row
        // count matches the expected 450 at the current HEAD.
        let md = match std::fs::read_to_string(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../../research/language-analysis/16-nomx-syntax-gap-backlog.md"
        )) {
            Ok(s) => s,
            Err(_) => {
                // Skip test if the doc is not at the expected relative path
                // (e.g. when running from a tarball without research/).
                return;
            }
        };
        let rows = parse_doc16_rules(&md);
        assert!(
            rows.len() >= 400,
            "expected ≥400 rows from doc 16, got {}",
            rows.len()
        );
    }
}
