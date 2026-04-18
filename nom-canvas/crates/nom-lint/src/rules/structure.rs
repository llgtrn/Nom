/// The specific kind of structural violation detected.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StructureViolationKind {
    EmptyBlock,
    DeepNesting,
    UnreachableCode,
    MissingReturn,
}

/// A structural violation found in a source snippet.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StructureViolation {
    pub kind: StructureViolationKind,
    pub location: String,
    pub message: String,
}

/// Linter that checks structural properties of source code.
pub struct StructureLinter;

impl StructureLinter {
    /// Create a new `StructureLinter`.
    pub fn new() -> Self {
        Self
    }

    /// Detect nesting that exceeds 5 levels deep by counting `{` vs `}`.
    ///
    /// Returns `Some(StructureViolation)` with `kind = DeepNesting` if the
    /// maximum nesting depth seen anywhere in `source` exceeds 5.
    pub fn check_nesting_depth(&self, source: &str) -> Option<StructureViolation> {
        let mut depth: i32 = 0;
        let mut max_depth: i32 = 0;
        for ch in source.chars() {
            match ch {
                '{' => {
                    depth += 1;
                    if depth > max_depth {
                        max_depth = depth;
                    }
                }
                '}' => {
                    depth -= 1;
                }
                _ => {}
            }
        }
        if max_depth > 5 {
            Some(StructureViolation {
                kind: StructureViolationKind::DeepNesting,
                location: "source".to_owned(),
                message: format!("nesting depth {max_depth} exceeds maximum of 5"),
            })
        } else {
            None
        }
    }

    /// Detect empty blocks: either `{}` or `{ }` (with optional whitespace).
    ///
    /// Returns `Some(StructureViolation)` with `kind = EmptyBlock` on the
    /// first match.
    pub fn check_empty_blocks(&self, source: &str) -> Option<StructureViolation> {
        // Match `{` followed by optional whitespace and then `}`.
        let mut chars = source.char_indices().peekable();
        while let Some((i, ch)) = chars.next() {
            if ch == '{' {
                // Collect characters until we hit a non-whitespace or `}`.
                let mut only_whitespace = true;
                let mut closed = false;
                let start = i;
                for (_, inner) in chars.by_ref() {
                    if inner == '}' {
                        closed = true;
                        break;
                    } else if !inner.is_whitespace() {
                        only_whitespace = false;
                        break;
                    }
                }
                if closed && only_whitespace {
                    return Some(StructureViolation {
                        kind: StructureViolationKind::EmptyBlock,
                        location: format!("offset {start}"),
                        message: "empty block detected".to_owned(),
                    });
                }
            }
        }
        None
    }

    /// Run all structural checks and return every violation found.
    pub fn lint_all(&self, source: &str) -> Vec<StructureViolation> {
        let mut violations = Vec::new();
        if let Some(v) = self.check_nesting_depth(source) {
            violations.push(v);
        }
        if let Some(v) = self.check_empty_blocks(source) {
            violations.push(v);
        }
        violations
    }
}

impl Default for StructureLinter {
    fn default() -> Self {
        Self::new()
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn no_violations() {
        let linter = StructureLinter::new();
        let src = "fn foo() { let x = 1; }";
        assert!(linter.lint_all(src).is_empty());
    }

    #[test]
    fn deep_nesting_detected() {
        let linter = StructureLinter::new();
        // 6 levels of `{` — exceeds the limit of 5.
        let src = "{ { { { { { } } } } } }";
        let v = linter.check_nesting_depth(src);
        assert!(v.is_some());
        let v = v.unwrap();
        assert_eq!(v.kind, StructureViolationKind::DeepNesting);
        assert!(v.message.contains('6'));
    }

    #[test]
    fn empty_block_detected() {
        let linter = StructureLinter::new();
        let src = "fn foo() {}";
        let v = linter.check_empty_blocks(src);
        assert!(v.is_some());
        assert_eq!(v.unwrap().kind, StructureViolationKind::EmptyBlock);
    }

    #[test]
    fn lint_all_multiple() {
        let linter = StructureLinter::new();
        // Both deep nesting AND an empty block somewhere.
        let src = "{ { { { { { } } } } } } fn bar() {}";
        let violations = linter.lint_all(src);
        assert_eq!(violations.len(), 2);
        let kinds: Vec<&StructureViolationKind> = violations.iter().map(|v| &v.kind).collect();
        assert!(kinds.contains(&&StructureViolationKind::DeepNesting));
        assert!(kinds.contains(&&StructureViolationKind::EmptyBlock));
    }

    #[test]
    fn structure_violation_fields() {
        let v = StructureViolation {
            kind: StructureViolationKind::UnreachableCode,
            location: "line:10".to_owned(),
            message: "unreachable after return".to_owned(),
        };
        assert_eq!(v.kind, StructureViolationKind::UnreachableCode);
        assert_eq!(v.location, "line:10");
        assert_eq!(v.message, "unreachable after return");
    }

    #[test]
    fn nesting_depth_ok() {
        let linter = StructureLinter::new();
        // Exactly 5 levels — should NOT fire.
        let src = "{ { { { { } } } } }";
        assert!(linter.check_nesting_depth(src).is_none());
    }
}
