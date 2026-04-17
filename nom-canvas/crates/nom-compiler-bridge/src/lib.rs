#![deny(unsafe_code)]
pub mod shared;
pub mod sqlite_dict;
pub mod ui_tier;
pub mod interactive_tier;
pub mod background_tier;
pub mod adapters;

pub use shared::{SharedState, PipelineOutput, GrammarKind};
pub use sqlite_dict::SqliteDictReader;

/// Bridge state — central coordinator for all nom-compiler access from nom-canvas
pub struct BridgeState {
    pub shared: std::sync::Arc<SharedState>,
}

impl BridgeState {
    pub fn new(dict_path: impl Into<String>, grammar_path: impl Into<String>) -> Self {
        Self {
            shared: std::sync::Arc::new(SharedState::new(dict_path, grammar_path)),
        }
    }

    pub fn sqlite_dict_reader(&self) -> impl nom_blocks::dict_reader::DictReader + '_ {
        #[cfg(feature = "compiler")]
        return SqliteDictReader::new(self.shared.clone());
        #[cfg(not(feature = "compiler"))]
        return SqliteDictReader::new_stub();
    }
}
