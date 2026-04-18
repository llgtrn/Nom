//! Block diff/patch: compute and apply structural differences between block lists.
#![deny(unsafe_code)]
use crate::block_model::{BlockMeta, NomtuRef};
use serde::{Deserialize, Serialize};

/// A single change between two block lists, keyed by [`NomtuRef`].
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum BlockDiff {
    /// A block present in the new list but not in the old.
    Added(NomtuRef),
    /// A block present in the old list but not in the new.
    Removed(NomtuRef),
    /// A block whose field value changed between old and new.
    Modified {
        /// Identity of the block.
        id: NomtuRef,
        /// Name of the changed field.
        field: String,
        /// Value before the change.
        old: String,
        /// Value after the change.
        new: String,
    },
}

/// An entry in a diffable block list — carries identity + one inspectable field.
///
/// We re-use [`BlockMeta`] as the "payload" so callers can diff version/author fields
/// without needing the full [`crate::block_model::BlockModel`].
#[derive(Clone, Debug, PartialEq)]
pub struct DiffEntry {
    /// The unique identity of this block.
    pub id: NomtuRef,
    /// Audit metadata (version, author, timestamps) used as the diff surface.
    pub meta: BlockMeta,
}

impl DiffEntry {
    /// Construct a [`DiffEntry`] from an entity ref and default metadata.
    pub fn new(id: NomtuRef) -> Self {
        Self {
            id,
            meta: BlockMeta::default(),
        }
    }

    /// Construct a [`DiffEntry`] with explicit metadata.
    pub fn with_meta(id: NomtuRef, meta: BlockMeta) -> Self {
        Self { id, meta }
    }
}

/// Invert a single [`BlockDiff`]: `Added` ↔ `Removed`, `Modified` swaps `old`/`new`.
/// Returns `None` for variants that cannot be inverted.
pub fn invert(diff: &BlockDiff) -> Option<BlockDiff> {
    match diff {
        BlockDiff::Added(r) => Some(BlockDiff::Removed(r.clone())),
        BlockDiff::Removed(r) => Some(BlockDiff::Added(r.clone())),
        BlockDiff::Modified {
            id,
            field,
            old,
            new,
        } => Some(BlockDiff::Modified {
            id: id.clone(),
            field: field.clone(),
            old: new.clone(),
            new: old.clone(),
        }),
    }
}

/// Compare two ordered lists of [`DiffEntry`] values and return the minimal set of
/// [`BlockDiff`] operations that transforms `old` into `new`.
///
/// Rules:
/// - A ref present only in `old` → [`BlockDiff::Removed`].
/// - A ref present only in `new` → [`BlockDiff::Added`].
/// - A ref present in both but whose `meta.author` differs → [`BlockDiff::Modified`] on `"author"`.
/// - A ref present in both but whose `meta.version` differs → [`BlockDiff::Modified`] on `"version"`.
/// - A ref present in both but whose `id.kind` differs → [`BlockDiff::Modified`] on `"kind"`.
/// - Identical entries produce no diff entry.
pub fn diff_blocks(old: &[DiffEntry], new: &[DiffEntry]) -> Vec<BlockDiff> {
    use std::collections::HashMap;

    let old_map: HashMap<&str, &DiffEntry> = old.iter().map(|e| (e.id.id.as_str(), e)).collect();
    let new_map: HashMap<&str, &DiffEntry> = new.iter().map(|e| (e.id.id.as_str(), e)).collect();

    let mut diffs = Vec::new();

    // Removed: in old but not in new
    for entry in old {
        if !new_map.contains_key(entry.id.id.as_str()) {
            diffs.push(BlockDiff::Removed(entry.id.clone()));
        }
    }

    // Added: in new but not in old
    for entry in new {
        if !old_map.contains_key(entry.id.id.as_str()) {
            diffs.push(BlockDiff::Added(entry.id.clone()));
        }
    }

    // Modified: in both but fields differ
    for entry in new {
        if let Some(old_entry) = old_map.get(entry.id.id.as_str()) {
            if old_entry.meta.author != entry.meta.author {
                diffs.push(BlockDiff::Modified {
                    id: entry.id.clone(),
                    field: "author".to_string(),
                    old: old_entry.meta.author.clone(),
                    new: entry.meta.author.clone(),
                });
            }
            if old_entry.meta.version != entry.meta.version {
                diffs.push(BlockDiff::Modified {
                    id: entry.id.clone(),
                    field: "version".to_string(),
                    old: old_entry.meta.version.to_string(),
                    new: entry.meta.version.to_string(),
                });
            }
            if old_entry.id.kind != entry.id.kind {
                diffs.push(BlockDiff::Modified {
                    id: entry.id.clone(),
                    field: "kind".to_string(),
                    old: old_entry.id.kind.clone(),
                    new: entry.id.kind.clone(),
                });
            }
            if old_entry.id.word != entry.id.word {
                diffs.push(BlockDiff::Modified {
                    id: entry.id.clone(),
                    field: "word".to_string(),
                    old: old_entry.id.word.clone(),
                    new: entry.id.word.clone(),
                });
            }
        }
    }

    diffs
}

/// Apply a slice of [`BlockDiff`] operations to a mutable list of [`DiffEntry`] values in-place.
///
/// - [`BlockDiff::Added`] — appends a new entry with default metadata.
/// - [`BlockDiff::Removed`] — removes the first entry whose id matches.
/// - [`BlockDiff::Modified`] — updates the named field (`"author"` or `"version"`) on the
///   matching entry.
///
/// Unknown field names in `Modified` are silently ignored (forward-compatible).
pub fn apply_diff(blocks: &mut Vec<DiffEntry>, diff: &[BlockDiff]) {
    for op in diff {
        match op {
            BlockDiff::Added(id) => {
                // Only add if not already present
                if !blocks.iter().any(|e| e.id.id == id.id) {
                    blocks.push(DiffEntry::new(id.clone()));
                }
            }
            BlockDiff::Removed(id) => {
                blocks.retain(|e| e.id.id != id.id);
            }
            BlockDiff::Modified { id, field, new, .. } => {
                if let Some(entry) = blocks.iter_mut().find(|e| e.id.id == id.id) {
                    match field.as_str() {
                        "author" => entry.meta.author = new.clone(),
                        "version" => {
                            if let Ok(v) = new.parse::<u32>() {
                                entry.meta.version = v;
                            }
                        }
                        "word" => entry.id.word = new.clone(),
                        _ => {} // forward-compatible: ignore unknown fields
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry(id: &str) -> DiffEntry {
        DiffEntry::new(NomtuRef::new(id, "word", "concept"))
    }

    fn entry_with_author(id: &str, author: &str) -> DiffEntry {
        let meta = BlockMeta {
            author: author.to_string(),
            ..BlockMeta::default()
        };
        DiffEntry::with_meta(NomtuRef::new(id, "word", "concept"), meta)
    }

    fn entry_with_version(id: &str, version: u32) -> DiffEntry {
        let meta = BlockMeta {
            version,
            ..BlockMeta::default()
        };
        DiffEntry::with_meta(NomtuRef::new(id, "word", "concept"), meta)
    }

    /// diff of identical lists produces an empty diff.
    #[test]
    fn diff_identical_lists_empty() {
        let old = vec![entry("a"), entry("b")];
        let new = old.clone();
        let diffs = diff_blocks(&old, &new);
        assert!(
            diffs.is_empty(),
            "identical lists must produce no diffs, got: {diffs:?}"
        );
    }

    /// diff detects an added block (present in new, absent from old).
    #[test]
    fn diff_added_block_detected() {
        let old = vec![entry("a")];
        let new = vec![entry("a"), entry("b")];
        let diffs = diff_blocks(&old, &new);
        assert_eq!(diffs.len(), 1);
        assert!(
            matches!(&diffs[0], BlockDiff::Added(r) if r.id == "b"),
            "expected Added(b), got: {diffs:?}"
        );
    }

    /// diff detects a removed block (present in old, absent from new).
    #[test]
    fn diff_removed_block_detected() {
        let old = vec![entry("a"), entry("b")];
        let new = vec![entry("a")];
        let diffs = diff_blocks(&old, &new);
        assert_eq!(diffs.len(), 1);
        assert!(
            matches!(&diffs[0], BlockDiff::Removed(r) if r.id == "b"),
            "expected Removed(b), got: {diffs:?}"
        );
    }

    /// diff detects a Modified variant when author field changes.
    #[test]
    fn diff_modified_field_detected() {
        let old = vec![entry_with_author("a", "alice")];
        let new = vec![entry_with_author("a", "bob")];
        let diffs = diff_blocks(&old, &new);
        assert_eq!(diffs.len(), 1);
        assert!(
            matches!(&diffs[0], BlockDiff::Modified { id, field, old, new }
                if id.id == "a" && field == "author" && old == "alice" && new == "bob"),
            "expected Modified author alice→bob, got: {diffs:?}"
        );
    }

    /// apply(diff(old, new), old) produces a list equivalent to new.
    #[test]
    fn apply_diff_produces_new() {
        let old = vec![entry("x"), entry("y")];
        let new = vec![entry("x"), entry("z")];
        let diffs = diff_blocks(&old, &new);
        let mut result = old.clone();
        apply_diff(&mut result, &diffs);
        // result should contain "x" and "z", not "y"
        let ids: Vec<&str> = result.iter().map(|e| e.id.id.as_str()).collect();
        assert!(ids.contains(&"x"), "x must be present");
        assert!(ids.contains(&"z"), "z must be present");
        assert!(!ids.contains(&"y"), "y must be removed");
    }

    /// apply of an empty diff is a no-op — list unchanged.
    #[test]
    fn apply_empty_diff_is_noop() {
        let original = vec![entry("a"), entry("b"), entry("c")];
        let mut result = original.clone();
        apply_diff(&mut result, &[]);
        assert_eq!(result.len(), original.len());
        for (r, o) in result.iter().zip(original.iter()) {
            assert_eq!(r.id.id, o.id.id);
        }
    }

    /// apply diff twice is idempotent — result equals single application.
    #[test]
    fn apply_diff_idempotent() {
        let old = vec![entry("a"), entry("b")];
        let new = vec![entry("a"), entry("c")];
        let diffs = diff_blocks(&old, &new);

        let mut result_once = old.clone();
        apply_diff(&mut result_once, &diffs);

        let mut result_twice = result_once.clone();
        apply_diff(&mut result_twice, &diffs);

        // After applying the same diff twice, result should be the same as after one application
        let ids_once: Vec<&str> = result_once.iter().map(|e| e.id.id.as_str()).collect();
        let ids_twice: Vec<&str> = result_twice.iter().map(|e| e.id.id.as_str()).collect();
        assert_eq!(
            ids_once, ids_twice,
            "applying diff twice must be idempotent"
        );
    }

    // ── additional diff/patch tests ──────────────────────────────────────────

    /// diff of two empty lists produces an empty diff.
    #[test]
    fn diff_both_empty_produces_no_diff() {
        let diffs = diff_blocks(&[], &[]);
        assert!(diffs.is_empty());
    }

    /// diff when all blocks are removed produces only Removed variants.
    #[test]
    fn diff_all_removed() {
        let old = vec![entry("x"), entry("y"), entry("z")];
        let diffs = diff_blocks(&old, &[]);
        assert_eq!(diffs.len(), 3);
        assert!(diffs.iter().all(|d| matches!(d, BlockDiff::Removed(_))));
    }

    /// diff when all blocks are added produces only Added variants.
    #[test]
    fn diff_all_added() {
        let new = vec![entry("p"), entry("q")];
        let diffs = diff_blocks(&[], &new);
        assert_eq!(diffs.len(), 2);
        assert!(diffs.iter().all(|d| matches!(d, BlockDiff::Added(_))));
    }

    /// apply Modified diff updates the author field correctly.
    #[test]
    fn apply_modified_updates_author_field() {
        let mut blocks = vec![entry_with_author("e1", "alice")];
        let diff = vec![BlockDiff::Modified {
            id: NomtuRef::new("e1", "word", "concept"),
            field: "author".to_string(),
            old: "alice".to_string(),
            new: "bob".to_string(),
        }];
        apply_diff(&mut blocks, &diff);
        assert_eq!(blocks[0].meta.author, "bob");
    }

    /// apply Modified diff updates the version field correctly.
    #[test]
    fn apply_modified_updates_version_field() {
        let mut blocks = vec![entry_with_version("v1", 1)];
        let diff = vec![BlockDiff::Modified {
            id: NomtuRef::new("v1", "word", "concept"),
            field: "version".to_string(),
            old: "1".to_string(),
            new: "5".to_string(),
        }];
        apply_diff(&mut blocks, &diff);
        assert_eq!(blocks[0].meta.version, 5);
    }

    /// diff detects version change as Modified variant.
    #[test]
    fn diff_version_change_detected_as_modified() {
        let old = vec![entry_with_version("ver-block", 1)];
        let new = vec![entry_with_version("ver-block", 2)];
        let diffs = diff_blocks(&old, &new);
        assert_eq!(diffs.len(), 1);
        assert!(matches!(
            &diffs[0],
            BlockDiff::Modified { id, field, old, new }
                if id.id == "ver-block" && field == "version" && old == "1" && new == "2"
        ));
    }

    /// DiffEntry::new produces default meta with version == 1 and empty author.
    #[test]
    fn diff_entry_new_default_meta() {
        let e = entry("test-id");
        assert_eq!(e.meta.version, 1);
        assert!(e.meta.author.is_empty());
        assert_eq!(e.id.id, "test-id");
    }

    /// apply Removed on non-existent id is a no-op.
    #[test]
    fn apply_removed_nonexistent_is_noop() {
        let mut blocks = vec![entry("real")];
        let diff = vec![BlockDiff::Removed(NomtuRef::new("ghost", "w", "concept"))];
        apply_diff(&mut blocks, &diff);
        assert_eq!(blocks.len(), 1);
        assert_eq!(blocks[0].id.id, "real");
    }

    /// apply Added does not create duplicate when id already present.
    #[test]
    fn apply_added_no_duplicate_when_already_present() {
        let mut blocks = vec![entry("exists")];
        let diff = vec![BlockDiff::Added(NomtuRef::new("exists", "w", "concept"))];
        apply_diff(&mut blocks, &diff);
        assert_eq!(
            blocks.len(),
            1,
            "must not duplicate an already-present entry"
        );
    }

    /// BlockDiff::Added and Removed variants hold the correct NomtuRef.
    #[test]
    fn diff_variants_carry_correct_nomtu_ref() {
        let r = NomtuRef::new("my-id", "my-word", "my-kind");
        let added = BlockDiff::Added(r.clone());
        let removed = BlockDiff::Removed(r.clone());
        assert!(matches!(added, BlockDiff::Added(ref inner) if inner.id == "my-id"));
        assert!(matches!(removed, BlockDiff::Removed(ref inner) if inner.word == "my-word"));
    }

    /// diff result is empty when author and version are both unchanged.
    #[test]
    fn diff_unchanged_author_and_version_produces_no_diff() {
        let old = vec![entry_with_author("stable", "alice")];
        let mut new_entry = entry_with_author("stable", "alice");
        new_entry.meta.version = 1; // same as default
        let new = vec![new_entry];
        let diffs = diff_blocks(&old, &new);
        assert!(diffs.is_empty(), "no changes = no diffs, got: {diffs:?}");
    }

    /// apply_diff on a large list of Added ops appends all entries.
    #[test]
    fn apply_diff_large_added_list() {
        let mut blocks: Vec<DiffEntry> = Vec::new();
        let ops: Vec<BlockDiff> = (0..20u32)
            .map(|i| BlockDiff::Added(NomtuRef::new(format!("bulk-{i}"), "w", "concept")))
            .collect();
        apply_diff(&mut blocks, &ops);
        assert_eq!(blocks.len(), 20);
    }

    // ── BlockDiff::invert() and kind-change tests ────────────────────────────

    #[test]
    fn invert_added_becomes_removed() {
        let r = NomtuRef::new("x", "w", "concept");
        let diff = BlockDiff::Added(r.clone());
        let inv = invert(&diff).unwrap();
        assert!(matches!(inv, BlockDiff::Removed(ref inner) if inner.id == "x"));
    }

    #[test]
    fn invert_removed_becomes_added() {
        let r = NomtuRef::new("y", "w", "concept");
        let diff = BlockDiff::Removed(r.clone());
        let inv = invert(&diff).unwrap();
        assert!(matches!(inv, BlockDiff::Added(ref inner) if inner.id == "y"));
    }

    #[test]
    fn invert_modified_swaps_old_new() {
        let diff = BlockDiff::Modified {
            id: NomtuRef::new("m", "w", "concept"),
            field: "author".to_string(),
            old: "alice".to_string(),
            new: "bob".to_string(),
        };
        let inv = invert(&diff).unwrap();
        assert!(matches!(
            &inv,
            BlockDiff::Modified { old, new, .. } if old == "bob" && new == "alice"
        ));
    }

    #[test]
    fn invert_modified_preserves_field_name() {
        let diff = BlockDiff::Modified {
            id: NomtuRef::new("f", "w", "concept"),
            field: "version".to_string(),
            old: "1".to_string(),
            new: "2".to_string(),
        };
        let inv = invert(&diff).unwrap();
        assert!(matches!(&inv, BlockDiff::Modified { field, .. } if field == "version"));
    }

    #[test]
    fn invert_roundtrip_added() {
        let r = NomtuRef::new("rt", "w", "concept");
        let orig = BlockDiff::Added(r);
        let inv = invert(&orig).unwrap();
        let inv2 = invert(&inv).unwrap();
        assert_eq!(orig, inv2, "double-invert of Added must equal original");
    }

    #[test]
    fn invert_roundtrip_modified() {
        let orig = BlockDiff::Modified {
            id: NomtuRef::new("rt2", "w", "concept"),
            field: "author".to_string(),
            old: "a".to_string(),
            new: "b".to_string(),
        };
        let inv = invert(&orig).unwrap();
        let inv2 = invert(&inv).unwrap();
        assert_eq!(orig, inv2, "double-invert of Modified must equal original");
    }

    #[test]
    fn diff_kind_change_detected() {
        fn entry_kind(id: &str, kind: &str) -> DiffEntry {
            DiffEntry::new(NomtuRef::new(id, "w", kind))
        }
        let old = vec![entry_kind("kblock", "verb")];
        let new = vec![entry_kind("kblock", "concept")];
        let diffs = diff_blocks(&old, &new);
        let kind_diff = diffs
            .iter()
            .find(|d| matches!(d, BlockDiff::Modified { field, .. } if field == "kind"));
        assert!(
            kind_diff.is_some(),
            "kind change must be detected, diffs: {diffs:?}"
        );
        assert!(matches!(
            kind_diff.unwrap(),
            BlockDiff::Modified { old, new, .. } if old == "verb" && new == "concept"
        ));
    }

    #[test]
    fn diff_no_kind_change_produces_no_kind_diff() {
        let old = vec![entry("stable-kind")];
        let new = vec![entry("stable-kind")];
        let diffs = diff_blocks(&old, &new);
        assert!(
            !diffs
                .iter()
                .any(|d| matches!(d, BlockDiff::Modified { field, .. } if field == "kind")),
            "unchanged kind must not produce a kind diff"
        );
    }

    #[test]
    fn invert_modified_kind_diff() {
        let diff = BlockDiff::Modified {
            id: NomtuRef::new("kd", "w", "concept"),
            field: "kind".to_string(),
            old: "verb".to_string(),
            new: "concept".to_string(),
        };
        let inv = invert(&diff).unwrap();
        assert!(matches!(
            &inv,
            BlockDiff::Modified { field, old, new, .. }
                if field == "kind" && old == "concept" && new == "verb"
        ));
    }

    #[test]
    fn diff_kind_and_author_both_changed_produces_two_diffs() {
        let old_meta = BlockMeta {
            author: "alice".to_string(),
            ..BlockMeta::default()
        };
        let old_entry = DiffEntry::with_meta(NomtuRef::new("dual", "w", "verb"), old_meta);

        let new_meta = BlockMeta {
            author: "bob".to_string(),
            ..BlockMeta::default()
        };
        let new_entry = DiffEntry::with_meta(NomtuRef::new("dual", "w", "concept"), new_meta);

        let diffs = diff_blocks(&[old_entry], &[new_entry]);
        let has_author = diffs
            .iter()
            .any(|d| matches!(d, BlockDiff::Modified { field, .. } if field == "author"));
        let has_kind = diffs
            .iter()
            .any(|d| matches!(d, BlockDiff::Modified { field, .. } if field == "kind"));
        assert!(has_author, "author change must be detected");
        assert!(has_kind, "kind change must be detected");
    }

    #[test]
    fn diff_same_kind_different_word_no_kind_diff() {
        let old_entry = DiffEntry::new(NomtuRef::new("wdiff", "hello", "verb"));
        let new_entry = DiffEntry::new(NomtuRef::new("wdiff", "world", "verb"));
        let diffs = diff_blocks(&[old_entry], &[new_entry]);
        assert!(
            !diffs
                .iter()
                .any(|d| matches!(d, BlockDiff::Modified { field, .. } if field == "kind")),
            "same kind with different word must not produce kind diff"
        );
    }

    #[test]
    fn invert_preserves_id_in_modified() {
        let id = NomtuRef::new("preserve-me", "w", "concept");
        let diff = BlockDiff::Modified {
            id: id.clone(),
            field: "version".to_string(),
            old: "3".to_string(),
            new: "4".to_string(),
        };
        let inv = invert(&diff).unwrap();
        assert!(
            matches!(&inv, BlockDiff::Modified { id: inv_id, .. } if inv_id.id == "preserve-me")
        );
    }

    #[test]
    fn diff_multiple_kind_changes_all_detected() {
        fn ek(id: &str, kind: &str) -> DiffEntry {
            DiffEntry::new(NomtuRef::new(id, "w", kind))
        }
        let old = vec![ek("a", "verb"), ek("b", "concept"), ek("c", "noun")];
        let new = vec![ek("a", "concept"), ek("b", "verb"), ek("c", "noun")];
        let diffs = diff_blocks(&old, &new);
        let kind_diffs: Vec<_> = diffs
            .iter()
            .filter(|d| matches!(d, BlockDiff::Modified { field, .. } if field == "kind"))
            .collect();
        assert_eq!(
            kind_diffs.len(),
            2,
            "two kind changes must be detected, got: {diffs:?}"
        );
    }

    #[test]
    fn invert_added_then_removed_roundtrip() {
        let r = NomtuRef::new("rtrip", "w", "concept");
        let added = BlockDiff::Added(r.clone());
        let removed = invert(&added).unwrap();
        let back = invert(&removed).unwrap();
        assert_eq!(added, back, "Added -> invert -> invert must equal original");
    }

    #[test]
    fn diff_entry_with_meta_stores_meta() {
        let meta = BlockMeta {
            author: "tester".to_string(),
            version: 7,
            ..BlockMeta::default()
        };
        let e = DiffEntry::with_meta(NomtuRef::new("wm", "w", "concept"), meta.clone());
        assert_eq!(e.meta.author, "tester");
        assert_eq!(e.meta.version, 7);
    }

    #[test]
    fn apply_diff_modified_kind_is_ignored_gracefully() {
        // apply_diff currently only handles author/version fields — unknown fields are silently ignored
        let mut blocks = vec![entry("ktest")];
        let diff = vec![BlockDiff::Modified {
            id: NomtuRef::new("ktest", "w", "concept"),
            field: "kind".to_string(),
            old: "verb".to_string(),
            new: "concept".to_string(),
        }];
        apply_diff(&mut blocks, &diff);
        // Should not panic; entry is unchanged
        assert_eq!(blocks.len(), 1);
        assert_eq!(blocks[0].id.id, "ktest");
    }
}

// ── Typed diff record and accumulator ────────────────────────────────────────

/// Discriminant for a single block-level change.
#[derive(Clone, Debug, PartialEq)]
pub enum BlockDiffKind {
    /// Block was introduced in the new revision.
    Added,
    /// Block was present in the old revision but absent in the new.
    Removed,
    /// A field on the block changed between revisions.
    Modified,
    /// Block retained its identity but moved to a different position.
    Moved,
}

/// A flat, typed record of one change to a block, identified by a numeric id.
#[derive(Clone, Debug, PartialEq)]
pub struct BlockDiffRecord {
    /// The kind of change.
    pub kind: BlockDiffKind,
    /// Numeric identifier of the affected block.
    pub block_id: u32,
    /// Name of the changed field (empty for Added / Removed).
    pub field: String,
    /// Value before the change, if applicable.
    pub old_value: Option<String>,
    /// Value after the change, if applicable.
    pub new_value: Option<String>,
}

impl BlockDiffRecord {
    /// Construct an `Added` record for the given block id.
    pub fn added(id: u32) -> Self {
        Self {
            kind: BlockDiffKind::Added,
            block_id: id,
            field: String::new(),
            old_value: None,
            new_value: None,
        }
    }

    /// Construct a `Removed` record for the given block id.
    pub fn removed(id: u32) -> Self {
        Self {
            kind: BlockDiffKind::Removed,
            block_id: id,
            field: String::new(),
            old_value: None,
            new_value: None,
        }
    }

    /// Construct a `Modified` record recording a field transition.
    pub fn modified(id: u32, field: &str, old: &str, new: &str) -> Self {
        Self {
            kind: BlockDiffKind::Modified,
            block_id: id,
            field: field.to_string(),
            old_value: Some(old.to_string()),
            new_value: Some(new.to_string()),
        }
    }
}

/// An ordered accumulator of [`BlockDiffRecord`] entries.
#[derive(Debug, Default)]
pub struct BlockDiffer {
    /// The accumulated diff records.
    pub diffs: Vec<BlockDiffRecord>,
}

impl BlockDiffer {
    /// Create an empty differ.
    pub fn new() -> Self {
        Self { diffs: Vec::new() }
    }

    /// Append one diff record.
    pub fn push(&mut self, diff: BlockDiffRecord) {
        self.diffs.push(diff);
    }

    /// Total number of diff records.
    pub fn diff_count(&self) -> usize {
        self.diffs.len()
    }

    /// Number of `Added` records.
    pub fn added_count(&self) -> usize {
        self.diffs
            .iter()
            .filter(|d| d.kind == BlockDiffKind::Added)
            .count()
    }

    /// Number of `Removed` records.
    pub fn removed_count(&self) -> usize {
        self.diffs
            .iter()
            .filter(|d| d.kind == BlockDiffKind::Removed)
            .count()
    }

    /// Number of `Modified` records.
    pub fn modified_count(&self) -> usize {
        self.diffs
            .iter()
            .filter(|d| d.kind == BlockDiffKind::Modified)
            .count()
    }
}

#[cfg(test)]
mod typed_diff_tests {
    use super::{BlockDiffKind, BlockDiffRecord, BlockDiffer};

    /// BlockDiffRecord::added produces an Added record with the correct block_id.
    #[test]
    fn diff_added() {
        let r = BlockDiffRecord::added(42);
        assert_eq!(r.kind, BlockDiffKind::Added);
        assert_eq!(r.block_id, 42);
        assert!(r.old_value.is_none());
        assert!(r.new_value.is_none());
    }

    /// BlockDiffRecord::removed produces a Removed record with the correct block_id.
    #[test]
    fn diff_removed() {
        let r = BlockDiffRecord::removed(7);
        assert_eq!(r.kind, BlockDiffKind::Removed);
        assert_eq!(r.block_id, 7);
        assert!(r.field.is_empty());
    }

    /// BlockDiffRecord::modified captures field, old_value, and new_value correctly.
    #[test]
    fn diff_modified() {
        let r = BlockDiffRecord::modified(1, "title", "Hello", "World");
        assert_eq!(r.kind, BlockDiffKind::Modified);
        assert_eq!(r.block_id, 1);
        assert_eq!(r.field, "title");
        assert_eq!(r.old_value.as_deref(), Some("Hello"));
        assert_eq!(r.new_value.as_deref(), Some("World"));
    }

    /// BlockDiffer::diff_count returns the total number of pushed records.
    #[test]
    fn differ_counts() {
        let mut d = BlockDiffer::new();
        d.push(BlockDiffRecord::added(1));
        d.push(BlockDiffRecord::removed(2));
        d.push(BlockDiffRecord::modified(3, "x", "a", "b"));
        assert_eq!(d.diff_count(), 3);
    }

    /// BlockDiffer::added_count counts only Added records.
    #[test]
    fn differ_added_count() {
        let mut d = BlockDiffer::new();
        d.push(BlockDiffRecord::added(1));
        d.push(BlockDiffRecord::added(2));
        d.push(BlockDiffRecord::removed(3));
        assert_eq!(d.added_count(), 2);
        assert_eq!(d.removed_count(), 1);
        assert_eq!(d.modified_count(), 0);
    }

    /// BlockDiffer::modified_count counts only Modified records.
    #[test]
    fn differ_modified_count() {
        let mut d = BlockDiffer::new();
        d.push(BlockDiffRecord::modified(10, "field", "old", "new"));
        d.push(BlockDiffRecord::modified(11, "other", "x", "y"));
        d.push(BlockDiffRecord::added(12));
        assert_eq!(d.modified_count(), 2);
        assert_eq!(d.added_count(), 1);
    }
}
