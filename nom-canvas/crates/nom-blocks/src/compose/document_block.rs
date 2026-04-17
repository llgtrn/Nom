#![deny(unsafe_code)]
use crate::block_model::NomtuRef;
use serde::{Deserialize, Serialize};
pub const FLAVOUR: &str = "nom:compose-document";
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DocumentBlock {
    pub entity: NomtuRef,
    pub artifact_hash: [u8; 32],
    pub page_count: u32,
    pub mime: String,
}
impl DocumentBlock {
    pub fn new(entity: NomtuRef, artifact_hash: [u8; 32], mime: impl Into<String>) -> Self {
        Self {
            entity,
            artifact_hash,
            page_count: 0,
            mime: mime.into(),
        }
    }
    pub fn flavour() -> &'static str {
        FLAVOUR
    }
}
