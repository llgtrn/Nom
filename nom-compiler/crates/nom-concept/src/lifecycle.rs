//! Entry lifecycle transitions: merge / eliminate / evolve.
//!
//! Tracks the progression of a dictionary entry from `Draft` through
//! `Complete`, and the terminal states `Deprecated` and `Eliminated`.

/// The state an entry currently occupies in the lifecycle.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EntryState {
    Draft,
    Partial,
    Complete,
    Deprecated,
    Eliminated,
}

impl EntryState {
    /// Human-readable name for display / CLI output.
    pub fn display_name(&self) -> &str {
        match self {
            EntryState::Draft => "draft",
            EntryState::Partial => "partial",
            EntryState::Complete => "complete",
            EntryState::Deprecated => "deprecated",
            EntryState::Eliminated => "eliminated",
        }
    }

    /// True for states that accept no further transitions.
    pub fn is_terminal(&self) -> bool {
        matches!(self, EntryState::Deprecated | EntryState::Eliminated)
    }

    /// Whether a direct state transition (not via a `LifecycleTransition`) is
    /// structurally valid.
    ///
    /// Valid edges:
    /// - Draft       → Partial | Eliminated
    /// - Partial     → Complete | Deprecated | Eliminated
    /// - Complete    → Deprecated | Eliminated
    /// - Deprecated  → Eliminated
    /// - Eliminated  → (nothing)
    pub fn can_transition_to(&self, next: &EntryState) -> bool {
        match self {
            EntryState::Draft => {
                matches!(next, EntryState::Partial | EntryState::Eliminated)
            }
            EntryState::Partial => {
                matches!(
                    next,
                    EntryState::Complete | EntryState::Deprecated | EntryState::Eliminated
                )
            }
            EntryState::Complete => {
                matches!(next, EntryState::Deprecated | EntryState::Eliminated)
            }
            EntryState::Deprecated => {
                matches!(next, EntryState::Eliminated)
            }
            EntryState::Eliminated => false,
        }
    }
}

/// A named transition applied by `LifecycleManager`.
#[derive(Debug, Clone, PartialEq)]
pub enum LifecycleTransition {
    /// Merge this entry into another (identified by hash). Entry becomes
    /// `Eliminated` once merged.
    Merge { into_hash: u64 },
    /// Permanently remove the entry for the given reason.
    Eliminate { reason: String },
    /// Introduce an evolved variant; the entry returns to `Partial` for
    /// re-validation against the new content.
    Evolve { new_hash: u64, delta: String },
}

impl LifecycleTransition {
    /// Short machine-readable name used in CLI output and logging.
    pub fn transition_name(&self) -> &str {
        match self {
            LifecycleTransition::Merge { .. } => "merge",
            LifecycleTransition::Eliminate { .. } => "eliminate",
            LifecycleTransition::Evolve { .. } => "evolve",
        }
    }
}

/// Applies and validates `LifecycleTransition`s against an `EntryState`.
pub struct LifecycleManager;

impl LifecycleManager {
    pub fn new() -> Self {
        LifecycleManager
    }

    /// Apply `transition` to `state`, returning the resulting `EntryState` or
    /// an error string describing why the transition is invalid.
    ///
    /// Rules:
    /// - `Merge`     → `Eliminated`  (only from `Complete` or `Partial`)
    /// - `Eliminate` → `Eliminated`  (always valid except from `Eliminated`)
    /// - `Evolve`    → `Partial`     (always valid except from `Eliminated`)
    pub fn apply_transition(
        state: &EntryState,
        transition: &LifecycleTransition,
    ) -> Result<EntryState, String> {
        if *state == EntryState::Eliminated {
            return Err(format!(
                "cannot apply '{}' transition: entry is already eliminated",
                transition.transition_name()
            ));
        }

        match transition {
            LifecycleTransition::Merge { .. } => {
                match state {
                    EntryState::Complete | EntryState::Partial => Ok(EntryState::Eliminated),
                    other => Err(format!(
                        "merge requires Complete or Partial state, got '{}'",
                        other.display_name()
                    )),
                }
            }
            LifecycleTransition::Eliminate { .. } => Ok(EntryState::Eliminated),
            LifecycleTransition::Evolve { .. } => Ok(EntryState::Partial),
        }
    }

    /// Names of transitions that may be applied to `state`.
    pub fn valid_transitions(state: &EntryState) -> Vec<&'static str> {
        match state {
            EntryState::Eliminated => vec![],
            EntryState::Complete | EntryState::Partial => vec!["merge", "eliminate", "evolve"],
            _ => vec!["eliminate", "evolve"],
        }
    }
}

impl Default for LifecycleManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod lifecycle_tests {
    use super::*;

    #[test]
    fn test_display_name() {
        assert_eq!(EntryState::Draft.display_name(), "draft");
        assert_eq!(EntryState::Partial.display_name(), "partial");
        assert_eq!(EntryState::Complete.display_name(), "complete");
        assert_eq!(EntryState::Deprecated.display_name(), "deprecated");
        assert_eq!(EntryState::Eliminated.display_name(), "eliminated");
    }

    #[test]
    fn test_is_terminal_deprecated() {
        assert!(EntryState::Deprecated.is_terminal());
    }

    #[test]
    fn test_is_terminal_eliminated() {
        assert!(EntryState::Eliminated.is_terminal());
    }

    #[test]
    fn test_can_transition_draft_to_partial_valid() {
        assert!(EntryState::Draft.can_transition_to(&EntryState::Partial));
    }

    #[test]
    fn test_can_transition_complete_to_draft_invalid() {
        assert!(!EntryState::Complete.can_transition_to(&EntryState::Draft));
    }

    #[test]
    fn test_transition_name() {
        assert_eq!(
            LifecycleTransition::Merge { into_hash: 42 }.transition_name(),
            "merge"
        );
        assert_eq!(
            LifecycleTransition::Eliminate { reason: "dup".into() }.transition_name(),
            "eliminate"
        );
        assert_eq!(
            LifecycleTransition::Evolve { new_hash: 7, delta: "v2".into() }.transition_name(),
            "evolve"
        );
    }

    #[test]
    fn test_apply_merge_becomes_eliminated() {
        let result = LifecycleManager::apply_transition(
            &EntryState::Complete,
            &LifecycleTransition::Merge { into_hash: 99 },
        );
        assert_eq!(result, Ok(EntryState::Eliminated));
    }

    #[test]
    fn test_apply_eliminate_becomes_eliminated() {
        let result = LifecycleManager::apply_transition(
            &EntryState::Draft,
            &LifecycleTransition::Eliminate { reason: "stale".into() },
        );
        assert_eq!(result, Ok(EntryState::Eliminated));
    }

    #[test]
    fn test_apply_evolve_becomes_partial() {
        let result = LifecycleManager::apply_transition(
            &EntryState::Complete,
            &LifecycleTransition::Evolve { new_hash: 10, delta: "patch".into() },
        );
        assert_eq!(result, Ok(EntryState::Partial));
    }

    #[test]
    fn test_apply_from_eliminated_returns_err() {
        let result = LifecycleManager::apply_transition(
            &EntryState::Eliminated,
            &LifecycleTransition::Eliminate { reason: "again".into() },
        );
        assert!(result.is_err());
    }

    #[test]
    fn test_valid_transitions_draft_non_empty() {
        let transitions = LifecycleManager::valid_transitions(&EntryState::Draft);
        assert!(!transitions.is_empty());
    }
}
