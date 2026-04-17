//! Whole-tree structural validators for a block graph.
//!
//! Wraps the per-node `block_schema::validate_parent/validate_child` in a
//! recursive walk + emits aggregated errors so the caller sees every
//! violation in one pass instead of failing on the first.
#![deny(unsafe_code)]

use crate::block_model::{BlockId, BlockModel};
use crate::block_schema::{BlockSchema, SchemaError};
use crate::flavour::Flavour;

#[derive(Clone, Debug, PartialEq)]
pub struct TreeValidationReport {
    pub errors: Vec<TreeValidationError>,
    pub warnings: Vec<TreeValidationWarning>,
    pub visited_count: usize,
}

impl TreeValidationReport {
    pub fn new() -> Self {
        Self { errors: Vec::new(), warnings: Vec::new(), visited_count: 0 }
    }
    pub fn is_valid(&self) -> bool { self.errors.is_empty() }
    pub fn has_warnings(&self) -> bool { !self.warnings.is_empty() }
}

impl Default for TreeValidationReport {
    fn default() -> Self { Self::new() }
}

#[derive(Clone, Debug, PartialEq)]
pub enum TreeValidationError {
    /// A child block's flavour is not in its parent's `children` allowlist.
    IllegalChild {
        parent: BlockId,
        parent_flavour: Flavour,
        child: BlockId,
        child_flavour: Flavour,
    },
    /// A block's version is greater than the schema's declared version.
    VersionTooNew { block: BlockId, schema: u32, found: u32 },
    /// A block references a child that cannot be resolved via the store_fn.
    DanglingChildRef { parent: BlockId, missing_child: BlockId },
    /// Cycle detected: a block appears as descendant of itself.
    Cycle { start: BlockId, via: BlockId },
}

#[derive(Clone, Debug, PartialEq)]
pub enum TreeValidationWarning {
    /// A block's version is older than the schema's latest; consider migration.
    VersionTooOld { block: BlockId, schema: u32, found: u32 },
    /// A block has no children but the schema's `role` suggests a Hub.
    HubWithoutChildren { block: BlockId },
}

/// Walk the tree rooted at `root_ids` and report violations.  `schema_fn`
/// returns the schema for a given flavour; `store_fn` resolves block ids.
pub fn validate_tree<'a, P, SchemaFn, StoreFn>(
    root_ids: &[BlockId],
    schema_fn: SchemaFn,
    store_fn: StoreFn,
) -> TreeValidationReport
where
    P: 'a,
    SchemaFn: Fn(Flavour) -> Option<BlockSchema>,
    StoreFn: Fn(BlockId) -> Option<&'a BlockModel<P>>,
{
    let mut report = TreeValidationReport::new();
    let mut seen = std::collections::HashSet::new();
    for &root in root_ids {
        walk(root, &schema_fn, &store_fn, &mut report, &mut seen, &mut Vec::new());
    }
    report
}

fn walk<'a, P, SchemaFn, StoreFn>(
    id: BlockId,
    schema_fn: &SchemaFn,
    store_fn: &StoreFn,
    report: &mut TreeValidationReport,
    seen: &mut std::collections::HashSet<BlockId>,
    path: &mut Vec<BlockId>,
)
where
    P: 'a,
    SchemaFn: Fn(Flavour) -> Option<BlockSchema>,
    StoreFn: Fn(BlockId) -> Option<&'a BlockModel<P>>,
{
    if path.contains(&id) {
        let start = *path.first().unwrap();
        report.errors.push(TreeValidationError::Cycle { start, via: id });
        return;
    }
    if !seen.insert(id) { return; }
    report.visited_count += 1;
    path.push(id);
    let Some(block) = store_fn(id) else {
        path.pop();
        return;
    };
    let Some(schema) = schema_fn(block.flavour) else {
        // Unknown flavour — skip silently; external plugin flavours are legal.
        path.pop();
        return;
    };
    // Version compatibility check.
    if block.version > schema.version {
        report.errors.push(TreeValidationError::VersionTooNew {
            block: id,
            schema: schema.version,
            found: block.version,
        });
    } else if block.version < schema.version {
        report.warnings.push(TreeValidationWarning::VersionTooOld {
            block: id,
            schema: schema.version,
            found: block.version,
        });
    }
    // Hub-without-children warning.
    if matches!(schema.role, crate::block_schema::Role::Hub) && block.children.is_empty() {
        report.warnings.push(TreeValidationWarning::HubWithoutChildren { block: id });
    }
    // Per-child validation.
    for &child_id in &block.children {
        match store_fn(child_id) {
            None => {
                report.errors.push(TreeValidationError::DanglingChildRef {
                    parent: id,
                    missing_child: child_id,
                });
            }
            Some(child_block) => {
                let child_flavour = child_block.flavour;
                if !schema.children.contains(&child_flavour) {
                    report.errors.push(TreeValidationError::IllegalChild {
                        parent: id,
                        parent_flavour: block.flavour,
                        child: child_id,
                        child_flavour,
                    });
                }
                walk(child_id, schema_fn, store_fn, report, seen, path);
            }
        }
    }
    path.pop();
}

/// Adapter: convert `SchemaError` into the equivalent `TreeValidationError`.
pub fn error_from_schema(
    parent: BlockId,
    parent_flavour: Flavour,
    child: BlockId,
    child_flavour: Flavour,
    _err: SchemaError,
) -> TreeValidationError {
    TreeValidationError::IllegalChild { parent, parent_flavour, child, child_flavour }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::block_schema::Role;
    use crate::flavour::{NOTE, PROSE, SURFACE};
    use std::collections::HashMap;

    // Schema constants used across tests.
    const NOTE_SCHEMA: BlockSchema = BlockSchema {
        flavour: NOTE,
        version: 1,
        role: Role::Hub,
        parents: &[],
        children: &[PROSE],
    };
    const PROSE_SCHEMA: BlockSchema = BlockSchema {
        flavour: PROSE,
        version: 1,
        role: Role::Content,
        parents: &[NOTE],
        children: &[],
    };
    const SURFACE_SCHEMA: BlockSchema = BlockSchema {
        flavour: SURFACE,
        version: 1,
        role: Role::Root,
        parents: &[],
        children: &[NOTE],
    };

    fn schema_fn(f: Flavour) -> Option<BlockSchema> {
        match f {
            NOTE => Some(NOTE_SCHEMA),
            PROSE => Some(PROSE_SCHEMA),
            SURFACE => Some(SURFACE_SCHEMA),
            _ => None,
        }
    }

    fn make_store(blocks: Vec<BlockModel<()>>) -> HashMap<BlockId, BlockModel<()>> {
        blocks.into_iter().map(|b| (b.id, b)).collect()
    }

    fn store_fn<'a>(map: &'a HashMap<BlockId, BlockModel<()>>) -> impl Fn(BlockId) -> Option<&'a BlockModel<()>> {
        move |id| map.get(&id)
    }

    // Test 1: valid two-level tree.
    #[test]
    fn valid_tree_no_errors() {
        let mut note = BlockModel::new(1, NOTE, ());
        note.version = 1;
        note.add_child(2);
        let mut prose = BlockModel::new(2, PROSE, ());
        prose.version = 1;
        let store = make_store(vec![note, prose]);
        let report = validate_tree(&[1], schema_fn, store_fn(&store));
        assert!(report.is_valid());
        assert!(!report.has_warnings());
        assert_eq!(report.visited_count, 2);
    }

    // Test 2: IllegalChild — note contains a surface (not in allowlist).
    #[test]
    fn illegal_child_note_contains_surface() {
        let mut note = BlockModel::new(1, NOTE, ());
        note.version = 1;
        note.add_child(2);
        let mut surface = BlockModel::new(2, SURFACE, ());
        surface.version = 1;
        let store = make_store(vec![note, surface]);
        let report = validate_tree(&[1], schema_fn, store_fn(&store));
        assert!(!report.is_valid());
        assert!(report.errors.iter().any(|e| matches!(
            e,
            TreeValidationError::IllegalChild { parent: 1, child: 2, .. }
        )));
    }

    // Test 3: VersionTooNew emitted as error.
    #[test]
    fn version_too_new_is_error() {
        let mut note = BlockModel::new(1, NOTE, ());
        note.version = 99; // schema says 1
        let store = make_store(vec![note]);
        let report = validate_tree(&[1], schema_fn, store_fn(&store));
        assert!(!report.is_valid());
        assert!(report.errors.iter().any(|e| matches!(
            e,
            TreeValidationError::VersionTooNew { block: 1, schema: 1, found: 99 }
        )));
    }

    // Test 4: VersionTooOld emitted as warning, not error.
    #[test]
    fn version_too_old_is_warning_only() {
        let mut note = BlockModel::new(1, NOTE, ());
        note.version = 0; // schema says 1
        let store = make_store(vec![note]);
        let report = validate_tree(&[1], schema_fn, store_fn(&store));
        // Hub warning fires too; what matters is VersionTooOld is a warning not an error.
        assert!(report.errors.iter().all(|e| !matches!(e, TreeValidationError::VersionTooNew { .. })));
        assert!(report.warnings.iter().any(|w| matches!(
            w,
            TreeValidationWarning::VersionTooOld { block: 1, schema: 1, found: 0 }
        )));
    }

    // Test 5: DanglingChildRef.
    #[test]
    fn dangling_child_ref_emitted() {
        let mut note = BlockModel::new(1, NOTE, ());
        note.version = 1;
        note.add_child(999); // does not exist
        let store = make_store(vec![note]);
        let report = validate_tree(&[1], schema_fn, store_fn(&store));
        assert!(!report.is_valid());
        assert!(report.errors.iter().any(|e| matches!(
            e,
            TreeValidationError::DanglingChildRef { parent: 1, missing_child: 999 }
        )));
    }

    // Test 6: HubWithoutChildren warning.
    #[test]
    fn hub_without_children_warning() {
        let mut note = BlockModel::new(1, NOTE, ());
        note.version = 1;
        // no children added
        let store = make_store(vec![note]);
        let report = validate_tree(&[1], schema_fn, store_fn(&store));
        assert!(report.warnings.iter().any(|w| matches!(
            w,
            TreeValidationWarning::HubWithoutChildren { block: 1 }
        )));
    }

    // Test 7: Cycle detection (A.children = [B], B.children = [A]).
    #[test]
    fn cycle_detected() {
        let mut a = BlockModel::new(1, NOTE, ());
        a.version = 1;
        a.add_child(2);
        let mut b = BlockModel::new(2, NOTE, ());
        b.version = 1;
        b.add_child(1); // back-edge → cycle
        let store = make_store(vec![a, b]);
        let report = validate_tree(&[1], schema_fn, store_fn(&store));
        assert!(report.errors.iter().any(|e| matches!(e, TreeValidationError::Cycle { .. })));
    }

    // Test 8: Unknown flavour silently skipped.
    #[test]
    fn unknown_flavour_skipped_silently() {
        let block = BlockModel::new(1, "nom:plugin-unknown", ());
        let store = make_store(vec![block]);
        let report = validate_tree(&[1], schema_fn, store_fn(&store));
        // visited_count == 1 (we visited it) but no errors or warnings from schema checks
        assert!(report.is_valid());
        assert!(!report.has_warnings());
    }

    // Test 9: Multiple roots processed.
    #[test]
    fn multiple_roots_processed() {
        let mut note1 = BlockModel::new(1, NOTE, ());
        note1.version = 1;
        note1.add_child(3);
        let mut note2 = BlockModel::new(2, NOTE, ());
        note2.version = 1;
        note2.add_child(4);
        let mut prose1 = BlockModel::new(3, PROSE, ());
        prose1.version = 1;
        let mut prose2 = BlockModel::new(4, PROSE, ());
        prose2.version = 1;
        let store = make_store(vec![note1, note2, prose1, prose2]);
        let report = validate_tree(&[1, 2], schema_fn, store_fn(&store));
        assert!(report.is_valid());
        assert_eq!(report.visited_count, 4);
    }

    // Test 10: visited_count skips already-seen ids (shared-child scenario).
    #[test]
    fn shared_child_visited_once() {
        // Both roots share child 3 (PROSE).
        let mut note1 = BlockModel::new(1, NOTE, ());
        note1.version = 1;
        note1.add_child(3);
        let mut note2 = BlockModel::new(2, NOTE, ());
        note2.version = 1;
        note2.add_child(3);
        let mut shared_prose = BlockModel::new(3, PROSE, ());
        shared_prose.version = 1;
        let store = make_store(vec![note1, note2, shared_prose]);
        let report = validate_tree(&[1, 2], schema_fn, store_fn(&store));
        assert!(report.is_valid());
        // 1 + 2 + 3 each visited once = 3
        assert_eq!(report.visited_count, 3);
    }

    // Test 11: Report::is_valid and has_warnings behave correctly with mixed findings.
    #[test]
    fn mixed_findings_report_accessors() {
        // Block with old version (warning) + illegal child (error).
        let mut note = BlockModel::new(1, NOTE, ());
        note.version = 0; // triggers VersionTooOld warning
        note.add_child(2);
        let mut surface = BlockModel::new(2, SURFACE, ());
        surface.version = 1;
        let store = make_store(vec![note, surface]);
        let report = validate_tree(&[1], schema_fn, store_fn(&store));
        assert!(!report.is_valid());
        assert!(report.has_warnings());
    }

    // Test 12: error_from_schema adapter maps SchemaError to TreeValidationError correctly.
    #[test]
    fn error_from_schema_adapter() {
        use crate::block_schema::SchemaError;
        let err = SchemaError::ChildNotAllowed(SURFACE, NOTE);
        let tv_err = error_from_schema(1, NOTE, 2, SURFACE, err);
        assert_eq!(
            tv_err,
            TreeValidationError::IllegalChild {
                parent: 1,
                parent_flavour: NOTE,
                child: 2,
                child_flavour: SURFACE,
            }
        );
    }
}
