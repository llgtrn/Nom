#![deny(unsafe_code)]
use nom_blocks::NomtuRef;

/// Typed panel metadata boundary for optional UI selections.
///
/// This is not canvas-object identity; canvas objects still carry a concrete
/// `NomtuRef`. Panels use `None` when no entity is selected.
#[derive(Clone, Debug, Default, PartialEq)]
pub enum PanelEntityRef {
    #[default]
    None,
    Nomtu(NomtuRef),
}

impl PanelEntityRef {
    pub fn nomtu(entity: NomtuRef) -> Self {
        Self::Nomtu(entity)
    }

    pub fn as_nomtu(&self) -> Option<&NomtuRef> {
        match self {
            Self::None => None,
            Self::Nomtu(entity) => Some(entity),
        }
    }

    pub fn id(&self) -> Option<&str> {
        self.as_nomtu().map(|entity| entity.id.as_str())
    }

    pub fn kind(&self) -> Option<&str> {
        self.as_nomtu().map(|entity| entity.kind.as_str())
    }

    /// Convert into `Option<NomtuRef>`, consuming self.
    pub fn into_option(self) -> Option<NomtuRef> {
        match self {
            Self::None => None,
            Self::Nomtu(entity) => Some(entity),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn panel_entity_ref_none_has_no_id() {
        let entity = PanelEntityRef::None;
        assert_eq!(entity.id(), None);
        assert_eq!(entity.kind(), None);
    }

    #[test]
    fn panel_entity_ref_wraps_nomtu_ref() {
        let entity = PanelEntityRef::nomtu(NomtuRef::new("e1", "word", "kind"));
        assert_eq!(entity.id(), Some("e1"));
        assert_eq!(entity.kind(), Some("kind"));
    }

    #[test]
    fn panel_entity_ref_equality_none() {
        assert_eq!(PanelEntityRef::None, PanelEntityRef::None);
    }

    #[test]
    fn panel_entity_ref_equality_nomtu_same() {
        let a = PanelEntityRef::nomtu(NomtuRef::new("id1", "word", "kind"));
        let b = PanelEntityRef::nomtu(NomtuRef::new("id1", "word", "kind"));
        assert_eq!(a, b);
    }

    #[test]
    fn panel_entity_ref_equality_nomtu_different_id() {
        let a = PanelEntityRef::nomtu(NomtuRef::new("id1", "word", "kind"));
        let b = PanelEntityRef::nomtu(NomtuRef::new("id2", "word", "kind"));
        assert_ne!(a, b);
    }

    #[test]
    fn panel_entity_ref_equality_none_vs_nomtu() {
        let a = PanelEntityRef::None;
        let b = PanelEntityRef::nomtu(NomtuRef::new("id1", "word", "kind"));
        assert_ne!(a, b);
    }

    #[test]
    fn panel_entity_ref_as_nomtu_none_returns_none() {
        let entity = PanelEntityRef::None;
        assert!(entity.as_nomtu().is_none());
    }

    #[test]
    fn panel_entity_ref_as_nomtu_returns_inner() {
        let inner = NomtuRef::new("e99", "myword", "mykind");
        let entity = PanelEntityRef::nomtu(inner.clone());
        assert_eq!(entity.as_nomtu(), Some(&inner));
    }

    #[test]
    fn panel_entity_ref_default_is_none() {
        let entity = PanelEntityRef::default();
        assert_eq!(entity, PanelEntityRef::None);
        assert!(entity.id().is_none());
        assert!(entity.kind().is_none());
    }

    #[test]
    fn panel_entity_ref_clone_preserves_value() {
        let a = PanelEntityRef::nomtu(NomtuRef::new("abc", "word", "Function"));
        let b = a.clone();
        assert_eq!(a, b);
        assert_eq!(b.id(), Some("abc"));
        assert_eq!(b.kind(), Some("Function"));
    }

    #[test]
    fn panel_entity_ref_id_returns_id_str() {
        let entity = PanelEntityRef::nomtu(NomtuRef::new("my-id-123", "w", "k"));
        assert_eq!(entity.id(), Some("my-id-123"));
    }

    #[test]
    fn panel_entity_ref_kind_returns_kind_str() {
        let entity = PanelEntityRef::nomtu(NomtuRef::new("e", "w", "Concept"));
        assert_eq!(entity.kind(), Some("Concept"));
    }

    // ── into_option conversion ────────────────────────────────────────────────

    #[test]
    fn panel_entity_ref_into_option_none_yields_none() {
        let entity = PanelEntityRef::None;
        assert!(entity.into_option().is_none());
    }

    #[test]
    fn panel_entity_ref_into_option_nomtu_yields_some() {
        let inner = NomtuRef::new("x1", "word", "Kind");
        let entity = PanelEntityRef::nomtu(inner.clone());
        let result = entity.into_option();
        assert!(result.is_some());
        assert_eq!(result.unwrap(), inner);
    }

    #[test]
    fn panel_entity_ref_into_option_preserves_id() {
        let entity = PanelEntityRef::nomtu(NomtuRef::new("id-abc", "w", "K"));
        let result = entity.into_option().unwrap();
        assert_eq!(result.id, "id-abc");
    }

    #[test]
    fn panel_entity_ref_into_option_preserves_kind() {
        let entity = PanelEntityRef::nomtu(NomtuRef::new("id", "w", "Concept"));
        let result = entity.into_option().unwrap();
        assert_eq!(result.kind, "Concept");
    }

    #[test]
    fn panel_entity_ref_into_option_preserves_word() {
        let entity = PanelEntityRef::nomtu(NomtuRef::new("id", "myword", "K"));
        let result = entity.into_option().unwrap();
        assert_eq!(result.word, "myword");
    }

    // ── hash consistency ──────────────────────────────────────────────────────

    #[test]
    fn nomtu_ref_hash_same_inputs_produce_same_hash() {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        let r1 = NomtuRef::new("id-1", "word", "Kind");
        let r2 = NomtuRef::new("id-1", "word", "Kind");

        let mut h1 = DefaultHasher::new();
        let mut h2 = DefaultHasher::new();
        r1.hash(&mut h1);
        r2.hash(&mut h2);
        assert_eq!(h1.finish(), h2.finish(), "same NomtuRef must produce same hash");
    }

    #[test]
    fn nomtu_ref_hash_different_id_different_hash() {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        let r1 = NomtuRef::new("id-1", "word", "Kind");
        let r2 = NomtuRef::new("id-2", "word", "Kind");

        let mut h1 = DefaultHasher::new();
        let mut h2 = DefaultHasher::new();
        r1.hash(&mut h1);
        r2.hash(&mut h2);
        assert_ne!(h1.finish(), h2.finish(), "different id must produce different hash");
    }

    #[test]
    fn nomtu_ref_hash_different_kind_different_hash() {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        let r1 = NomtuRef::new("id", "word", "KindA");
        let r2 = NomtuRef::new("id", "word", "KindB");

        let mut h1 = DefaultHasher::new();
        let mut h2 = DefaultHasher::new();
        r1.hash(&mut h1);
        r2.hash(&mut h2);
        assert_ne!(h1.finish(), h2.finish());
    }

    #[test]
    fn nomtu_ref_can_be_used_as_hashmap_key() {
        use std::collections::HashMap;
        let r = NomtuRef::new("id-42", "word", "Function");
        let mut map: HashMap<NomtuRef, u32> = HashMap::new();
        map.insert(r.clone(), 100);
        assert_eq!(map.get(&r), Some(&100));
        // Same content → same key
        let r2 = NomtuRef::new("id-42", "word", "Function");
        assert_eq!(map.get(&r2), Some(&100));
    }

    #[test]
    fn nomtu_ref_hash_stable_across_clones() {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        let r = NomtuRef::new("stable-id", "stable-word", "StableKind");
        let clone = r.clone();

        let mut h1 = DefaultHasher::new();
        let mut h2 = DefaultHasher::new();
        r.hash(&mut h1);
        clone.hash(&mut h2);
        assert_eq!(h1.finish(), h2.finish());
    }

    // ── PanelEntityRef preserves all NomtuRef fields ──────────────────────────

    #[test]
    fn panel_entity_ref_preserves_id_field() {
        let inner = NomtuRef::new("id-preserve-01", "word", "Kind");
        let e = PanelEntityRef::nomtu(inner);
        assert_eq!(e.id(), Some("id-preserve-01"));
    }

    #[test]
    fn panel_entity_ref_preserves_word_field() {
        let inner = NomtuRef::new("id", "my-special-word", "Kind");
        let e = PanelEntityRef::nomtu(inner);
        let nomtu = e.as_nomtu().unwrap();
        assert_eq!(nomtu.word, "my-special-word");
    }

    #[test]
    fn panel_entity_ref_preserves_kind_field() {
        let inner = NomtuRef::new("id", "word", "my-special-kind");
        let e = PanelEntityRef::nomtu(inner);
        assert_eq!(e.kind(), Some("my-special-kind"));
    }

    #[test]
    fn panel_entity_ref_all_three_fields_preserved() {
        let inner = NomtuRef::new("triple-id", "triple-word", "triple-kind");
        let e = PanelEntityRef::nomtu(inner);
        let nomtu = e.as_nomtu().unwrap();
        assert_eq!(nomtu.id, "triple-id");
        assert_eq!(nomtu.word, "triple-word");
        assert_eq!(nomtu.kind, "triple-kind");
    }

    #[test]
    fn panel_entity_ref_from_nomtu_ref_via_into_option() {
        let inner = NomtuRef::new("io-id", "io-word", "io-kind");
        let e = PanelEntityRef::nomtu(inner.clone());
        let result = e.into_option().unwrap();
        assert_eq!(result.id, inner.id);
        assert_eq!(result.word, inner.word);
        assert_eq!(result.kind, inner.kind);
    }

    #[test]
    fn panel_entity_ref_none_into_option_yields_none() {
        let e: PanelEntityRef = PanelEntityRef::None;
        assert!(e.into_option().is_none());
    }

    #[test]
    fn panel_entity_ref_nomtu_debug_format_non_empty() {
        let inner = NomtuRef::new("debug-id", "w", "k");
        let e = PanelEntityRef::nomtu(inner);
        let s = format!("{:?}", e);
        assert!(s.contains("debug-id"), "debug output must include the id");
    }

    #[test]
    fn panel_entity_ref_preserves_unicode_word() {
        let inner = NomtuRef::new("uni-id", "synthetize", "Concept");
        let e = PanelEntityRef::nomtu(inner);
        let nomtu = e.as_nomtu().unwrap();
        assert_eq!(nomtu.word, "synthetize");
    }
}
