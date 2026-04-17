#![deny(unsafe_code)]
use serde::{Deserialize, Serialize};
use crate::block_model::NomtuRef;
pub const FLAVOUR: &str = "nom:compose-audio";
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AudioBlock { pub entity: NomtuRef, pub artifact_hash: [u8; 32], pub duration_ms: u64, pub codec: String }
impl AudioBlock {
    pub fn new(entity: NomtuRef, artifact_hash: [u8; 32], codec: impl Into<String>) -> Self {
        Self { entity, artifact_hash, duration_ms: 0, codec: codec.into() }
    }
    pub fn flavour() -> &'static str { FLAVOUR }
}
