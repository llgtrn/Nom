#![deny(unsafe_code)]
use serde::{Deserialize, Serialize};
use crate::block_model::NomtuRef;
pub const FLAVOUR: &str = "nom:compose-video";
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct VideoBlock { pub entity: NomtuRef, pub artifact_hash: [u8; 32], pub duration_ms: u64, pub width: u32, pub height: u32, pub progress: Option<f32> }
impl VideoBlock {
    pub fn new(entity: NomtuRef, artifact_hash: [u8; 32], width: u32, height: u32) -> Self {
        Self { entity, artifact_hash, duration_ms: 0, width, height, progress: None }
    }
    pub fn flavour() -> &'static str { FLAVOUR }
}
