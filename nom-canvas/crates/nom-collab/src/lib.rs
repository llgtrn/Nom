//! nom-collab — CRDT snapshot + protocol scaffolding for Nom collab server.
//!
//! Phase 5 scaffolding: data structures and protocol types only.
//! No yrs dependency, no WebSocket server — pure types + in-memory impls.

#![deny(unsafe_code)]

pub mod awareness;
pub mod auth;
pub mod doc_id;
pub mod offline;
pub mod persistence;
pub mod protocol;
pub mod snapshot;
pub mod transaction;
pub mod presence;
