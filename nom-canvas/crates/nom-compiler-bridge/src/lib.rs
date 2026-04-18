#![deny(unsafe_code)]
#[cfg(feature = "compiler")]
pub mod candle_adapter;
#[cfg(feature = "compiler")]
pub use candle_adapter::{BackendDevice, CandleAdapter, ModelConfig};
pub mod adapters;
pub mod background_tier;
pub mod lsp_server;
pub use lsp_server::{
    AuthoringEvent, AuthoringProtocol, LspLoopState, LspRequest, LspResponse, LspServerLoop,
    LspTransport, dispatch_lsp_request,
};
pub mod benchmarks;
#[cfg(feature = "compiler")]
pub mod dictwriter;
pub mod interactive_tier;
pub mod shared;
pub mod sqlite_dict;
pub mod ui_tier;

pub use adapters::lsp::CompilerLspProvider;
pub use background_tier::BackgroundTierOps;
#[cfg(feature = "compiler")]
pub use dictwriter::DictWriter;
pub use interactive_tier::InteractiveTierOps;
pub use shared::{GrammarKind, KindStatus, PipelineOutput, SharedState};
pub use sqlite_dict::SqliteDictReader;
pub use ui_tier::UiTierOps;

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

    pub fn ui_tier(&self) -> UiTierOps<'_> {
        UiTierOps::new(&self.shared)
    }

    pub fn interactive_tier(&self) -> InteractiveTierOps<'_> {
        InteractiveTierOps::new(&self.shared)
    }

    pub fn background_tier(&self) -> BackgroundTierOps {
        BackgroundTierOps::new(self.shared.clone())
    }

    pub fn lsp_provider(&self) -> CompilerLspProvider {
        CompilerLspProvider::new(self.shared.clone())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bridge_state_creates_shared() {
        let bridge = BridgeState::new("test.db", "test.grammar");
        // Arc strong count is 1 (only bridge holds it)
        assert_eq!(std::sync::Arc::strong_count(&bridge.shared), 1);
    }

    #[test]
    fn shared_state_has_paths() {
        let state = SharedState::new("my_dict.db", "my_grammar.db");
        assert_eq!(state.dict_path, "my_dict.db");
        assert_eq!(state.grammar_path, "my_grammar.db");
    }

    #[test]
    fn pipeline_output_fields() {
        let output = PipelineOutput {
            source_hash: 42,
            grammar_version: 7,
            output_json: "{\"ok\":true}".into(),
        };
        assert_eq!(output.source_hash, 42);
        assert_eq!(output.grammar_version, 7);
        assert_eq!(output.output_json, "{\"ok\":true}");
    }

    #[test]
    fn grammar_kind_variants() {
        let kind = GrammarKind {
            name: "verb".into(),
            description: "an action word".into(),
            status: KindStatus::Transient,
        };
        assert_eq!(kind.name, "verb");
        assert_eq!(kind.description, "an action word");
    }

    #[test]
    #[cfg(not(feature = "compiler"))]
    fn sqlite_dict_reader_stub() {
        use nom_blocks::dict_reader::DictReader;
        let reader = SqliteDictReader::new_stub();
        assert!(!reader.is_known_kind("verb"));
        assert!(reader.clause_shapes_for("verb").is_empty());
        assert!(reader.lookup_entity("run", "verb").is_none());
    }

    #[test]
    fn bridge_state_ui_tier_available() {
        let bridge = BridgeState::new("test.db", "test.grammar");
        let ops = bridge.ui_tier();
        // UiTierOps is available — is_known_kind returns false on empty cache
        assert!(!ops.is_known_kind("verb"));
    }

    #[test]
    fn bridge_state_interactive_tier_available() {
        let bridge = BridgeState::new("test.db", "test.grammar");
        let ops = bridge.interactive_tier();
        // InteractiveTierOps is available — hover_info returns something without panic
        let _ = ops.hover_info("define");
    }

    #[test]
    fn bridge_state_background_tier_available() {
        let bridge = BridgeState::new("test.db", "test.grammar");
        let _ops = bridge.background_tier();
        // BackgroundTierOps constructed without panic
    }

    #[test]
    fn bridge_state_lsp_provider_available() {
        use nom_editor::lsp_bridge::LspProvider;
        let bridge = BridgeState::new("test.db", "test.grammar");
        let provider = bridge.lsp_provider();
        // lsp_provider() returns a CompilerLspProvider — hover on an empty path returns None
        let result = provider.hover(std::path::Path::new(""), 0);
        assert!(result.is_none());
    }

    #[test]
    fn bridge_state_two_instances_independent() {
        let bridge_a = BridgeState::new("a.db", "a.grammar");
        let bridge_b = BridgeState::new("b.db", "b.grammar");
        // Each bridge has its own Arc — strong count on each is 1
        assert_eq!(std::sync::Arc::strong_count(&bridge_a.shared), 1);
        assert_eq!(std::sync::Arc::strong_count(&bridge_b.shared), 1);
        // Their dict paths differ
        assert_ne!(bridge_a.shared.dict_path, bridge_b.shared.dict_path);
    }

    #[test]
    fn grammar_kind_clone() {
        let kind = GrammarKind {
            name: "transform".into(),
            description: "converts one form to another".into(),
            status: KindStatus::Transient,
        };
        let cloned = kind.clone();
        assert_eq!(cloned.name, kind.name);
        assert_eq!(cloned.description, kind.description);
    }

    #[test]
    fn grammar_kind_debug_format() {
        let kind = GrammarKind {
            name: "emit".into(),
            description: "outputs a value".into(),
            status: KindStatus::Transient,
        };
        let debug_str = format!("{:?}", kind);
        // Debug impl produces a non-empty string and doesn't panic
        assert!(!debug_str.is_empty());
        assert!(debug_str.contains("emit"));
    }

    #[test]
    fn pipeline_output_json_field() {
        let output = PipelineOutput {
            source_hash: 1,
            grammar_version: 2,
            output_json: r#"{"status":"ok","count":3}"#.into(),
        };
        assert_eq!(output.output_json, r#"{"status":"ok","count":3}"#);
    }

    #[test]
    fn pipeline_output_clone() {
        let output = PipelineOutput {
            source_hash: 99,
            grammar_version: 5,
            output_json: "cloned".into(),
        };
        let cloned = output.clone();
        assert_eq!(cloned.source_hash, output.source_hash);
        assert_eq!(cloned.grammar_version, output.grammar_version);
        assert_eq!(cloned.output_json, output.output_json);
    }
}
