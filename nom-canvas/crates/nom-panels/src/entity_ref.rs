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
}
