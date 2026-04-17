//! Atomic edit operations with transaction scoping.
#![deny(unsafe_code)]

use ropey::Rope;
use std::ops::Range;

/// A single edit: replace `range` (char offsets) with `text`.
#[derive(Clone, Debug)]
pub struct Edit {
    pub range: Range<usize>,
    pub text: String,
}

/// Autoindent mode for new-line insertion.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum AutoindentMode {
    None,
    EachLine,
    Block,
}

/// Errors produced by edit operations.
#[derive(Debug, thiserror::Error)]
pub enum EditError {
    #[error("edit range {start}..{end} overlaps with another edit")]
    OverlappingRanges { start: usize, end: usize },
    #[error("edit range {start}..{end} exceeds buffer length {len}")]
    OutOfBounds { start: usize, end: usize, len: usize },
}

/// Apply a batch of edits atomically to a Rope.
///
/// Edits are sorted by `range.start` **descending** before application so
/// earlier char offsets are not shifted by later removals/insertions.
/// Overlapping ranges return `Err(EditError::OverlappingRanges)`.
/// Out-of-bounds ranges return `Err(EditError::OutOfBounds)`.
pub fn apply_edits(rope: &mut Rope, mut edits: Vec<Edit>) -> Result<(), EditError> {
    let len = rope.len_chars();

    // Validate all edits before touching the rope.
    for edit in &edits {
        let start = edit.range.start;
        let end = edit.range.end;
        if start > len || end > len {
            return Err(EditError::OutOfBounds { start, end, len });
        }
    }

    // Sort descending by start so we apply from back to front.
    edits.sort_by(|a, b| b.range.start.cmp(&a.range.start));

    // Check for overlaps (after sort, adjacent pairs where prev.start < cur.end).
    for window in edits.windows(2) {
        // window[0].start >= window[1].start (descending sort)
        let later = &window[0]; // higher start
        let earlier = &window[1]; // lower start
        // Overlap if earlier.end > later.start
        if earlier.range.end > later.range.start {
            return Err(EditError::OverlappingRanges {
                start: later.range.start,
                end: earlier.range.end,
            });
        }
    }

    // Apply back-to-front.
    for edit in &edits {
        if edit.range.start < edit.range.end {
            rope.remove(edit.range.clone());
        }
        if !edit.text.is_empty() {
            rope.insert(edit.range.start, &edit.text);
        }
    }

    Ok(())
}

/// Transaction scope guard: increments a depth counter on construction,
/// decrements on drop.
pub struct Transaction<'a> {
    depth_counter: &'a mut usize,
}

impl<'a> Transaction<'a> {
    /// Begin a transaction, incrementing the nesting depth.
    pub fn new(depth_counter: &'a mut usize) -> Self {
        *depth_counter += 1;
        Self { depth_counter }
    }

    /// True only when this is the outermost (non-nested) transaction.
    pub fn is_outermost(&self) -> bool {
        *self.depth_counter == 1
    }
}

impl<'a> Drop for Transaction<'a> {
    fn drop(&mut self) {
        *self.depth_counter -= 1;
    }
}

/// Run `f` inside a transaction scope; returns `f`'s result.
pub fn transact<R>(depth_counter: &mut usize, f: impl FnOnce() -> R) -> R {
    let _tx = Transaction::new(depth_counter);
    f()
}

/// Apply autoindent when mode is not `None`.
///
/// `EachLine`: for each `Edit` whose text contains a newline, prepend the
/// leading whitespace of the line at `edit.range.start` to each new line
/// introduced by the edit.
///
/// `Block`: same as `EachLine` (minimal stub — full block-indent logic is
/// a separate concern).
pub fn apply_autoindent(rope: &Rope, edits: &mut Vec<Edit>, mode: AutoindentMode) {
    if mode == AutoindentMode::None {
        return;
    }
    for edit in edits.iter_mut() {
        if !edit.text.contains('\n') {
            continue;
        }
        // Determine leading whitespace of the line containing range.start.
        let line_idx = rope.char_to_line(edit.range.start.min(rope.len_chars()));
        let line_text = rope.line(line_idx).to_string();
        let indent: String = line_text
            .chars()
            .take_while(|c| *c == ' ' || *c == '\t')
            .collect();
        if indent.is_empty() {
            continue;
        }
        // Prepend indent to every line after the first newline in edit.text.
        let mut result = String::with_capacity(edit.text.len() + indent.len() * 4);
        for (i, part) in edit.text.split('\n').enumerate() {
            if i > 0 {
                result.push('\n');
                result.push_str(&indent);
            }
            result.push_str(part);
        }
        edit.text = result;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ropey::Rope;

    fn rope(s: &str) -> Rope {
        Rope::from_str(s)
    }

    #[test]
    fn single_edit_insertion() {
        let mut r = rope("hello world");
        apply_edits(
            &mut r,
            vec![Edit { range: 5..5, text: ",".into() }],
        )
        .unwrap();
        assert_eq!(r.to_string(), "hello, world");
    }

    #[test]
    fn multi_edit_reverse_offset_ordering() {
        // Replace "world" with "there" and "hello" with "hi".
        // Edits given in any order; function must sort descending.
        let mut r = rope("hello world");
        apply_edits(
            &mut r,
            vec![
                Edit { range: 0..5, text: "hi".into() },
                Edit { range: 6..11, text: "there".into() },
            ],
        )
        .unwrap();
        assert_eq!(r.to_string(), "hi there");
    }

    #[test]
    fn overlapping_ranges_error() {
        let mut r = rope("hello world");
        let result = apply_edits(
            &mut r,
            vec![
                Edit { range: 2..8, text: "X".into() },
                Edit { range: 5..10, text: "Y".into() },
            ],
        );
        assert!(matches!(result, Err(EditError::OverlappingRanges { .. })));
    }

    #[test]
    fn out_of_bounds_error() {
        let mut r = rope("hi");
        let result = apply_edits(
            &mut r,
            vec![Edit { range: 0..99, text: "X".into() }],
        );
        assert!(matches!(result, Err(EditError::OutOfBounds { .. })));
    }

    #[test]
    fn transaction_depth_increments_on_enter_decrements_on_drop() {
        let mut depth = 0usize;
        {
            let tx = Transaction::new(&mut depth);
            assert_eq!(*tx.depth_counter, 1);
        }
        assert_eq!(depth, 0);
    }

    #[test]
    fn outermost_detection() {
        let mut depth = 0usize;
        {
            let tx = Transaction::new(&mut depth);
            assert!(tx.is_outermost());
            assert_eq!(*tx.depth_counter, 1);
        }
        // After drop, depth is back to 0 — any new transaction would be outermost.
        assert_eq!(depth, 0);
    }

    #[test]
    fn transact_returns_value() {
        let mut depth = 0usize;
        let result = transact(&mut depth, || 42usize);
        assert_eq!(result, 42);
        assert_eq!(depth, 0); // depth back to 0 after transact
    }

    #[test]
    fn autoindent_each_line_adds_spaces() {
        let r = rope("    hello\n");
        let mut edits = vec![Edit {
            range: 9..9, // end of "    hello"
            text: "\nworld".into(),
        }];
        apply_autoindent(&r, &mut edits, AutoindentMode::EachLine);
        // "world" should become "    world"
        assert_eq!(edits[0].text, "\n    world");
    }

    #[test]
    fn autoindent_none_leaves_edits_unchanged() {
        let r = rope("    hello\n");
        let mut edits = vec![Edit {
            range: 9..9,
            text: "\nworld".into(),
        }];
        apply_autoindent(&r, &mut edits, AutoindentMode::None);
        assert_eq!(edits[0].text, "\nworld");
    }
}
