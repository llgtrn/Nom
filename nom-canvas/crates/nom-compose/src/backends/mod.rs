pub mod document;
pub mod video;
pub mod image;
pub mod audio;
pub mod data;
pub mod app;
pub mod code_exec;
pub mod web_screen;
pub mod workflow;
pub mod scenario;
pub mod rag_query;
pub mod transform;
pub mod embed_gen;
pub mod render;
pub mod export;
pub mod pipeline;
pub mod data_frame;
pub mod data_query;
pub mod presentation;
pub mod app_bundle;
pub mod ad_creative;
pub mod data_extract;
pub mod mesh;
pub mod storyboard;
pub mod native_screen;
pub mod mobile_screen;

/// Uniform error-wrapping return type for safe compose wrappers.
pub type ComposeResult = Result<(), String>;
