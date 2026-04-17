#![deny(unsafe_code)]

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum HintKind {
    Type,
    Parameter,
    Return,
    Reference,
}

impl HintKind {
    fn from_str(s: &str) -> Self {
        match s {
            "type" => HintKind::Type,
            "parameter" => HintKind::Parameter,
            "return" => HintKind::Return,
            "reference" => HintKind::Reference,
            _ => HintKind::Type,
        }
    }
}

#[derive(Clone, Debug)]
pub struct InlayHint {
    pub line: u32,
    pub col: u32,
    pub label: String,
    pub kind: HintKind,
    pub tooltip: Option<String>,
}

pub struct InlayHintProvider {
    hints: Vec<InlayHint>,
}

impl InlayHintProvider {
    pub fn new() -> Self {
        Self { hints: Vec::new() }
    }

    pub fn add_hint(&mut self, line: u32, col: u32, label: impl Into<String>, kind: HintKind) {
        self.hints.push(InlayHint { line, col, label: label.into(), kind, tooltip: None });
    }

    pub fn hints_for_line(&self, line: u32) -> Vec<&InlayHint> {
        self.hints.iter().filter(|h| h.line == line).collect()
    }

    pub fn hint_count(&self) -> usize {
        self.hints.len()
    }

    pub fn clear(&mut self) {
        self.hints.clear();
    }

    /// Parse (line, col, label, kind_str) tuples from an LSP response.
    pub fn from_lsp_response(raw: &[(u32, u32, &str, &str)]) -> Self {
        let hints = raw
            .iter()
            .map(|&(line, col, label, kind_str)| InlayHint {
                line,
                col,
                label: label.to_owned(),
                kind: HintKind::from_str(kind_str),
                tooltip: None,
            })
            .collect();
        Self { hints }
    }
}

impl Default for InlayHintProvider {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hints_add_and_retrieve_by_line() {
        let mut provider = InlayHintProvider::new();
        provider.add_hint(3, 10, ": str", HintKind::Type);
        provider.add_hint(3, 20, "name:", HintKind::Parameter);
        provider.add_hint(5, 0, "-> bool", HintKind::Return);
        assert_eq!(provider.hints_for_line(3).len(), 2);
        assert_eq!(provider.hints_for_line(5).len(), 1);
        assert_eq!(provider.hints_for_line(99).len(), 0);
    }

    #[test]
    fn hints_from_lsp_response_parses_kind() {
        let raw = [
            (1, 5, ": u32", "type"),
            (2, 0, "count:", "parameter"),
            (3, 8, "-> bool", "return"),
            (4, 1, "ref", "reference"),
            (5, 0, "unknown", "other"),
        ];
        let provider = InlayHintProvider::from_lsp_response(&raw);
        assert_eq!(provider.hint_count(), 5);
        assert_eq!(provider.hints_for_line(1)[0].kind, HintKind::Type);
        assert_eq!(provider.hints_for_line(2)[0].kind, HintKind::Parameter);
        assert_eq!(provider.hints_for_line(3)[0].kind, HintKind::Return);
        assert_eq!(provider.hints_for_line(4)[0].kind, HintKind::Reference);
        // unknown kind_str falls back to Type
        assert_eq!(provider.hints_for_line(5)[0].kind, HintKind::Type);
    }

    #[test]
    fn hints_clear_removes_all() {
        let mut provider = InlayHintProvider::new();
        provider.add_hint(0, 0, ": i32", HintKind::Type);
        provider.add_hint(1, 0, "x:", HintKind::Parameter);
        assert_eq!(provider.hint_count(), 2);
        provider.clear();
        assert_eq!(provider.hint_count(), 0);
        assert!(provider.hints_for_line(0).is_empty());
    }

    #[test]
    fn hints_for_line_filters_correctly() {
        let mut provider = InlayHintProvider::new();
        for line in 0..5u32 {
            provider.add_hint(line, 0, "hint", HintKind::Type);
        }
        // add a second hint on line 2
        provider.add_hint(2, 8, "extra", HintKind::Parameter);
        assert_eq!(provider.hints_for_line(2).len(), 2);
        for line in [0u32, 1, 3, 4] {
            assert_eq!(provider.hints_for_line(line).len(), 1);
        }
    }

    #[test]
    fn hint_provider_hints_for_line_empty() {
        let provider = InlayHintProvider::new();
        assert!(provider.hints_for_line(0).is_empty());
        assert!(provider.hints_for_line(99).is_empty());
    }

    #[test]
    fn hint_provider_add_then_count() {
        let mut provider = InlayHintProvider::new();
        provider.add_hint(0, 0, ": u8", HintKind::Type);
        provider.add_hint(1, 0, "x:", HintKind::Parameter);
        provider.add_hint(2, 0, "-> bool", HintKind::Return);
        assert_eq!(provider.hint_count(), 3);
    }

    #[test]
    fn hint_provider_clear_removes_all() {
        let mut provider = InlayHintProvider::new();
        provider.add_hint(0, 0, ": i32", HintKind::Type);
        provider.add_hint(1, 4, "n:", HintKind::Parameter);
        assert_eq!(provider.hint_count(), 2);
        provider.clear();
        assert_eq!(provider.hint_count(), 0);
    }

    #[test]
    fn hint_provider_from_lsp_response_parses() {
        let raw = [(10, 3, ": str", "type"), (11, 0, "val:", "parameter")];
        let provider = InlayHintProvider::from_lsp_response(&raw);
        assert_eq!(provider.hint_count(), 2);
        assert_eq!(provider.hints_for_line(10)[0].label, ": str");
        assert_eq!(provider.hints_for_line(11)[0].kind, HintKind::Parameter);
    }

    #[test]
    fn hint_provider_hints_for_line_filters() {
        let mut provider = InlayHintProvider::new();
        provider.add_hint(1, 0, "a", HintKind::Type);
        provider.add_hint(2, 0, "b", HintKind::Type);
        provider.add_hint(3, 0, "c", HintKind::Type);
        assert_eq!(provider.hints_for_line(2).len(), 1);
        assert_eq!(provider.hints_for_line(2)[0].label, "b");
    }

    #[test]
    fn hint_kind_type_is_not_parameter() {
        assert_ne!(HintKind::Type, HintKind::Parameter);
        assert_eq!(HintKind::Return, HintKind::Return);
        assert_ne!(HintKind::Reference, HintKind::Type);
    }
}
