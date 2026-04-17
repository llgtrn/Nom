//! Thin wrapper around `taffy::TaffyTree` that speaks in our [`Style`] type.

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

/// Layout engine wrapping `taffy::TaffyTree`.
pub struct LayoutEngine {
    tree: taffy::TaffyTree<NodeContext>,
    resolved: HashMap<taffy::NodeId, Bounds<ScaledPixels>>,
}

/// Per-node caller data (reserved for measure functions / ids).
#[derive(Debug, Default)]
pub struct NodeContext {}

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

    /// Compute layout for a subtree rooted at `root`. Must be called before
    /// `resolve_bounds` returns meaningful results.
    pub fn compute_layout(
        &mut self,
        root: LayoutId,
        available: Size<ScaledPixels>,
    ) -> Result<(), LayoutError> {
        let avail = taffy::Size {
            width: taffy::AvailableSpace::Definite(available.width.0),
            height: taffy::AvailableSpace::Definite(available.height.0),
        };
        self.tree.compute_layout(root.0, avail)?;
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
        // Also verify request_layout (infallible wrapper) gives equivalent result.
        let leaf2 = engine.request_layout(&s, &[]);
        // Both are distinct nodes but structurally equivalent — just confirm no panic.
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
}
