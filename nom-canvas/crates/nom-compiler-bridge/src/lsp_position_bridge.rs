#![deny(unsafe_code)]
use nom_editor::lsp_bridge::{LspPosition, LspRange};

/// Namespace for LSP ↔ buffer-point position conversions.
pub struct LspPositionBridge;

impl LspPositionBridge {
    /// Convert an [`LspPosition`] to a `(row, col)` tuple of `usize`.
    pub fn lsp_to_point(pos: &LspPosition) -> (usize, usize) {
        (pos.line as usize, pos.character as usize)
    }

    /// Create an [`LspPosition`] from a `(row, col)` pair of `usize`.
    pub fn point_to_lsp(row: usize, col: usize) -> LspPosition {
        LspPosition {
            line: row as u32,
            character: col as u32,
        }
    }

    /// Convert an [`LspRange`] to a pair of `(row, col)` tuples: `(start, end)`.
    pub fn lsp_range_to_span(range: &LspRange) -> ((usize, usize), (usize, usize)) {
        (
            Self::lsp_to_point(&range.start),
            Self::lsp_to_point(&range.end),
        )
    }
}

// ── Error types ───────────────────────────────────────────────────────────────

#[derive(Debug, PartialEq, Eq)]
pub enum PositionConversionError {
    OutOfBounds { line: u32, character: u32 },
    InvalidRange,
}

// ── Bounded bridge ────────────────────────────────────────────────────────────

/// A position bridge that validates positions against fixed document bounds.
pub struct BoundedLspBridge {
    pub max_line: u32,
    pub max_char: u32,
}

impl BoundedLspBridge {
    pub fn new(max_line: u32, max_char: u32) -> Self {
        Self { max_line, max_char }
    }

    /// Validate `pos` against the document bounds.
    ///
    /// Returns `Ok((row, col))` when in-bounds, or
    /// `Err(PositionConversionError::OutOfBounds)` when the position exceeds
    /// `max_line` or `max_char`.
    pub fn validate_position(
        &self,
        pos: &LspPosition,
    ) -> Result<(usize, usize), PositionConversionError> {
        if pos.line > self.max_line || pos.character > self.max_char {
            Err(PositionConversionError::OutOfBounds {
                line: pos.line,
                character: pos.character,
            })
        } else {
            Ok((pos.line as usize, pos.character as usize))
        }
    }

    /// Validate that `range.start` does not come after `range.end` on the same
    /// or a later line.
    ///
    /// Returns `Err(PositionConversionError::InvalidRange)` when
    /// `start.line > end.line`, or when the lines are equal and
    /// `start.character > end.character`.
    pub fn validate_range(&self, range: &LspRange) -> Result<(), PositionConversionError> {
        let start = &range.start;
        let end = &range.end;
        if start.line > end.line
            || (start.line == end.line && start.character > end.character)
        {
            Err(PositionConversionError::InvalidRange)
        } else {
            Ok(())
        }
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod lsp_position_bridge_tests {
    use super::*;

    /// 1. lsp_to_point() roundtrip: convert to point and back to LspPosition.
    #[test]
    fn lsp_to_point_roundtrip() {
        let pos = LspPosition {
            line: 5,
            character: 12,
        };
        let (row, col) = LspPositionBridge::lsp_to_point(&pos);
        let back = LspPositionBridge::point_to_lsp(row, col);
        assert_eq!(back, pos);
    }

    /// 2. point_to_lsp() creates correct LspPosition.
    #[test]
    fn point_to_lsp_creates_correct_position() {
        let lsp = LspPositionBridge::point_to_lsp(3, 7);
        assert_eq!(lsp.line, 3);
        assert_eq!(lsp.character, 7);
    }

    /// 3. lsp_range_to_span() returns start and end tuples.
    #[test]
    fn lsp_range_to_span_returns_start_and_end() {
        let range = LspRange {
            start: LspPosition {
                line: 1,
                character: 4,
            },
            end: LspPosition {
                line: 2,
                character: 9,
            },
        };
        let (start, end) = LspPositionBridge::lsp_range_to_span(&range);
        assert_eq!(start, (1, 4));
        assert_eq!(end, (2, 9));
    }

    /// 4. BoundedLspBridge::new() stores fields correctly.
    #[test]
    fn bounded_lsp_bridge_new_stores_fields() {
        let bridge = BoundedLspBridge::new(100, 200);
        assert_eq!(bridge.max_line, 100);
        assert_eq!(bridge.max_char, 200);
    }

    /// 5. validate_position() succeeds when position is in bounds.
    #[test]
    fn validate_position_success() {
        let bridge = BoundedLspBridge::new(10, 80);
        let pos = LspPosition {
            line: 5,
            character: 20,
        };
        let result = bridge.validate_position(&pos);
        assert_eq!(result, Ok((5, 20)));
    }

    /// 6. validate_position() returns OutOfBounds when position exceeds limits.
    #[test]
    fn validate_position_out_of_bounds_returns_err() {
        let bridge = BoundedLspBridge::new(10, 80);
        let pos = LspPosition {
            line: 11,
            character: 0,
        };
        let result = bridge.validate_position(&pos);
        assert_eq!(
            result,
            Err(PositionConversionError::OutOfBounds {
                line: 11,
                character: 0,
            })
        );
    }

    /// 7. validate_range() succeeds for a valid (start <= end) range.
    #[test]
    fn validate_range_valid_range_ok() {
        let bridge = BoundedLspBridge::new(100, 200);
        let range = LspRange {
            start: LspPosition {
                line: 0,
                character: 5,
            },
            end: LspPosition {
                line: 0,
                character: 10,
            },
        };
        assert_eq!(bridge.validate_range(&range), Ok(()));
    }

    /// 8. validate_range() returns InvalidRange when start > end on the same line.
    #[test]
    fn validate_range_start_after_end_returns_invalid() {
        let bridge = BoundedLspBridge::new(100, 200);
        let range = LspRange {
            start: LspPosition {
                line: 3,
                character: 10,
            },
            end: LspPosition {
                line: 2,
                character: 0,
            },
        };
        assert_eq!(
            bridge.validate_range(&range),
            Err(PositionConversionError::InvalidRange)
        );
    }

    /// 9. LspPosition at line 0, character 0 roundtrips through lsp_to_point / point_to_lsp.
    #[test]
    fn lsp_position_line0_col0_roundtrip() {
        let pos = LspPosition {
            line: 0,
            character: 0,
        };
        let (row, col) = LspPositionBridge::lsp_to_point(&pos);
        assert_eq!((row, col), (0, 0));
        let back = LspPositionBridge::point_to_lsp(row, col);
        assert_eq!(back, LspPosition {
            line: 0,
            character: 0,
        });
    }
}
