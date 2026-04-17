#![deny(unsafe_code)]
use crate::block_model::NomtuRef;
use serde::{Deserialize, Serialize};
pub const FLAVOUR: &str = "nom:compose-data";
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ColumnSpec {
    pub name: String,
    pub col_type: String,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DataBlock {
    pub entity: NomtuRef,
    pub artifact_hash: [u8; 32],
    pub row_count: u64,
    pub schema: Vec<ColumnSpec>,
}
impl DataBlock {
    pub fn new(entity: NomtuRef, artifact_hash: [u8; 32]) -> Self {
        Self {
            entity,
            artifact_hash,
            row_count: 0,
            schema: Vec::new(),
        }
    }
    pub fn flavour() -> &'static str {
        FLAVOUR
    }
}
