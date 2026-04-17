#![deny(unsafe_code)]
use crate::block_model::NomtuRef;
use serde::{Deserialize, Serialize};
pub const FLAVOUR: &str = "nom:compose-app";
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AppBlock {
    pub entity: NomtuRef,
    pub artifact_hash: [u8; 32],
    pub target_platform: String,
    pub deploy_url: Option<String>,
}
impl AppBlock {
    pub fn new(
        entity: NomtuRef,
        artifact_hash: [u8; 32],
        target_platform: impl Into<String>,
    ) -> Self {
        Self {
            entity,
            artifact_hash,
            target_platform: target_platform.into(),
            deploy_url: None,
        }
    }
    pub fn flavour() -> &'static str {
        FLAVOUR
    }
}
