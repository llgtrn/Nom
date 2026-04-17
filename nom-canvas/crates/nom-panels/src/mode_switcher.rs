//! Mode switcher panel — unified editing mode per document.

use std::collections::HashMap;

use crate::DocumentId;

/// The editing mode for a document in the unified canvas.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum UnifiedMode {
    Code,
    Doc,
    Canvas,
    Graph,
    Draw,
}

impl Default for UnifiedMode {
    fn default() -> Self {
        UnifiedMode::Code
    }
}

/// Mode switcher state — tracks global mode and per-document overrides.
#[derive(Debug)]
pub struct ModeSwitcher {
    pub current: UnifiedMode,
    pub per_document: HashMap<DocumentId, UnifiedMode>,
}

impl ModeSwitcher {
    pub fn new() -> Self {
        Self {
            current: UnifiedMode::Code,
            per_document: HashMap::new(),
        }
    }

    /// Set the global current mode.
    pub fn set(&mut self, mode: UnifiedMode) {
        self.current = mode;
    }

    /// Get the mode for a specific document, falling back to `current`.
    pub fn get_for_document(&self, id: DocumentId) -> UnifiedMode {
        self.per_document.get(&id).copied().unwrap_or(self.current)
    }

    /// Stub paint method — rendering lives in the GPU layer.
    pub fn paint(&self) {}
}

impl Default for ModeSwitcher {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_mode_is_code() {
        let ms = ModeSwitcher::new();
        assert_eq!(ms.current, UnifiedMode::Code);
    }

    #[test]
    fn set_changes_current_mode() {
        let mut ms = ModeSwitcher::new();
        ms.set(UnifiedMode::Canvas);
        assert_eq!(ms.current, UnifiedMode::Canvas);
    }

    #[test]
    fn per_doc_mode_persists_and_overrides() {
        let mut ms = ModeSwitcher::new();
        ms.per_document.insert(99, UnifiedMode::Draw);
        assert_eq!(ms.get_for_document(99), UnifiedMode::Draw);
    }

    #[test]
    fn missing_doc_falls_back_to_current() {
        let mut ms = ModeSwitcher::new();
        ms.set(UnifiedMode::Graph);
        assert_eq!(ms.get_for_document(404), UnifiedMode::Graph);
    }
}
