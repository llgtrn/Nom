//! nom-panels — panel state and data layer for NomCanvas IDE.
//!
//! Provides the data/state structs for all IDE panels. GPU rendering is handled
//! by the renderer layer; paint methods here are stubs returning `()`.
//!
//! # Panels
//! - [`sidebar`]          — document tree, search, recent
//! - [`toolbar`]          — active block/flavour, actions
//! - [`preview`]          — document preview pane
//! - [`library`]          — shared asset library
//! - [`command_palette`]  — fuzzy command search
//! - [`statusbar`]        — compile status, cursor, git branch, diagnostics
//! - [`mode_switcher`]    — unified editing mode per-document
//! - [`properties`]       — property inspector for selected block

#![deny(unsafe_code)]

pub mod command_history;
pub mod layout;
pub mod command_palette;
pub mod library;
pub mod mode_switcher;
pub mod preview;
pub mod properties;
pub mod shortcuts;
pub mod sidebar;
pub mod statusbar;
pub mod toolbar;

/// Opaque document identifier used across all panels.
pub type DocumentId = u64;
