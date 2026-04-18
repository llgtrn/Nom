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
        self.hints.push(InlayHint {
            line,
            col,
            label: label.into(),
            kind,
            tooltip: None,
        });
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

    // ── wave AJ-7: inlay hint tests ──────────────────────────────────────────

    /// Inlay hint for type annotation is present at the given position.
    #[test]
    fn inlay_hint_type_annotation_present() {
        let mut provider = InlayHintProvider::new();
        provider.add_hint(5, 12, ": u32", HintKind::Type);
        let hints = provider.hints_for_line(5);
        assert_eq!(hints.len(), 1);
        assert_eq!(hints[0].label, ": u32");
    }

    /// Inlay hint position is correct — line and col match what was added.
    #[test]
    fn inlay_hint_position_correct() {
        let mut provider = InlayHintProvider::new();
        provider.add_hint(3, 7, ": str", HintKind::Type);
        let hints = provider.hints_for_line(3);
        assert_eq!(hints.len(), 1);
        assert_eq!(hints[0].line, 3);
        assert_eq!(hints[0].col, 7);
    }

    /// Inlay hint kind is Type for a type annotation.
    #[test]
    fn inlay_hint_kind_is_type() {
        let mut provider = InlayHintProvider::new();
        provider.add_hint(0, 0, ": bool", HintKind::Type);
        let hints = provider.hints_for_line(0);
        assert_eq!(hints.len(), 1);
        assert_eq!(hints[0].kind, HintKind::Type);
    }

    /// Multiple type hints on the same line are all present.
    #[test]
    fn inlay_hint_multiple_on_same_line() {
        let mut provider = InlayHintProvider::new();
        provider.add_hint(2, 0, ": u8", HintKind::Type);
        provider.add_hint(2, 10, ": str", HintKind::Type);
        provider.add_hint(2, 20, "-> bool", HintKind::Return);
        let hints = provider.hints_for_line(2);
        assert_eq!(hints.len(), 3);
    }

    /// hint_count grows monotonically as hints are added.
    #[test]
    fn inlay_hint_count_grows_monotonically() {
        let mut provider = InlayHintProvider::new();
        for i in 0u32..5 {
            provider.add_hint(i, 0, ": i32", HintKind::Type);
            assert_eq!(provider.hint_count(), (i + 1) as usize);
        }
    }

    /// hint label for a parameter hint starts with the param name followed by ':'.
    #[test]
    fn inlay_hint_parameter_label_format() {
        let mut provider = InlayHintProvider::new();
        provider.add_hint(1, 5, "count:", HintKind::Parameter);
        let hints = provider.hints_for_line(1);
        assert_eq!(hints.len(), 1);
        assert!(hints[0].label.ends_with(':'), "parameter hint label must end with ':'");
    }

    /// Hint tooltip field is None by default (add_hint does not set tooltip).
    #[test]
    fn inlay_hint_tooltip_none_by_default() {
        let mut provider = InlayHintProvider::new();
        provider.add_hint(0, 0, "hint", HintKind::Type);
        let hints = provider.hints_for_line(0);
        assert!(hints[0].tooltip.is_none());
    }

    /// from_lsp_response produces hints sorted by insertion order (index order).
    #[test]
    fn inlay_hint_from_lsp_response_preserves_order() {
        let raw = [
            (1, 0, ": u8", "type"),
            (2, 0, "x:", "parameter"),
            (3, 0, "-> bool", "return"),
        ];
        let provider = InlayHintProvider::from_lsp_response(&raw);
        // hint_count == 3 and each line has exactly one hint
        assert_eq!(provider.hint_count(), 3);
        assert_eq!(provider.hints_for_line(1)[0].label, ": u8");
        assert_eq!(provider.hints_for_line(2)[0].label, "x:");
        assert_eq!(provider.hints_for_line(3)[0].label, "-> bool");
    }

    // ── wave AB: inlay hint tests ────────────────────────────────────────────

    /// Inlay hint at column 0 has correct position.
    #[test]
    fn inlay_hint_at_column_zero_correct_position() {
        let mut provider = InlayHintProvider::new();
        provider.add_hint(0, 0, ": i64", HintKind::Type);
        let hints = provider.hints_for_line(0);
        assert_eq!(hints.len(), 1);
        assert_eq!(hints[0].col, 0);
        assert_eq!(hints[0].line, 0);
    }

    /// Inlay hint label includes a type annotation marker.
    #[test]
    fn inlay_hint_label_includes_type_annotation() {
        let mut provider = InlayHintProvider::new();
        provider.add_hint(2, 5, ": f64", HintKind::Type);
        let hints = provider.hints_for_line(2);
        assert!(hints[0].label.contains(':'), "type annotation hint label must contain ':'");
    }

    /// Multiple hints on the same line ordered by col as inserted.
    #[test]
    fn inlay_hints_same_line_ordered_by_col() {
        let mut provider = InlayHintProvider::new();
        provider.add_hint(1, 3, ": u8", HintKind::Type);
        provider.add_hint(1, 10, "count:", HintKind::Parameter);
        provider.add_hint(1, 20, "-> bool", HintKind::Return);
        let hints = provider.hints_for_line(1);
        assert_eq!(hints.len(), 3);
        // Verify cols in insertion order
        assert_eq!(hints[0].col, 3);
        assert_eq!(hints[1].col, 10);
        assert_eq!(hints[2].col, 20);
    }

    /// Hint with tooltip preserved when set directly on the InlayHint struct.
    #[test]
    fn inlay_hint_tooltip_preserved_when_set() {
        let hint = InlayHint {
            line: 5,
            col: 8,
            label: ": str".into(),
            kind: HintKind::Type,
            tooltip: Some("The name of the entity".into()),
        };
        assert_eq!(hint.tooltip.as_deref(), Some("The name of the entity"));
    }

    /// Clear hints empties the hint list completely.
    #[test]
    fn inlay_hint_clear_empties_list() {
        let mut provider = InlayHintProvider::new();
        provider.add_hint(0, 0, ": i32", HintKind::Type);
        provider.add_hint(1, 0, ": str", HintKind::Type);
        provider.add_hint(2, 0, "-> bool", HintKind::Return);
        assert_eq!(provider.hint_count(), 3);
        provider.clear();
        assert_eq!(provider.hint_count(), 0);
        assert!(provider.hints_for_line(0).is_empty());
        assert!(provider.hints_for_line(1).is_empty());
    }

    /// Return kind hint has label starting with "->".
    #[test]
    fn inlay_hint_return_kind_label_format() {
        let mut provider = InlayHintProvider::new();
        provider.add_hint(7, 0, "-> u32", HintKind::Return);
        let hints = provider.hints_for_line(7);
        assert_eq!(hints.len(), 1);
        assert_eq!(hints[0].kind, HintKind::Return);
        assert!(hints[0].label.starts_with("->"), "return hint label must start with '->'");
    }

    /// Reference kind hint is distinct from type hint.
    #[test]
    fn inlay_hint_reference_kind_distinct_from_type() {
        let mut provider = InlayHintProvider::new();
        provider.add_hint(0, 0, "ref", HintKind::Reference);
        provider.add_hint(1, 0, ": u8", HintKind::Type);
        assert_eq!(provider.hints_for_line(0)[0].kind, HintKind::Reference);
        assert_eq!(provider.hints_for_line(1)[0].kind, HintKind::Type);
        assert_ne!(
            provider.hints_for_line(0)[0].kind,
            provider.hints_for_line(1)[0].kind
        );
    }

    /// from_lsp_response with empty slice produces zero hints.
    #[test]
    fn inlay_hint_from_lsp_response_empty_produces_zero() {
        let raw: &[(u32, u32, &str, &str)] = &[];
        let provider = InlayHintProvider::from_lsp_response(raw);
        assert_eq!(provider.hint_count(), 0);
    }

    /// Default provider constructed via Default trait has zero hints.
    #[test]
    fn inlay_hint_provider_default_has_zero_hints() {
        let provider = InlayHintProvider::default();
        assert_eq!(provider.hint_count(), 0);
    }

    /// Hint added to high line number is retrievable.
    #[test]
    fn inlay_hint_high_line_number_retrievable() {
        let mut provider = InlayHintProvider::new();
        provider.add_hint(9999, 0, ": u64", HintKind::Type);
        let hints = provider.hints_for_line(9999);
        assert_eq!(hints.len(), 1);
        assert_eq!(hints[0].label, ": u64");
    }

    /// hints_for_line returns empty for a line with no hints.
    #[test]
    fn inlay_hint_line_with_no_hints_returns_empty() {
        let mut provider = InlayHintProvider::new();
        provider.add_hint(5, 0, ": u8", HintKind::Type);
        // Line 6 has no hints
        assert!(provider.hints_for_line(6).is_empty());
    }
}
