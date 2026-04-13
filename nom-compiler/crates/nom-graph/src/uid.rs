//! Content-hash node identity for the graph-durability refactor.
//!
//! Phase 2a of the graph-durability spec (docs/superpowers/specs/
//! 2026-04-14-graph-durability-design.md). This module is self-contained:
//! it introduces `NodeUid` + `compute_node_uid` without touching the
//! existing positional-index adjacency in `lib.rs`. Future phases (2b
//! upsert_entry, 2c HashMap storage) migrate callers one step at a time.
//!
//! ## Identity scheme
//!
//! ```text
//! NodeUid = hex(SHA-256(word || "\0" || kind || "\0" || body_hash))
//! ```
//!
//! Notes:
//! - Null-byte separator (not `::` as the spec sketched) because `::`
//!   can appear legitimately in tokenized Rust paths like `std::fmt`;
//!   a control byte is unambiguous across every language we ingest.
//! - `body_hash` is carried as `Option<String>`; when absent we hash the
//!   sentinel `"<no-body>"` so entries without bodies still get a stable
//!   uid (different from any non-empty hash).
//! - Variant is NOT part of uid. Two entries with same word+kind+body
//!   but different variant metadata have the same uid — variant is
//!   surface decoration, not identity. Same spec decision as nom-dict
//!   words_v2 where `hash` uses body content, not variant.
//!
//! ## Why content-addressed?
//!
//! The spec's motivation (§Problem, point 2): `nom-graph` currently uses
//! positional indices into `Vec<NomtuNode>`. A body edit → new hash →
//! new graph position → every existing edge silently broken.
//! Content-addressed identity means: a body edit produces a *new* uid,
//! and the rename-chain mechanism (Phase 2b's `prior_hashes`) records
//! the link between old and new so edges can be reattached deliberately
//! instead of silently lost.

use sha2::{Digest, Sha256};

/// Content-hash node identity. 64-char lowercase hex encoding of SHA-256.
///
/// Use the type alias in public APIs so future wedges can swap the
/// representation without an API break (e.g. to a fixed `[u8; 32]` if
/// profiling shows the String allocations matter).
pub type NodeUid = String;

/// Fallback body hash used for entries that have no body bytes. A literal
/// sentinel (not an empty string) so "no body" is distinguishable from
/// any real body hash and from a malformed empty input.
pub const NO_BODY_SENTINEL: &str = "<no-body>";

/// Separator byte between identity fields. Null byte because it cannot
/// appear inside any input string we'd hash (word, kind, body_hash are
/// all UTF-8 text) so ambiguity is structurally impossible.
const SEP: u8 = 0;

/// Compute the `NodeUid` for a `NomtuEntry`. Pure function: given the
/// same (word, kind, body_hash-or-none) triple it always returns the
/// same 64-char hex string, across platforms and across runs.
///
/// Variant metadata is intentionally excluded — entries that share
/// word+kind+body but differ in variant decoration have the same uid.
/// The `NomtuGraph` storage will still keep both `NomtuNode`s because
/// positional presence is decided by existence in the entry list, not
/// by uid uniqueness (though Phase 2c's HashMap storage WILL collapse
/// them — that's a later design decision, not Phase 2a's concern).
pub fn compute_node_uid(entry: &nom_types::NomtuEntry) -> NodeUid {
    let body = entry.body_hash.as_deref().unwrap_or(NO_BODY_SENTINEL);
    compute_from_fields(&entry.word, &entry.kind, body)
}

/// Field-level constructor. Exposed so callers who only have the raw
/// fields (e.g. Cypher import dialog, Phase 3 export roundtrip) can
/// compute a uid without synthesizing a full `NomtuEntry`.
pub fn compute_from_fields(word: &str, kind: &str, body_hash: &str) -> NodeUid {
    let mut hasher = Sha256::new();
    hasher.update(word.as_bytes());
    hasher.update([SEP]);
    hasher.update(kind.as_bytes());
    hasher.update([SEP]);
    hasher.update(body_hash.as_bytes());
    format!("{:x}", hasher.finalize())
}

#[cfg(test)]
mod tests {
    use super::*;
    use nom_types::NomtuEntry;

    fn mk_entry(word: &str, kind: &str, body: Option<&str>) -> NomtuEntry {
        NomtuEntry {
            word: word.into(),
            kind: kind.into(),
            body_hash: body.map(|s| s.into()),
            language: "rust".into(),
            ..Default::default()
        }
    }

    #[test]
    fn uid_is_64_char_lowercase_hex() {
        let e = mk_entry("add", "function", Some("deadbeef"));
        let uid = compute_node_uid(&e);
        assert_eq!(uid.len(), 64);
        assert!(uid.chars().all(|c| c.is_ascii_hexdigit() && !c.is_uppercase()));
    }

    #[test]
    fn uid_is_deterministic() {
        let e1 = mk_entry("add", "function", Some("deadbeef"));
        let e2 = mk_entry("add", "function", Some("deadbeef"));
        assert_eq!(compute_node_uid(&e1), compute_node_uid(&e2));
    }

    #[test]
    fn uid_changes_with_word() {
        let a = compute_node_uid(&mk_entry("add", "function", Some("h")));
        let b = compute_node_uid(&mk_entry("mul", "function", Some("h")));
        assert_ne!(a, b);
    }

    #[test]
    fn uid_changes_with_kind() {
        let a = compute_node_uid(&mk_entry("x", "function", Some("h")));
        let b = compute_node_uid(&mk_entry("x", "module", Some("h")));
        assert_ne!(a, b);
    }

    #[test]
    fn uid_changes_with_body_hash() {
        let a = compute_node_uid(&mk_entry("add", "function", Some("h1")));
        let b = compute_node_uid(&mk_entry("add", "function", Some("h2")));
        assert_ne!(a, b, "body change must produce new uid (rename signal)");
    }

    #[test]
    fn uid_with_no_body_hashes_sentinel_stably() {
        let no_body1 = compute_node_uid(&mk_entry("x", "function", None));
        let no_body2 = compute_node_uid(&mk_entry("x", "function", None));
        let with_body =
            compute_node_uid(&mk_entry("x", "function", Some(NO_BODY_SENTINEL)));
        // Two no-body entries produce the same uid …
        assert_eq!(no_body1, no_body2);
        // … and that uid equals an entry explicitly hashing the sentinel.
        assert_eq!(no_body1, with_body);
    }

    #[test]
    fn variant_does_not_affect_uid() {
        let mut a = mk_entry("x", "function", Some("h"));
        let mut b = mk_entry("x", "function", Some("h"));
        a.variant = Some("v1".into());
        b.variant = Some("v2".into());
        assert_eq!(
            compute_node_uid(&a),
            compute_node_uid(&b),
            "variant is surface decoration, not identity"
        );
    }

    #[test]
    fn null_byte_separator_blocks_ambiguity() {
        // If we concatenated without a delimiter, ("ab", "c") and ("a", "bc")
        // would collide. Verify they don't.
        let a = compute_from_fields("ab", "c", "body");
        let b = compute_from_fields("a", "bc", "body");
        assert_ne!(a, b);
    }
}
