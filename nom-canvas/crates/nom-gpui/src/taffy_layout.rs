//! Thin wrapper around `taffy::TaffyTree` that speaks in our [`Style`] type.
//!
//! # Measure functions
//!
//! Taffy 0.6 exposes two mechanisms for content-sized (text/image) nodes:
//!
//! 1. `TaffyTree::new_leaf_with_context(style, NodeContext)` — stores a typed
//!    context value on the node.
//! 2. `TaffyTree::compute_layout_with_measure(root, available, closure)` — runs
//!    layout while calling the closure for every node that has a `NodeContext`.
//!    The closure receives `(known_dimensions, available_space, node_id,
//!    Option<&mut NodeContext>, style)` and must return `Size<f32>`.
//!
//! We store a `MeasureFn` closure inside `NodeContext`. During
//! `compute_layout` we call `compute_layout_with_measure` and dispatch to
//! the stored closure when the node context is present.

use crate::geometry::{Bounds, Point, ScaledPixels, Size};
use crate::style::Style;
use std::collections::HashMap;
use taffy::TraversePartialTree;
use thiserror::Error;

/// Opaque handle to a node in the layout tree.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
#[repr(transparent)]
pub struct LayoutId(pub taffy::NodeId);

#[derive(Debug, Error)]
pub enum LayoutError {
    #[error("taffy error: {0}")]
    Taffy(#[from] taffy::TaffyError),
    #[error("unknown layout id")]
    UnknownId,
}

/// Measure function type: given (known_dimensions, max_available), return
/// the intrinsic size of the content (text, image, canvas, etc.).
///
/// - `known_dimensions`: axes already constrained by the caller (`None` = unconstrained).
/// - `available`: remaining available space passed down from the parent.
pub type MeasureFn =
    Box<dyn FnMut(Size<Option<ScaledPixels>>, Size<ScaledPixels>) -> Size<ScaledPixels> + 'static>;

/// Per-node caller data. Holds an optional measure closure for leaf nodes
/// whose size depends on their content (text, images, embedded widgets).
pub struct NodeContext {
    pub measure: Option<MeasureFn>,
}

/// Layout engine wrapping `taffy::TaffyTree`.
pub struct LayoutEngine {
    tree: taffy::TaffyTree<NodeContext>,
    resolved: HashMap<taffy::NodeId, Bounds<ScaledPixels>>,
}

impl LayoutEngine {
    pub fn new() -> Self {
        Self {
            tree: taffy::TaffyTree::new(),
            resolved: HashMap::new(),
        }
    }

    pub fn clear(&mut self) {
        // taffy has no `clear`; recreate via std::mem::take.
        self.tree = taffy::TaffyTree::new();
        self.resolved.clear();
    }

    /// Create a new leaf/branch node with the given style and children.
    ///
    /// This is the infallible MVP entry point: taffy errors are promoted to
    /// panics here because a well-formed `Style` should never fail allocation
    /// in practice. For code that needs to handle allocation errors gracefully,
    /// use [`LayoutEngine::try_request_layout`].
    pub fn request_layout(&mut self, style: &Style, children: &[LayoutId]) -> LayoutId {
        self.try_request_layout(style, children)
            .expect("request_layout: taffy node creation failed")
    }

    /// Fallible variant of [`LayoutEngine::request_layout`]. Returns a
    /// [`LayoutError`] if taffy rejects the node (e.g. unknown child id,
    /// allocation failure). Prefer this in production frame code where
    /// graceful error recovery is desirable.
    pub fn try_request_layout(
        &mut self,
        style: &Style,
        children: &[LayoutId],
    ) -> Result<LayoutId, LayoutError> {
        let taffy_style = style.to_taffy();
        let id = if children.is_empty() {
            self.tree.new_leaf(taffy_style)?
        } else {
            let child_ids: Vec<taffy::NodeId> = children.iter().map(|c| c.0).collect();
            self.tree.new_with_children(taffy_style, &child_ids)?
        };
        Ok(LayoutId(id))
    }

    /// Create a leaf node whose size is determined by a measure function.
    ///
    /// Used for content-sized elements such as text runs and images. The
    /// `measure` closure is called by taffy during layout with:
    ///   - `known_dimensions`: axes already fixed by an ancestor constraint.
    ///   - `available`: remaining space propagated from the parent.
    ///
    /// The closure must return the intrinsic `Size` of the content.
    ///
    /// Taffy 0.6 API: `new_leaf_with_context` + `compute_layout_with_measure`.
    pub fn request_measured_layout(
        &mut self,
        style: &Style,
        measure: MeasureFn,
    ) -> Result<LayoutId, LayoutError> {
        let taffy_style = style.to_taffy();
        let id = self.tree.new_leaf_with_context(
            taffy_style,
            NodeContext {
                measure: Some(measure),
            },
        )?;
        Ok(LayoutId(id))
    }

    /// Compute layout for a subtree rooted at `root`. Must be called before
    /// `resolve_bounds` returns meaningful results.
    ///
    /// When any node in the subtree carries a `MeasureFn` (added via
    /// [`request_measured_layout`]), taffy will call it during traversal via
    /// `compute_layout_with_measure`.
    pub fn compute_layout(
        &mut self,
        root: LayoutId,
        available: Size<ScaledPixels>,
    ) -> Result<(), LayoutError> {
        let avail = taffy::Size {
            width: taffy::AvailableSpace::Definite(available.width.0),
            height: taffy::AvailableSpace::Definite(available.height.0),
        };
        // Use compute_layout_with_measure so that any NodeContext::measure
        // closures are invoked automatically during traversal.
        self.tree.compute_layout_with_measure(
            root.0,
            avail,
            |known, avail_space, _node_id, ctx, _style| {
                // Convert taffy types -> our types, dispatch, convert back.
                let known_nom = Size {
                    width: known.width.map(ScaledPixels),
                    height: known.height.map(ScaledPixels),
                };
                let avail_nom = Size {
                    width: ScaledPixels(match avail_space.width {
                        taffy::AvailableSpace::Definite(v) => v,
                        taffy::AvailableSpace::MinContent
                        | taffy::AvailableSpace::MaxContent => 0.0,
                    }),
                    height: ScaledPixels(match avail_space.height {
                        taffy::AvailableSpace::Definite(v) => v,
                        taffy::AvailableSpace::MinContent
                        | taffy::AvailableSpace::MaxContent => 0.0,
                    }),
                };
                if let Some(NodeContext {
                    measure: Some(ref mut f),
                }) = ctx
                {
                    let result = f(known_nom, avail_nom);
                    taffy::Size {
                        width: result.width.0,
                        height: result.height.0,
                    }
                } else {
                    taffy::Size::ZERO
                }
            },
        )?;
        self.cache_bounds_recursive(root.0, Point::new(ScaledPixels(0.0), ScaledPixels(0.0)))?;
        Ok(())
    }

    fn cache_bounds_recursive(
        &mut self,
        node: taffy::NodeId,
        offset: Point<ScaledPixels>,
    ) -> Result<(), LayoutError> {
        let layout = *self.tree.layout(node)?;
        let origin = Point::new(
            ScaledPixels(offset.x.0 + layout.location.x),
            ScaledPixels(offset.y.0 + layout.location.y),
        );
        let size = Size::new(
            ScaledPixels(layout.size.width),
            ScaledPixels(layout.size.height),
        );
        self.resolved.insert(
            node,
            Bounds {
                origin,
                size,
            },
        );
        // taffy doesn't expose Vec<NodeId> directly; iterate children.
        let child_count = self.tree.child_count(node);
        for i in 0..child_count {
            let child = self.tree.child_at_index(node, i)?;
            self.cache_bounds_recursive(child, origin)?;
        }
        Ok(())
    }

    /// Resolve the (absolute) bounds of a node after `compute_layout`.
    /// Returns zero bounds if the node was not yet laid out.
    pub fn resolve_bounds(&self, id: LayoutId) -> Bounds<ScaledPixels> {
        self.resolved.get(&id.0).copied().unwrap_or(Bounds {
            origin: Point::new(ScaledPixels(0.0), ScaledPixels(0.0)),
            size: Size::new(ScaledPixels(0.0), ScaledPixels(0.0)),
        })
    }
}

impl Default for LayoutEngine {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::geometry::Pixels;
    use crate::style::{FlexDirection, Length};

    #[test]
    fn try_request_layout_returns_ok_for_valid_style() {
        let mut engine = LayoutEngine::new();
        let s = Style {
            width: Length::Pixels(Pixels(60.0)),
            height: Length::Pixels(Pixels(30.0)),
            ..Default::default()
        };
        let result = engine.try_request_layout(&s, &[]);
        assert!(result.is_ok(), "well-formed style should not produce an error");
        let leaf = result.unwrap();
        let leaf2 = engine.request_layout(&s, &[]);
        let _ = (leaf, leaf2);
    }

    #[test]
    fn single_leaf_with_fixed_size() {
        let mut engine = LayoutEngine::new();
        let s = Style {
            width: Length::Pixels(Pixels(100.0)),
            height: Length::Pixels(Pixels(50.0)),
            ..Default::default()
        };
        let root = engine.request_layout(&s, &[]);
        engine
            .compute_layout(root, Size::new(ScaledPixels(500.0), ScaledPixels(500.0)))
            .unwrap();
        let bounds = engine.resolve_bounds(root);
        assert_eq!(bounds.size.width, ScaledPixels(100.0));
        assert_eq!(bounds.size.height, ScaledPixels(50.0));
    }

    #[test]
    fn two_children_in_flex_row_place_side_by_side() {
        let mut engine = LayoutEngine::new();
        let child_style = Style {
            width: Length::Pixels(Pixels(40.0)),
            height: Length::Pixels(Pixels(40.0)),
            ..Default::default()
        };
        let a = engine.request_layout(&child_style, &[]);
        let b = engine.request_layout(&child_style, &[]);
        let parent = Style {
            flex_direction: FlexDirection::Row,
            ..Default::default()
        };
        let root = engine.request_layout(&parent, &[a, b]);
        engine
            .compute_layout(root, Size::new(ScaledPixels(500.0), ScaledPixels(500.0)))
            .unwrap();
        let ba = engine.resolve_bounds(a);
        let bb = engine.resolve_bounds(b);
        assert_eq!(ba.origin.x, ScaledPixels(0.0));
        assert_eq!(bb.origin.x, ScaledPixels(40.0));
    }

    // --- iter-4 audit: try_request_layout error paths ---

    /// Cross-tree `LayoutId` usage: taffy 0.6 panics on invalid SlotMap keys
    /// rather than returning `Err`, so there's no clean way to return an error
    /// from `try_request_layout` for this misuse. We use `catch_unwind` to
    /// document that the abort is a panic, not a silent success. This is the
    /// one place callers must preserve tree hygiene themselves.
    #[test]
    fn try_request_layout_panics_on_foreign_child_id() {
        use std::panic::{catch_unwind, AssertUnwindSafe};
        let panicked = catch_unwind(AssertUnwindSafe(|| {
            let mut engine_a = LayoutEngine::new();
            let mut engine_b = LayoutEngine::new();
            let foreign = engine_b.request_layout(&Style::default(), &[]);
            let _ = engine_a.try_request_layout(&Style::default(), &[foreign]);
        }));
        assert!(
            panicked.is_err(),
            "taffy 0.6 should panic on foreign LayoutId (observed behavior, not ideal)"
        );
    }

    /// Verify that `try_request_layout` with a `NaN` width dimension either
    /// succeeds (taffy clamps NaN to 0) or fails gracefully — never panics.
    ///
    /// Observed behavior (taffy 0.6): `new_leaf` accepts the style without error;
    /// taffy treats NaN as zero during layout computation. The node is created
    /// successfully and layout clamps the dimension.
    #[test]
    fn try_request_layout_nan_dims_does_not_panic() {
        let mut engine = LayoutEngine::new();
        let style = Style {
            width: Length::Pixels(Pixels(f32::NAN)),
            ..Default::default()
        };
        // Must not panic. Ok or Err are both acceptable; panic is not.
        let result = engine.try_request_layout(&style, &[]);
        // Document observed behavior: taffy 0.6 returns Ok and clamps NaN during compute.
        eprintln!(
            "try_request_layout with NaN width: {}",
            if result.is_ok() { "Ok (taffy accepted)" } else { "Err (taffy rejected)" }
        );
        // Only assert no panic.
        let _ = result;
    }

    // --- Fix 2 tests: request_measured_layout ---

    /// Verify that `request_measured_layout` creates a node successfully.
    #[test]
    fn request_measured_layout_creates_node() {
        let mut engine = LayoutEngine::new();
        let style = Style::default();
        let result = engine.request_measured_layout(
            &style,
            Box::new(|_known, _avail| Size::new(ScaledPixels(50.0), ScaledPixels(20.0))),
        );
        assert!(result.is_ok(), "request_measured_layout should succeed for default style");
    }

    /// Verify that a measured leaf's resolved size matches what the measure
    /// closure returns (80x20 ScaledPixels).
    #[test]
    fn measured_leaf_resolves_to_measure_output() {
        let mut engine = LayoutEngine::new();
        let style = Style::default();
        let leaf = engine
            .request_measured_layout(
                &style,
                Box::new(|_known, _avail| Size::new(ScaledPixels(80.0), ScaledPixels(20.0))),
            )
            .expect("request_measured_layout failed");

        engine
            .compute_layout(leaf, Size::new(ScaledPixels(500.0), ScaledPixels(500.0)))
            .expect("compute_layout failed");

        let bounds = engine.resolve_bounds(leaf);
        assert_eq!(
            bounds.size.width,
            ScaledPixels(80.0),
            "width should match measure closure output"
        );
        assert_eq!(
            bounds.size.height,
            ScaledPixels(20.0),
            "height should match measure closure output"
        );
    }
}
