#![deny(unsafe_code)]
use crate::block_model::NomtuRef;
use serde::{Deserialize, Serialize};
pub const FLAVOUR: &str = "nom:compose-image";
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ImageBlock {
    pub entity: NomtuRef,
    pub artifact_hash: [u8; 32],
    pub width: u32,
    pub height: u32,
    pub prompt_used: String,
}
impl ImageBlock {
    pub fn new(entity: NomtuRef, artifact_hash: [u8; 32], prompt_used: impl Into<String>) -> Self {
        Self {
            entity,
            artifact_hash,
            width: 0,
            height: 0,
            prompt_used: prompt_used.into(),
        }
    }
    pub fn flavour() -> &'static str {
        FLAVOUR
    }
}
