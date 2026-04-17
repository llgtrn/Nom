use thiserror::Error;

use crate::flavour::Flavour;

/// Structural role of a block within the document tree.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Role {
    /// Carries user content (text, code, media, …).
    Content,
    /// Groups child blocks (e.g. a note container).
    Hub,
    /// Top-level document root.
    Root,
}

/// Static descriptor that declares which parent/child flavours a block accepts.
#[derive(Debug, Clone, Copy)]
pub struct BlockSchema {
    pub flavour: Flavour,
    pub version: u32,
    pub role: Role,
    /// Allowed parent flavours. Empty slice means any parent is permitted (Root).
    pub parents: &'static [Flavour],
    /// Allowed child flavours. Empty slice means no children (leaf block).
    pub children: &'static [Flavour],
}

/// Errors produced when schema constraints are violated.
#[derive(Debug, Error, PartialEq, Eq)]
pub enum SchemaError {
    #[error("parent flavour '{0}' is not allowed by schema '{1}'")]
    ParentNotAllowed(&'static str, &'static str),
    #[error("child flavour '{0}' is not allowed by schema '{1}'")]
    ChildNotAllowed(&'static str, &'static str),
}

/// Check that `parent_flavour` is an allowed parent for this schema.
///
/// Root blocks have an empty `parents` slice and accept any parent (no rejection).
pub fn validate_parent(schema: &BlockSchema, parent_flavour: Flavour) -> Result<(), SchemaError> {
    if schema.role == Role::Root || schema.parents.is_empty() {
        return Ok(());
    }
    if schema.parents.contains(&parent_flavour) {
        Ok(())
    } else {
        Err(SchemaError::ParentNotAllowed(parent_flavour, schema.flavour))
    }
}

/// Check that `child_flavour` is an allowed child for this schema.
pub fn validate_child(schema: &BlockSchema, child_flavour: Flavour) -> Result<(), SchemaError> {
    if schema.children.contains(&child_flavour) {
        Ok(())
    } else {
        Err(SchemaError::ChildNotAllowed(child_flavour, schema.flavour))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::flavour::{NOTE, PROSE};

    const TEST_SCHEMA: BlockSchema = BlockSchema {
        flavour: PROSE,
        version: 1,
        role: Role::Content,
        parents: &[NOTE],
        children: &[],
    };

    #[test]
    fn valid_parent_accepted() {
        assert!(validate_parent(&TEST_SCHEMA, NOTE).is_ok());
    }

    #[test]
    fn invalid_parent_rejected() {
        let err = validate_parent(&TEST_SCHEMA, "nom:unknown").unwrap_err();
        assert!(matches!(err, SchemaError::ParentNotAllowed(_, _)));
    }

    #[test]
    fn valid_child_accepted() {
        const HUB: BlockSchema = BlockSchema {
            flavour: NOTE,
            version: 1,
            role: Role::Hub,
            parents: &[],
            children: &[PROSE],
        };
        assert!(validate_child(&HUB, PROSE).is_ok());
    }

    #[test]
    fn leaf_schema_rejects_any_child() {
        let err = validate_child(&TEST_SCHEMA, NOTE).unwrap_err();
        assert!(matches!(err, SchemaError::ChildNotAllowed(_, _)));
    }
}
