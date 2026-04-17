//! In-place mutation and spread-replacement for canvas elements.
//!
//! Every write to an [`Element`] MUST go through one of these two functions so
//! that the CRDT versioning fields (`version`, `version_nonce`) stay consistent.
//!
//! ## Patterns
//!
//! **In-place mutation** (caller already holds `&mut Element`):
//! ```no_run
//! # use nom_canvas_core::{Element, mutation::mutate};
//! # fn example(e: &mut Element) {
//! mutate(e, |el| el.opacity = 0.75);
//! # }
//! ```
//!
//! **Spread-replacement** (produce a new value, keep the original unchanged):
//! ```no_run
//! # use nom_canvas_core::{Element, mutation::replace_with};
//! # fn example(e: &Element) -> Element {
//! let updated = replace_with(e, |el| el.locked = true);
//! # updated
//! # }
//! ```

use crate::element::{version_nonce_for, Element};

/// Mutate `element` in-place using `f`, then advance the version counters.
///
/// The version is incremented by 1 and a fresh deterministic nonce is derived
/// from `(id, new_version)`.
pub fn mutate(element: &mut Element, f: impl FnOnce(&mut Element)) {
    f(element);
    element.version += 1;
    element.version_nonce = version_nonce_for(element.id, element.version);
}

/// Clone `element`, apply `f` to the clone, advance its version counters,
/// and return it — leaving the original untouched.
///
/// This is the "spread" pattern from Excalidraw's `newElementWith` helper:
/// callers can snapshot the current state, produce a modified copy, and decide
/// whether to commit it (e.g., for undo/redo or CRDT patch generation).
pub fn replace_with(element: &Element, f: impl FnOnce(&mut Element)) -> Element {
    let mut next = element.clone();
    mutate(&mut next, f);
    next
}
