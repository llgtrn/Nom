//! Exhaustiveness checker for `when` clauses against `@Union` data types (GAP-12).
//!
//! A `when <var> is <variant> then <result>.` clause is exhaustive when every
//! variant declared in the corresponding `@Union of …` type is handled by at
//! least one `when` arm.  Missing variants are a warning (not an error) because
//! the source may intentionally omit some variants for a partial match.
//!
//! # Usage
//!
//! ```rust,ignore
//! use nom_concept::{WhenClause, UnionVariants, check_exhaustiveness};
//!
//! let whens = vec![
//!     WhenClause { variable: "m".into(), variant: "credit_card".into(), result: "\"Credit Card\"".into() },
//! ];
//! let union = UnionVariants { variants: vec!["credit_card".into(), "debit_card".into()] };
//! let missing = check_exhaustiveness(&whens, &union);
//! assert_eq!(missing, vec!["debit_card".to_string()]);
//! ```

use crate::{UnionVariants, WhenClause};

/// One exhaustiveness warning produced by [`check_exhaustiveness`].
///
/// Each `ExhaustivenessWarning` represents a single variant of the union
/// type that is not covered by any `when … is <variant> then …` arm.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExhaustivenessWarning {
    /// Short machine-friendly diagnostic code.
    pub code: &'static str,
    /// Human-readable explanation naming the missing variant.
    pub message: String,
    /// The variant name that has no matching `when` arm.
    pub missing_variant: String,
}

impl ExhaustivenessWarning {
    fn new(missing_variant: impl Into<String>) -> Self {
        let missing_variant = missing_variant.into();
        Self {
            code: "NOMX-GAP12-nonexhaustive",
            message: format!(
                "union variant `{missing_variant}` is not handled by any `when … is {missing_variant} then …` clause"
            ),
            missing_variant,
        }
    }
}

/// Check whether the given `when` clauses cover every variant in `union`.
///
/// Returns one [`ExhaustivenessWarning`] for each variant in `union.variants`
/// that is not mentioned in `whens`. The order of the returned warnings
/// matches the declaration order of the missing variants in `union.variants`.
///
/// An empty `whens` slice produces one warning per union variant.
/// A `whens` slice that covers all variants produces an empty `Vec`.
///
/// Only the `variant` field of each [`WhenClause`] is consulted; the
/// `variable` and `result` fields are ignored for coverage purposes.
pub fn check_exhaustiveness(
    whens: &[WhenClause],
    union: &UnionVariants,
) -> Vec<ExhaustivenessWarning> {
    let covered: std::collections::HashSet<&str> =
        whens.iter().map(|w| w.variant.as_str()).collect();

    union
        .variants
        .iter()
        .filter(|v| !covered.contains(v.as_str()))
        .map(|v| ExhaustivenessWarning::new(v.clone()))
        .collect()
}
