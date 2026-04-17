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

    /// Provide completion candidates for a given prefix from the grammar cache
    pub fn complete(&self, prefix: &str) -> Vec<String> {
        completion::complete_from_dict(prefix, None, self.interactive.shared())
            .into_iter()
            .map(|item| item.label)
            .collect()
    }

    /// Provide hover documentation for a word
    pub fn hover(&self, word: &str) -> Option<String> {
        self.interactive.hover_info(word)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::shared::{SharedState, GrammarKind};

    #[test]
    fn lsp_adapter_complete_empty_cache_returns_nothing() {
        let state = SharedState::new("a.db", "b.db");
        let ops = InteractiveTierOps::new(&state);
        let adapter = LspAdapter::new(ops);
        // No grammar kinds loaded — no completions
        let completions = adapter.complete("run");
        assert!(completions.is_empty());
    }

    #[test]
    fn lsp_adapter_complete_with_cached_kinds() {
        let state = SharedState::new("a.db", "b.db");
        state.update_grammar_kinds(vec![
            GrammarKind { name: "render".into(), description: "output action".into() },
            GrammarKind { name: "resolve".into(), description: "lookup action".into() },
            GrammarKind { name: "concept".into(), description: "abstract idea".into() },
        ]);
        let ops = InteractiveTierOps::new(&state);
        let adapter = LspAdapter::new(ops);
        let completions = adapter.complete("re");
        assert_eq!(completions.len(), 2);
        assert!(completions.contains(&"render".to_string()));
        assert!(completions.contains(&"resolve".to_string()));
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
