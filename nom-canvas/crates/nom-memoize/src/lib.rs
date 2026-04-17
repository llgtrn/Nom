//! Thread-local memoization primitive following the comemo pattern.
//!
//! Tracks fine-grained read dependencies through [`Tracked`] wrappers and
//! validates that cached results are still correct via [`Constraint`] records.
//!
//! # TODO
//! Procedural macro helpers (`#[nom_memoize]`, `#[nom_track]`) belong in a
//! separate proc-macro crate `nom-memoize-macros` (not created here).

#![deny(unsafe_code)]

pub mod cache;
pub mod constraint;
pub mod hash;
pub mod tracked;

pub use cache::{flush_thread_local, MemoizeCache, CACHE};
pub use constraint::{Constraint, Read};
pub use hash::fast_hash;
pub use tracked::{track, track_with, Tracked};
