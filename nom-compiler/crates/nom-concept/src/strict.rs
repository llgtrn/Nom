//! W4-A3 strict-mode validator for `.nomx v2` (doc 13 §5 A3).
//!
//! Additive pass over a parsed `NomFile` / `NomtuFile`. Surfaces style
//! violations that the default parser accepts but that doc 13 §5 A3
//! flags for opt-in rejection:
//!
//! - Typed-slot entity refs (`the @Kind matching "..."`) that omit a
//!   `with at-least N confidence` clause. Per doc 07 §6.3 every
//!   agentic-resolver ref should carry its threshold; making it
//!   mandatory in strict mode forces authors to make the resolver's
//!   confidence contract explicit.
//!
//! The pass is pure — it never mutates the AST or fails the parse. It
//! returns a list of `StrictWarning` values with enough locational
//! context (concept name + offending ref summary) for editors to
//! surface diagnostics.
//!
//! Opt-in: default parse still accepts the loose form; authors only
//! see warnings when they call `strict::validate_nom(&file)` (or the
//! matching `validate_nomtu`) explicitly. A future CLI flag or LSP
//! setting can wire this in wholesale.

use crate::{EntityRef, IndexClause, NomFile, NomtuFile, NomtuItem};

/// One strict-mode violation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StrictWarning {
    /// Short machine-friendly code. Editors can surface this as the
    /// diagnostic ID; future codes follow the `NOMX-<N>` convention.
    pub code: &'static str,
    /// One-line human-readable explanation.
    pub message: String,
    /// Best-effort locational hint — concept / entity name + the ref
    /// summary. The parser doesn't track positions for individual refs
    /// post-parse, so we surface the nearest structural identifier.
    pub location: String,
}

impl StrictWarning {
    fn missing_confidence(location: String, kind: &str, matching: Option<&str>) -> Self {
        let snippet = match matching {
            Some(m) => format!("the @{} matching \"{}\"", capitalize_kind(kind), m),
            None => format!("the @{}", capitalize_kind(kind)),
        };
        Self {
            code: "NOMX-A3",
            message: format!(
                "typed-slot ref `{snippet}` omits `with at-least N confidence`; \
                 strict mode requires every agentic-resolver ref to state its threshold"
            ),
            location,
        }
    }
}

fn capitalize_kind(kind: &str) -> String {
    let mut chars = kind.chars();
    match chars.next() {
        None => String::new(),
        Some(c) => c.to_uppercase().chain(chars).collect(),
    }
}

/// Validate a `NomFile` (multi-concept `.nom` source) against the
/// strict-mode rule set. Returns all warnings; empty vec = strict.
pub fn validate_nom(file: &NomFile) -> Vec<StrictWarning> {
    let mut out = Vec::new();
    for concept in &file.concepts {
        let loc_prefix = format!("concept `{}`", concept.name);
        for clause in &concept.index {
            if let IndexClause::Uses(refs) = clause {
                collect_typed_slot_warnings(refs, &loc_prefix, &mut out);
            }
        }
    }
    out
}

/// Validate a `NomtuFile` (multi-entity `.nomtu` source) against the
/// strict-mode rule set.  Walks composition decls, which are the
/// only items carrying typed-slot entity refs today.
pub fn validate_nomtu(file: &NomtuFile) -> Vec<StrictWarning> {
    let mut out = Vec::new();
    for item in &file.items {
        if let NomtuItem::Composition(comp) = item {
            let loc_prefix = format!("composition `{}`", comp.word);
            collect_typed_slot_warnings(&comp.composes, &loc_prefix, &mut out);
        }
    }
    out
}

fn collect_typed_slot_warnings(
    refs: &[EntityRef],
    loc_prefix: &str,
    out: &mut Vec<StrictWarning>,
) {
    for r in refs {
        if r.typed_slot && r.confidence_threshold.is_none() {
            let kind = r.kind.as_deref().unwrap_or("");
            out.push(StrictWarning::missing_confidence(
                loc_prefix.to_string(),
                kind,
                r.matching.as_deref(),
            ));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parse_nom;

    /// s01: a concept whose typed-slot ref carries `with at-least N
    /// confidence` yields zero strict warnings.
    #[test]
    fn s01_strict_concept_passes_clean() {
        let src = r#"
the concept s01 is
  intended to smoke-test strict mode pass-through.

  uses the @Function matching "auth flow" with at-least 0.85 confidence.

  favor correctness.
"#;
        let parsed = parse_nom(src).expect("parse");
        assert!(validate_nom(&parsed).is_empty(), "strict concept must pass");
    }

    /// s02: a concept whose typed-slot ref omits the threshold yields
    /// exactly one NOMX-A3 warning, and the warning carries the kind +
    /// matching string for editor surfacing.
    #[test]
    fn s02_missing_confidence_warns() {
        let src = r#"
the concept s02 is
  intended to smoke-test strict mode missing-confidence warning.

  uses the @Function matching "auth flow".

  favor correctness.
"#;
        let parsed = parse_nom(src).expect("parse");
        let warnings = validate_nom(&parsed);
        assert_eq!(warnings.len(), 1, "expected exactly one warning");
        let w = &warnings[0];
        assert_eq!(w.code, "NOMX-A3");
        assert!(
            w.location.contains("s02"),
            "warning must name the concept: {}",
            w.location
        );
        assert!(
            w.message.contains("@Function"),
            "warning must cite the kind: {}",
            w.message
        );
        assert!(
            w.message.contains("auth flow"),
            "warning must cite the matching phrase: {}",
            w.message
        );
    }

    /// s03: v1 bare-word form (`the function login_user matching "..."`)
    /// is NOT a typed-slot ref, so strict mode leaves it alone regardless
    /// of whether confidence is given.  v1 has its own resolution path
    /// (word+kind lookup, not matching-phrase embedding) so the threshold
    /// doesn't apply.
    #[test]
    fn s03_v1_bare_word_unaffected_by_strict_mode() {
        let src = r#"
the concept s03 is
  intended to test v1 bare-word is immune to strict mode.

  uses the function login_user matching "doesn't need threshold".

  favor correctness.
"#;
        let parsed = parse_nom(src).expect("parse");
        assert!(
            validate_nom(&parsed).is_empty(),
            "v1 form must not trigger NOMX-A3"
        );
    }

    /// s04: two typed-slot refs without thresholds produce two warnings.
    #[test]
    fn s04_multiple_missing_thresholds_collected() {
        let src = r#"
the concept s04 is
  intended to emit two warnings.

  uses the @Function matching "first".
  uses the @Module matching "second".

  favor correctness.
"#;
        let parsed = parse_nom(src).expect("parse");
        let warnings = validate_nom(&parsed);
        assert_eq!(
            warnings.len(),
            2,
            "expected two warnings, got {}",
            warnings.len()
        );
        assert!(warnings.iter().any(|w| w.message.contains("@Function")));
        assert!(warnings.iter().any(|w| w.message.contains("@Module")));
    }
}
