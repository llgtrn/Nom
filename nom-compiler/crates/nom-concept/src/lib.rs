//! Tier-1 (`.nomtu`) and Tier-2 (`.nom`) file-format types per
//! `research/language-analysis/08-layered-concept-component-architecture.md` §6.
//!
//! `.nomtu` = multi-entity DB2 container (small scope).
//! `.nom`   = multi-concept DB1 container (big scope).
//!
//! This crate defines the AST + (eventual) parser. Implementation lands in
//! follow-up commits: lexer integration, parse, serialize, DB sync.

use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Closed kind set per doc 08 §8.1.
pub const KINDS: &[&str] = &[
    "function", "module", "concept", "screen", "data", "event", "media",
];

/// `.nom` file: 1..N concept declarations.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NomFile {
    pub concepts: Vec<ConceptDecl>,
}

/// `.nomtu` file: 1..N entity declarations and/or composition declarations.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NomtuFile {
    pub items: Vec<NomtuItem>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum NomtuItem {
    Entity(EntityDecl),
    Composition(CompositionDecl),
}

/// One concept (one DB1 row).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ConceptDecl {
    pub name: String,
    pub intent: String,
    pub index: Vec<IndexClause>,
    pub exposes: Vec<String>,
    pub acceptance: Vec<String>,
    pub objectives: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum IndexClause {
    Uses(Vec<EntityRef>),
    Extends { base: String, change_set: ChangeSet },
}

#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct ChangeSet {
    pub adding: Vec<EntityRef>,
    pub removing: Vec<EntityRef>,
}

/// One DB2 entity declared inline in a `.nomtu`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EntityDecl {
    pub kind: String,
    pub word: String,
    pub signature: String,
    pub contracts: Vec<ContractClause>,
}

/// A composition emitted by a `.nomtu` (one extra DB2 row).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CompositionDecl {
    pub word: String,
    pub composes: Vec<EntityRef>,
    pub glue: Option<String>,
    pub contracts: Vec<ContractClause>,
}

/// Reference to an entity. After first build the resolver writes back `hash`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EntityRef {
    pub kind: Option<String>,
    pub word: String,
    pub hash: Option<String>,
    pub matching: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ContractClause {
    Requires(String),
    Ensures(String),
}

#[derive(Debug, Error)]
pub enum ConceptError {
    #[error("parser not yet implemented for `.nom` files (doc 08 §6.2)")]
    NomParserUnimplemented,
    #[error("parser not yet implemented for `.nomtu` files (doc 08 §6.1)")]
    NomtuParserUnimplemented,
    #[error("unknown kind `{0}`; closed set per doc 08 §8.1: {KINDS:?}")]
    UnknownKind(String),
}

/// Parse a `.nom` source text into a [`NomFile`].
///
/// Stub: lexer integration lands in a follow-up commit.
pub fn parse_nom(_src: &str) -> Result<NomFile, ConceptError> {
    Err(ConceptError::NomParserUnimplemented)
}

/// Parse a `.nomtu` source text into a [`NomtuFile`].
///
/// Stub: lexer integration lands in a follow-up commit.
pub fn parse_nomtu(_src: &str) -> Result<NomtuFile, ConceptError> {
    Err(ConceptError::NomtuParserUnimplemented)
}

/// True if `kind` is in the closed set per doc 08 §8.1.
pub fn is_known_kind(kind: &str) -> bool {
    KINDS.contains(&kind)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn closed_kind_set_has_seven_members() {
        assert_eq!(KINDS.len(), 7);
        for k in ["function", "module", "concept", "screen", "data", "event", "media"] {
            assert!(is_known_kind(k));
        }
        assert!(!is_known_kind("class"));
        assert!(!is_known_kind("trait"));
    }

    #[test]
    fn ast_constructs_and_round_trips_through_json() {
        let entity = EntityDecl {
            kind: "function".to_string(),
            word: "validate_token_jwt_hmac_sha256".to_string(),
            signature: "given a token of text, returns yes or no".to_string(),
            contracts: vec![
                ContractClause::Requires("the token is non-empty".to_string()),
                ContractClause::Ensures("the result reflects whether the signature verifies".to_string()),
            ],
        };
        let nomtu = NomtuFile { items: vec![NomtuItem::Entity(entity.clone())] };
        let json = serde_json::to_string(&nomtu).unwrap();
        let back: NomtuFile = serde_json::from_str(&json).unwrap();
        assert_eq!(nomtu, back);
    }

    #[test]
    fn concept_with_index_round_trips() {
        let concept = ConceptDecl {
            name: "concept_authentication_jwt_basic".to_string(),
            intent: "let users with valid tokens reach the dashboard".to_string(),
            index: vec![IndexClause::Uses(vec![EntityRef {
                kind: Some("module".to_string()),
                word: "auth_jwt_session_compose".to_string(),
                hash: Some("a1b2c3d4".to_string()),
                matching: None,
            }])],
            exposes: vec!["auth_jwt_session_compose".to_string()],
            acceptance: vec![
                "users with valid tokens reach the dashboard within 200 ms".to_string(),
            ],
            objectives: vec!["security".to_string(), "speed".to_string()],
        };
        let nom = NomFile { concepts: vec![concept] };
        let json = serde_json::to_string(&nom).unwrap();
        let back: NomFile = serde_json::from_str(&json).unwrap();
        assert_eq!(nom, back);
    }

    #[test]
    fn parse_stubs_return_unimplemented() {
        assert!(matches!(parse_nom(""), Err(ConceptError::NomParserUnimplemented)));
        assert!(matches!(parse_nomtu(""), Err(ConceptError::NomtuParserUnimplemented)));
    }
}
