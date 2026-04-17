//! Preview block types for Nom compose outputs (video/image/document/data/app/audio).
#![deny(unsafe_code)]

pub mod video_block;
pub mod image_block;
pub mod document_block;
pub mod data_block;
pub mod app_block;
pub mod audio_block;

// Re-export the 6 schema functions + props types.
pub use video_block::{VideoBlockProps, video_block_schema};
pub use image_block::{ImageBlockProps, image_block_schema};
pub use document_block::{DocumentBlockProps, document_block_schema};
pub use data_block::{DataBlockProps, data_block_schema};
pub use app_block::{AppBlockProps, AppKind, app_block_schema};
pub use audio_block::{AudioBlockProps, audio_block_schema};

// Local flavour constants for compose previews (added to the NomCanvas namespace).
pub const COMPOSE_VIDEO: &str = "nom:compose:video";
pub const COMPOSE_IMAGE: &str = "nom:compose:image";
pub const COMPOSE_DOCUMENT: &str = "nom:compose:document";
pub const COMPOSE_DATA: &str = "nom:compose:data";
pub const COMPOSE_APP: &str = "nom:compose:app";
pub const COMPOSE_AUDIO: &str = "nom:compose:audio";
