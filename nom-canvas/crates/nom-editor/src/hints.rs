//! Inlay hints from the LSP (nom-lsp). For MVP these are stub structs;
//! the actual LSP wiring lands with nom-compiler integration.

#![deny(unsafe_code)]

use std::ops::Range;

/// A single inlay hint to be rendered inline in the editor.
#[derive(Clone, Debug)]
pub struct InlayHint {
    /// Byte offset where the hint is displayed (immediately before the
    /// character at this position).
    pub offset: usize,
    /// Human-readable label, e.g. `": i32"` or `"count:"`.
    pub label: String,
    pub kind: InlayKind,
}

/// Classifies what the hint conveys; callers may use this for styling.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum InlayKind {
    /// Inferred type annotation.
    Type,
    /// Named argument label at a call site.
    Parameter,
    /// Method-chain intermediate type.
    Chaining,
}

/// Return only the hints whose `offset` falls within `visible_range`.
///
/// The range is half-open `[start, end)` matching Rust `Range` convention.
pub fn filter_visible(hints: &[InlayHint], visible_range: Range<usize>) -> Vec<InlayHint> {
    hints
        .iter()
        .filter(|h| h.offset >= visible_range.start && h.offset < visible_range.end)
        .cloned()
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_hint(offset: usize, label: &str, kind: InlayKind) -> InlayHint {
        InlayHint { offset, label: label.into(), kind }
    }

    #[test]
    fn filter_visible_returns_in_range() {
        let hints = vec![
            make_hint(5,  "i32", InlayKind::Type),
            make_hint(50, "i64", InlayKind::Type),
        ];
        let vis = filter_visible(&hints, 0..20);
        assert_eq!(vis.len(), 1);
        assert_eq!(vis[0].offset, 5);
    }

    #[test]
    fn filter_visible_excludes_end_boundary() {
        // offset == end is excluded (half-open range)
        let hints = vec![make_hint(20, "x", InlayKind::Parameter)];
        let vis = filter_visible(&hints, 0..20);
        assert!(vis.is_empty());
    }

    #[test]
    fn filter_visible_includes_start_boundary() {
        let hints = vec![make_hint(0, "x", InlayKind::Chaining)];
        let vis = filter_visible(&hints, 0..10);
        assert_eq!(vis.len(), 1);
    }

    #[test]
    fn filter_visible_empty_hints() {
        let vis = filter_visible(&[], 0..100);
        assert!(vis.is_empty());
    }

    #[test]
    fn filter_visible_empty_range() {
        let hints = vec![make_hint(5, "x", InlayKind::Type)];
        let vis = filter_visible(&hints, 5..5); // empty range
        assert!(vis.is_empty());
    }
}
