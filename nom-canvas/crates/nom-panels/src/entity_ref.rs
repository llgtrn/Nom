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
}
