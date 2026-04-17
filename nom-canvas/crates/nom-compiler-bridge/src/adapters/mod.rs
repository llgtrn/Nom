#![deny(unsafe_code)]
pub mod highlight;
pub mod lsp;
pub mod completion;
pub mod score;

use crate::interactive_tier::InteractiveTierOps;

/// LspAdapter — wraps the interactive tier to provide LSP-like operations
pub struct LspAdapter<'a> {
    interactive: InteractiveTierOps<'a>,
}

impl<'a> LspAdapter<'a> {
    pub fn new(interactive: InteractiveTierOps<'a>) -> Self {
        Self { interactive }
    }

    /// Provide a completion candidate for a given prefix
    pub fn complete(&self, prefix: &str) -> Vec<String> {
        vec![format!("{prefix}_completion")]
    }

    /// Provide hover documentation for a word
    pub fn hover(&self, word: &str) -> Option<String> {
        self.interactive.hover_info(word)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::shared::SharedState;

    #[test]
    fn lsp_adapter_complete() {
        let state = SharedState::new("a.db", "b.db");
        let ops = InteractiveTierOps::new(&state);
        let adapter = LspAdapter::new(ops);
        let completions = adapter.complete("run");
        assert_eq!(completions, vec!["run_completion"]);
    }

    #[test]
    fn lsp_adapter_hover() {
        let state = SharedState::new("a.db", "b.db");
        let ops = InteractiveTierOps::new(&state);
        let adapter = LspAdapter::new(ops);
        let hover = adapter.hover("define");
        assert_eq!(hover, Some("nomtu: define".to_string()));
    }
}
