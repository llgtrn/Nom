//! Block model primitives: [`BlockId`], [`NomtuRef`], [`BlockMeta`], [`BlockModel`].
#![deny(unsafe_code)]
use serde::{Deserialize, Serialize};

/// Unique identifier for a block within a workspace.
pub type BlockId = String;

/// Every block MUST have a DB entity reference. No Option<> wrapper.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct NomtuRef {
    /// Database entry identifier.
    pub id: String,
    /// Human-readable word for the entry.
    pub word: String,
    /// Grammar kind of the entry (verb, concept, noun, …).
    pub kind: String,
}

impl NomtuRef {
    /// Construct a [`NomtuRef`] from the three identifying strings.
    pub fn new(id: impl Into<String>, word: impl Into<String>, kind: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            word: word.into(),
            kind: kind.into(),
        }
    }
}

/// Audit metadata attached to every block.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct BlockMeta {
    /// Unix timestamp (ms) when the block was created.
    pub created_at: u64,
    /// Unix timestamp (ms) of the most recent update.
    pub updated_at: u64,
    /// Author identifier.
    pub author: String,
    /// Monotonically increasing version counter.
    pub version: u32,
}

impl Default for BlockMeta {
    fn default() -> Self {
        Self {
            created_at: 0,
            updated_at: 0,
            author: String::new(),
            version: 1,
        }
    }
}

/// The core document block — a typed, entity-backed unit of content.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct BlockModel {
    /// Unique identifier within the workspace.
    pub id: BlockId,
    /// NON-OPTIONAL DB entity reference. Blocks without an entity do not exist.
    pub entity: NomtuRef,
    /// Block flavour string (e.g. `"affine:paragraph"`).
    pub flavour: String,
    /// Named slot values carrying typed content.
    pub slots: Vec<(String, crate::slot::SlotValue)>,
    /// IDs of child blocks (ordered).
    pub children: Vec<BlockId>,
    /// Audit metadata.
    pub meta: BlockMeta,
    /// Optional parent block ID.
    pub parent: Option<BlockId>,
}

impl BlockModel {
    /// Construct a [`BlockModel`] directly from its three required fields.
    pub fn new(id: impl Into<String>, entity: NomtuRef, flavour: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            entity,
            flavour: flavour.into(),
            slots: Vec::new(),
            children: Vec::new(),
            meta: BlockMeta::default(),
            parent: None,
        }
    }

    /// Return the value of the named slot if present.
    pub fn get_slot(&self, name: &str) -> Option<&crate::slot::SlotValue> {
        self.slots.iter().find(|(k, _)| k == name).map(|(_, v)| v)
    }

    /// Set (or overwrite) a named slot value.
    pub fn set_slot(&mut self, name: impl Into<String>, value: crate::slot::SlotValue) {
        let name = name.into();
        if let Some(entry) = self.slots.iter_mut().find(|(k, _)| *k == name) {
            entry.1 = value;
        } else {
            self.slots.push((name, value));
        }
    }

    /// Create a block with a generated ID, validating `entity.kind` against the dict.
    pub fn insert(
        entity: NomtuRef,
        flavour: impl Into<String>,
        dict: &dyn crate::dict_reader::DictReader,
    ) -> Self {
        let kind = entity.kind.clone();
        debug_assert!(dict.is_known_kind(&kind), "Unknown grammar kind: {kind}");
        let id = uuid_v4();
        Self::new(id, entity, flavour)
    }
}

fn uuid_v4() -> String {
    // Simple deterministic ID for Wave B — real UUID in Wave C
    use std::time::{SystemTime, UNIX_EPOCH};
    let t = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    format!("{:032x}", t)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::stub_dict::StubDictReader;

    #[test]
    fn block_model_has_entity() {
        let entity = NomtuRef::new("id1", "summarize", "verb");
        let reader = StubDictReader::with_kinds(&["verb"]);
        let block = BlockModel::insert(entity.clone(), "affine:paragraph", &reader);
        assert_eq!(block.entity.kind, "verb");
        assert_eq!(block.entity.word, "summarize");
    }

    #[test]
    fn block_model_slots() {
        let mut block = BlockModel::new("id1", NomtuRef::new("id1", "w", "k"), "affine:paragraph");
        block.set_slot("text", crate::slot::SlotValue::Text("Hello".into()));
        assert!(block.get_slot("text").is_some());
        assert!(block.get_slot("missing").is_none());
    }

    /// Blueprint invariant: entity field is non-optional — BlockModel::new requires a concrete NomtuRef.
    #[test]
    fn block_model_nomtu_ref_required() {
        let entity = NomtuRef::new("eid", "plan", "concept");
        let block = BlockModel::new("blk1", entity, "affine:note");
        // Structural guarantee: entity is always present, not Option<NomtuRef>
        assert_eq!(block.entity.id, "eid");
        assert_eq!(block.entity.word, "plan");
        assert_eq!(block.entity.kind, "concept");
    }

    /// BlockModel::new with valid args succeeds and fields are populated correctly.
    #[test]
    fn block_model_valid_creates() {
        let entity = NomtuRef::new("e42", "render", "verb");
        let block = BlockModel::new("b42", entity, "affine:paragraph");
        assert_eq!(block.id, "b42");
        assert_eq!(block.flavour, "affine:paragraph");
        assert!(block.slots.is_empty());
        assert!(block.children.is_empty());
        assert!(block.parent.is_none());
        assert_eq!(block.meta.version, 1);
    }

    /// uuid_v4-based IDs generated by insert() are distinct across two successive calls.
    #[test]
    fn block_id_is_unique() {
        let dict = StubDictReader::new();
        let e1 = NomtuRef::new("e1", "fetch", "verb");
        let e2 = NomtuRef::new("e2", "store", "verb");
        let b1 = BlockModel::insert(e1, "affine:paragraph", &dict);
        let b2 = BlockModel::insert(e2, "affine:paragraph", &dict);
        assert_ne!(
            b1.id, b2.id,
            "Two successive insert() calls must produce distinct IDs"
        );
    }

    /// NomtuRef id field round-trips through a hex-format string (matching uuid_v4 format).
    #[test]
    fn nomtu_ref_id_roundtrips() {
        let hex_id = format!("{:032x}", 0xcafe_u64);
        let r = NomtuRef::new(hex_id.clone(), "cafe", "concept");
        assert_eq!(r.id, hex_id);
        // Parse back to confirm it is valid hex
        let parsed = u64::from_str_radix(r.id.trim_start_matches('0'), 16).unwrap_or(0);
        assert_eq!(parsed, 0xcafe_u64);
    }

    /// BlockModel.kind is accessible via entity.kind field
    #[test]
    fn block_model_kind_field() {
        let entity = NomtuRef::new("e1", "plan", "concept");
        let block = BlockModel::new("b1", entity, "affine:note");
        assert_eq!(block.entity.kind, "concept");
    }

    /// entity field is NomtuRef (non-optional) with all three sub-fields accessible
    #[test]
    fn block_model_entity_is_nomturef() {
        let entity = NomtuRef::new("eid", "compose", "verb");
        let block = BlockModel::new("b1", entity, "affine:paragraph");
        // Access all three fields of NomtuRef to confirm the type
        let _id: &str = &block.entity.id;
        let _word: &str = &block.entity.word;
        let _kind: &str = &block.entity.kind;
        assert_eq!(block.entity.id, "eid");
        assert_eq!(block.entity.word, "compose");
        assert_eq!(block.entity.kind, "verb");
    }

    /// children is Vec<BlockId> and starts empty
    #[test]
    fn block_model_children_vec() {
        let block = BlockModel::new("b1", NomtuRef::new("e1", "w", "verb"), "affine:note");
        assert!(block.children.is_empty());
        let mut block = block;
        block.children.push("child-1".to_string());
        block.children.push("child-2".to_string());
        assert_eq!(block.children.len(), 2);
        assert_eq!(block.children[0], "child-1");
    }

    /// parent is Option<BlockId> and defaults to None
    #[test]
    fn block_model_parent_optional() {
        let mut block = BlockModel::new("b1", NomtuRef::new("e1", "w", "verb"), "affine:note");
        assert!(block.parent.is_none());
        block.parent = Some("parent-block".to_string());
        assert_eq!(block.parent.as_deref(), Some("parent-block"));
    }

    /// Two BlockModels created via insert() have different IDs
    #[test]
    fn block_model_id_unique() {
        let dict = StubDictReader::new();
        let b1 = BlockModel::insert(
            NomtuRef::new("e1", "fetch", "verb"),
            "affine:paragraph",
            &dict,
        );
        let b2 = BlockModel::insert(
            NomtuRef::new("e2", "store", "verb"),
            "affine:paragraph",
            &dict,
        );
        assert_ne!(b1.id, b2.id, "insert() must generate unique IDs");
    }

    /// NomtuRef::new produces fields in correct order
    #[test]
    fn nomtu_ref_fields_order() {
        let r = NomtuRef::new("my-id", "my-word", "my-kind");
        assert_eq!(r.id, "my-id");
        assert_eq!(r.word, "my-word");
        assert_eq!(r.kind, "my-kind");
    }

    /// BlockMeta default has version == 1 and empty author
    #[test]
    fn block_meta_default() {
        let meta = BlockMeta::default();
        assert_eq!(meta.version, 1);
        assert_eq!(meta.author, "");
        assert_eq!(meta.created_at, 0);
        assert_eq!(meta.updated_at, 0);
    }

    /// set_slot overwrites an existing slot instead of appending a duplicate
    #[test]
    fn block_model_set_slot_overwrites() {
        let mut block = BlockModel::new("b1", NomtuRef::new("e1", "w", "verb"), "affine:paragraph");
        block.set_slot("key", crate::slot::SlotValue::Text("first".into()));
        block.set_slot("key", crate::slot::SlotValue::Text("second".into()));
        // Only one entry for "key"
        let count = block.slots.iter().filter(|(k, _)| k == "key").count();
        assert_eq!(count, 1);
        assert_eq!(
            block.get_slot("key").and_then(|v| v.as_text()),
            Some("second")
        );
    }

    // ── wave AG-8: additional block_model tests ──────────────────────────────

    #[test]
    fn block_model_eq_and_hash_consistent() {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};
        let r1 = NomtuRef::new("id1", "fetch", "verb");
        let r2 = NomtuRef::new("id1", "fetch", "verb");
        assert_eq!(r1, r2);
        let mut h1 = DefaultHasher::new();
        let mut h2 = DefaultHasher::new();
        r1.hash(&mut h1);
        r2.hash(&mut h2);
        assert_eq!(h1.finish(), h2.finish());
    }

    #[test]
    fn block_model_nomturef_eq_by_value() {
        let a = NomtuRef::new("same-id", "plan", "concept");
        let b = NomtuRef::new("same-id", "plan", "concept");
        assert_eq!(a, b);
        let c = NomtuRef::new("other-id", "plan", "concept");
        assert_ne!(a, c);
    }

    #[test]
    fn block_model_nomturef_hash_stable() {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};
        let r = NomtuRef::new("stable", "word", "kind");
        let mut h1 = DefaultHasher::new();
        let mut h2 = DefaultHasher::new();
        r.hash(&mut h1);
        r.hash(&mut h2);
        assert_eq!(h1.finish(), h2.finish());
    }

    #[test]
    fn block_model_new_assigns_unique_ids() {
        let dict = StubDictReader::new();
        let b1 = BlockModel::insert(NomtuRef::new("e1", "fetch", "verb"), "affine:paragraph", &dict);
        let b2 = BlockModel::insert(NomtuRef::new("e2", "store", "verb"), "affine:paragraph", &dict);
        assert_ne!(b1.id, b2.id, "each insert() must produce a unique id");
    }

    #[test]
    fn block_model_kind_str_nonempty() {
        let entity = NomtuRef::new("e1", "compose", "verb");
        assert!(!entity.kind.is_empty());
        let block = BlockModel::new("b1", entity, "affine:paragraph");
        assert!(!block.entity.kind.is_empty());
    }

    #[test]
    fn block_model_entity_ref_present() {
        let entity = NomtuRef::new("eid", "plan", "concept");
        let block = BlockModel::new("b1", entity, "affine:note");
        // entity must have non-empty id and word
        assert!(!block.entity.id.is_empty());
        assert!(!block.entity.word.is_empty());
    }

    #[test]
    fn block_model_default_kind_is_known() {
        use crate::dict_reader::DictReader;
        let reader = StubDictReader::with_kinds(&["verb", "concept", "noun"]);
        let entity = NomtuRef::new("e1", "render", "verb");
        let block = BlockModel::insert(entity, "affine:paragraph", &reader);
        // kind should be a known kind in the reader
        assert!(reader.is_known_kind(&block.entity.kind));
    }

    #[test]
    fn block_model_serialization_round_trip() {
        let entity = NomtuRef::new("ser-01", "serialize", "verb");
        let block = BlockModel::new("blk-ser", entity, "affine:paragraph");
        let json = serde_json::to_string(&block).expect("serialize must not fail");
        let restored: BlockModel = serde_json::from_str(&json).expect("deserialize must not fail");
        assert_eq!(restored.id, "blk-ser");
        assert_eq!(restored.entity.id, "ser-01");
        assert_eq!(restored.entity.word, "serialize");
        assert_eq!(restored.entity.kind, "verb");
        assert_eq!(restored.flavour, "affine:paragraph");
    }

    #[test]
    fn block_model_clone_equal_to_original() {
        let entity = NomtuRef::new("e-clone", "clone", "verb");
        let block = BlockModel::new("b-clone", entity, "affine:note");
        let cloned = block.clone();
        assert_eq!(cloned.id, block.id);
        assert_eq!(cloned.entity.id, block.entity.id);
        assert_eq!(cloned.entity.word, block.entity.word);
        assert_eq!(cloned.flavour, block.flavour);
    }

    #[test]
    fn block_model_nomturef_word_different_not_equal() {
        let r1 = NomtuRef::new("id", "fetch", "verb");
        let r2 = NomtuRef::new("id", "store", "verb");
        assert_ne!(r1, r2);
    }

    #[test]
    fn block_model_multiple_slots_stored() {
        let mut block = BlockModel::new("b1", NomtuRef::new("e1", "w", "verb"), "affine:paragraph");
        block.set_slot("name", crate::slot::SlotValue::Text("Alice".into()));
        block.set_slot("score", crate::slot::SlotValue::Number(42.0));
        assert_eq!(block.slots.len(), 2);
        assert!(block.get_slot("name").is_some());
        assert!(block.get_slot("score").is_some());
        assert!(block.get_slot("missing").is_none());
    }

    #[test]
    fn block_model_flavour_stored_exactly() {
        let entity = NomtuRef::new("e1", "w", "verb");
        let block = BlockModel::new("b1", entity, "affine:code");
        assert_eq!(block.flavour, "affine:code");
    }

    // ── wave AB: serialization + clone round-trip tests ─────────────────────

    /// NomtuRef value is preserved exactly after clone.
    #[test]
    fn nomtu_ref_value_preserved_after_clone() {
        let r = NomtuRef::new("orig-id", "orig-word", "orig-kind");
        let r2 = r.clone();
        assert_eq!(r2.id, "orig-id");
        assert_eq!(r2.word, "orig-word");
        assert_eq!(r2.kind, "orig-kind");
    }

    /// block_kind (entity.kind) string is preserved after BlockModel clone.
    #[test]
    fn block_kind_preserved_after_clone() {
        let entity = NomtuRef::new("e1", "render", "verb");
        let block = BlockModel::new("b1", entity, "affine:paragraph");
        let cloned = block.clone();
        assert_eq!(cloned.entity.kind, "verb");
    }

    /// position-like fields (meta timestamps used as position surrogates) preserved after clone.
    #[test]
    fn block_meta_timestamps_preserved_after_clone() {
        let mut block = BlockModel::new("b1", NomtuRef::new("e1", "w", "verb"), "affine:paragraph");
        block.meta.created_at = 1_000_000;
        block.meta.updated_at = 2_000_000;
        let cloned = block.clone();
        assert_eq!(cloned.meta.created_at, 1_000_000);
        assert_eq!(cloned.meta.updated_at, 2_000_000);
    }

    /// BlockMeta version is preserved after clone.
    #[test]
    fn block_meta_version_preserved_after_clone() {
        let mut block = BlockModel::new("b1", NomtuRef::new("e1", "w", "verb"), "affine:paragraph");
        block.meta.version = 7;
        let cloned = block.clone();
        assert_eq!(cloned.meta.version, 7);
    }

    /// BlockMeta author is preserved after clone.
    #[test]
    fn block_meta_author_preserved_after_clone() {
        let mut block = BlockModel::new("b1", NomtuRef::new("e1", "w", "verb"), "affine:paragraph");
        block.meta.author = "alice".to_string();
        let cloned = block.clone();
        assert_eq!(cloned.meta.author, "alice");
    }

    /// Slots list is preserved after BlockModel clone.
    #[test]
    fn block_slots_preserved_after_clone() {
        let mut block = BlockModel::new("b1", NomtuRef::new("e1", "w", "verb"), "affine:paragraph");
        block.set_slot("title", crate::slot::SlotValue::Text("My Title".into()));
        let cloned = block.clone();
        assert_eq!(cloned.slots.len(), 1);
        assert_eq!(
            cloned.get_slot("title").and_then(|v| v.as_text()),
            Some("My Title")
        );
    }

    /// Children list is preserved after BlockModel clone.
    #[test]
    fn block_children_preserved_after_clone() {
        let mut block = BlockModel::new("b1", NomtuRef::new("e1", "w", "verb"), "affine:note");
        block.children.push("child-a".to_string());
        block.children.push("child-b".to_string());
        let cloned = block.clone();
        assert_eq!(cloned.children, vec!["child-a", "child-b"]);
    }

    /// serde_json round-trip preserves all BlockModel fields.
    #[test]
    fn block_model_json_round_trip_all_fields() {
        let mut block = BlockModel::new(
            "rtrip-1",
            NomtuRef::new("rtrip-e", "roundtrip", "concept"),
            "affine:note",
        );
        block.meta.author = "bob".to_string();
        block.meta.version = 3;
        block.children.push("child-x".to_string());
        block.parent = Some("parent-x".to_string());
        let json = serde_json::to_string(&block).expect("serialize");
        let restored: BlockModel = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(restored.id, "rtrip-1");
        assert_eq!(restored.entity.id, "rtrip-e");
        assert_eq!(restored.entity.word, "roundtrip");
        assert_eq!(restored.entity.kind, "concept");
        assert_eq!(restored.flavour, "affine:note");
        assert_eq!(restored.meta.author, "bob");
        assert_eq!(restored.meta.version, 3);
        assert_eq!(restored.children, vec!["child-x"]);
        assert_eq!(restored.parent.as_deref(), Some("parent-x"));
    }

    /// Modifying a clone does not affect the original.
    #[test]
    fn block_model_clone_independent() {
        let block = BlockModel::new("b1", NomtuRef::new("e1", "w", "verb"), "affine:paragraph");
        let mut cloned = block.clone();
        cloned.flavour = "affine:note".to_string();
        assert_eq!(block.flavour, "affine:paragraph");
        assert_eq!(cloned.flavour, "affine:note");
    }

    /// NomtuRef word field round-trips through serde_json.
    #[test]
    fn nomtu_ref_word_survives_serde_round_trip() {
        let r = NomtuRef::new("id-77", "long_word_example", "noun");
        let json = serde_json::to_string(&r).expect("serialize");
        let r2: NomtuRef = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(r2.word, "long_word_example");
    }

    /// BlockModel with no slots produces empty JSON slots array on round-trip.
    #[test]
    fn block_model_no_slots_round_trip() {
        let block = BlockModel::new("no-slots", NomtuRef::new("e1", "w", "verb"), "affine:paragraph");
        let json = serde_json::to_string(&block).expect("serialize");
        let restored: BlockModel = serde_json::from_str(&json).expect("deserialize");
        assert!(restored.slots.is_empty());
    }

    /// BlockModel parent None is preserved through serde_json round-trip.
    #[test]
    fn block_model_parent_none_round_trip() {
        let block = BlockModel::new("b-no-parent", NomtuRef::new("e1", "w", "verb"), "affine:note");
        let json = serde_json::to_string(&block).expect("serialize");
        let restored: BlockModel = serde_json::from_str(&json).expect("deserialize");
        assert!(restored.parent.is_none());
    }

    /// BlockMeta fields all zero after default, version == 1.
    #[test]
    fn block_meta_default_all_fields() {
        let meta = BlockMeta::default();
        assert_eq!(meta.created_at, 0);
        assert_eq!(meta.updated_at, 0);
        assert!(meta.author.is_empty());
        assert_eq!(meta.version, 1);
    }

    /// NomtuRef kind field survives clone.
    #[test]
    fn nomtu_ref_kind_survives_clone() {
        let r = NomtuRef::new("k-test", "word", "very-specific-kind");
        let r2 = r.clone();
        assert_eq!(r2.kind, "very-specific-kind");
    }

    /// BlockModel id field is the same string after serde_json round-trip.
    #[test]
    fn block_model_id_round_trip() {
        let block = BlockModel::new("exact-id-42", NomtuRef::new("e1", "w", "verb"), "affine:paragraph");
        let json = serde_json::to_string(&block).expect("serialize");
        let restored: BlockModel = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(restored.id, "exact-id-42");
    }
}
