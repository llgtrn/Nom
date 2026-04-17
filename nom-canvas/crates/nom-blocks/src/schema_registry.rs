//! Runtime block-schema registry.
//!
//! Looks up a `BlockSchema` by flavour string + enforces parent/child
//! relationships.  Seeded with the 9 built-in block types on construction.
#![deny(unsafe_code)]

use std::collections::HashMap;

use crate::block_schema::{BlockSchema, SchemaError};
use crate::flavour::Flavour;

pub struct SchemaRegistry {
    schemas: HashMap<Flavour, BlockSchema>,
}

impl SchemaRegistry {
    pub fn new() -> Self {
        let mut s = Self { schemas: HashMap::new() };
        s.register_default_schemas();
        s
    }

    fn register_default_schemas(&mut self) {
        self.insert(crate::prose::prose_schema());
        self.insert(crate::nomx::nomx_schema());
        self.insert(crate::media::image_schema());
        self.insert(crate::media::attachment_schema());
        self.insert(crate::graph_node::graph_node_schema());
        self.insert(crate::drawing::drawing_schema());
        self.insert(crate::table::table_schema());
        self.insert(crate::embed::embed_schema());
    }

    pub fn insert(&mut self, schema: BlockSchema) {
        self.schemas.insert(schema.flavour, schema);
    }

    pub fn get(&self, flavour: Flavour) -> Option<&BlockSchema> {
        self.schemas.get(flavour)
    }

    /// Try to insert a child of `child_flavour` under `parent_flavour`.
    /// Returns Ok when both schemas exist AND the parent's schema lists the
    /// child's flavour in its `children` array.
    pub fn validate_insertion(
        &self,
        parent_flavour: Flavour,
        child_flavour: Flavour,
    ) -> Result<(), SchemaError> {
        let parent_schema = self
            .get(parent_flavour)
            .ok_or(SchemaError::UnknownFlavour(parent_flavour))?;
        let _ = self
            .get(child_flavour)
            .ok_or(SchemaError::UnknownFlavour(child_flavour))?;
        crate::block_schema::validate_child(parent_schema, child_flavour)
    }

    pub fn len(&self) -> usize {
        self.schemas.len()
    }

    pub fn is_empty(&self) -> bool {
        self.schemas.is_empty()
    }

    pub fn all_flavours(&self) -> Vec<Flavour> {
        self.schemas.keys().copied().collect()
    }
}

impl Default for SchemaRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::block_schema::{BlockSchema, Role};
    use crate::flavour::{
        DRAWING, EMBED, GRAPH_NODE, MEDIA_ATTACHMENT, MEDIA_IMAGE, NOMX, PROSE, TABLE,
    };

    // ── seeding ──────────────────────────────────────────────────────────────

    #[test]
    fn new_seeds_8_built_in_schemas() {
        let reg = SchemaRegistry::new();
        assert_eq!(reg.len(), 8);
    }

    // ── get ───────────────────────────────────────────────────────────────────

    #[test]
    fn get_prose_returns_some() {
        let reg = SchemaRegistry::new();
        assert!(reg.get(PROSE).is_some());
        assert_eq!(reg.get(PROSE).unwrap().flavour, PROSE);
    }

    #[test]
    fn get_hits_all_8_built_ins() {
        let reg = SchemaRegistry::new();
        for flavour in &[
            PROSE,
            NOMX,
            MEDIA_IMAGE,
            MEDIA_ATTACHMENT,
            GRAPH_NODE,
            DRAWING,
            TABLE,
            EMBED,
        ] {
            assert!(
                reg.get(flavour).is_some(),
                "missing built-in flavour: {flavour}"
            );
        }
    }

    #[test]
    fn get_returns_none_for_unknown_flavour() {
        let reg = SchemaRegistry::new();
        assert!(reg.get("nom:does-not-exist").is_none());
    }

    // ── insert ────────────────────────────────────────────────────────────────

    #[test]
    fn insert_custom_schema_then_get_returns_it() {
        const CUSTOM: &str = "nom:custom-test";
        let custom = BlockSchema {
            flavour: CUSTOM,
            version: 1,
            role: Role::Content,
            parents: &[],
            children: &[],
        };
        let mut reg = SchemaRegistry::new();
        reg.insert(custom);
        assert_eq!(reg.len(), 9);
        assert!(reg.get(CUSTOM).is_some());
        assert_eq!(reg.get(CUSTOM).unwrap().flavour, CUSTOM);
    }

    #[test]
    fn insert_replaces_existing_flavour_upsert() {
        let mut reg = SchemaRegistry::new();
        let before_len = reg.len();
        // Replace prose with version 99
        let updated = BlockSchema {
            flavour: PROSE,
            version: 99,
            role: Role::Content,
            parents: &[],
            children: &[],
        };
        reg.insert(updated);
        assert_eq!(reg.len(), before_len, "len must not grow on upsert");
        assert_eq!(reg.get(PROSE).unwrap().version, 99);
    }

    // ── validate_insertion ────────────────────────────────────────────────────

    #[test]
    fn validate_insertion_ok_for_legal_combo() {
        // graph_node children = [PROSE, NOMX, MEDIA_IMAGE]
        let reg = SchemaRegistry::new();
        assert!(reg.validate_insertion(GRAPH_NODE, PROSE).is_ok());
        assert!(reg.validate_insertion(GRAPH_NODE, NOMX).is_ok());
        assert!(reg.validate_insertion(GRAPH_NODE, MEDIA_IMAGE).is_ok());
    }

    #[test]
    fn validate_insertion_rejects_unknown_parent() {
        let reg = SchemaRegistry::new();
        let err = reg
            .validate_insertion("nom:ghost", PROSE)
            .unwrap_err();
        assert!(
            matches!(err, SchemaError::UnknownFlavour("nom:ghost")),
            "unexpected error: {err:?}"
        );
    }

    #[test]
    fn validate_insertion_rejects_unknown_child() {
        let reg = SchemaRegistry::new();
        let err = reg
            .validate_insertion(GRAPH_NODE, "nom:ghost")
            .unwrap_err();
        assert!(
            matches!(err, SchemaError::UnknownFlavour("nom:ghost")),
            "unexpected error: {err:?}"
        );
    }

    #[test]
    fn validate_insertion_rejects_child_not_in_parents_children() {
        // prose is a leaf (children = []), so it cannot accept any child
        let reg = SchemaRegistry::new();
        let err = reg.validate_insertion(PROSE, NOMX).unwrap_err();
        assert!(
            matches!(err, SchemaError::ChildNotAllowed(_, _)),
            "unexpected error: {err:?}"
        );
    }

    // ── len + all_flavours ────────────────────────────────────────────────────

    #[test]
    fn len_and_all_flavours_consistent() {
        let reg = SchemaRegistry::new();
        let flavours = reg.all_flavours();
        assert_eq!(flavours.len(), reg.len());
        // all 8 built-ins must be present
        for f in &[
            PROSE,
            NOMX,
            MEDIA_IMAGE,
            MEDIA_ATTACHMENT,
            GRAPH_NODE,
            DRAWING,
            TABLE,
            EMBED,
        ] {
            assert!(flavours.contains(f), "all_flavours missing {f}");
        }
    }
}
