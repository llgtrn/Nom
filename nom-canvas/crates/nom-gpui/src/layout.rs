use crate::types::*;
use crate::styled::StyleRefinement;

/// Taffy layout engine integration.
/// `LayoutId` is a newtype over taffy's NodeId (u64 here — taffy is not linked yet).
/// Pattern: Zed (APP/zed-main/crates/gpui/src/taffy.rs)
pub struct LayoutEngine {
    next_id: u64,
    layouts: std::collections::HashMap<LayoutId, Bounds<Pixels>>,
}

impl LayoutEngine {
    pub fn new() -> Self {
        Self {
            next_id: 1,
            layouts: std::collections::HashMap::new(),
        }
    }

    /// Request a new layout node. Returns a `LayoutId`.
    /// In a real impl this would create a `taffy::NodeId` and store style + children.
    pub fn request_layout(
        &mut self,
        _style: &StyleRefinement,
        _children: &[LayoutId],
    ) -> LayoutId {
        let id = LayoutId(self.next_id);
        self.next_id += 1;
        self.layouts.insert(id, Bounds::default());
        id
    }

    /// Get computed bounds for a layout node.
    pub fn layout(&self, id: LayoutId) -> Bounds<Pixels> {
        self.layouts.get(&id).copied().unwrap_or_default()
    }

    /// Free a layout node (called on element drop).
    pub fn remove_layout_id(&mut self, id: LayoutId) {
        self.layouts.remove(&id);
    }

    /// Compute layout for the tree rooted at `root_id`, given available space.
    /// In a real impl this calls `taffy::compute_layout()`.
    pub fn compute_layout(&mut self, root_id: LayoutId, available: Size<Pixels>) {
        if let Some(layout) = self.layouts.get_mut(&root_id) {
            layout.size = available;
        }
    }
}

impl Default for LayoutEngine {
    fn default() -> Self { Self::new() }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn request_layout_returns_incrementing_ids() {
        let mut engine = LayoutEngine::new();
        let style = StyleRefinement::default();
        let id1 = engine.request_layout(&style, &[]);
        let id2 = engine.request_layout(&style, &[]);
        let id3 = engine.request_layout(&style, &[]);
        assert_eq!(id1, LayoutId(1));
        assert_eq!(id2, LayoutId(2));
        assert_eq!(id3, LayoutId(3));
    }

    #[test]
    fn layout_returns_default_bounds_initially() {
        let mut engine = LayoutEngine::new();
        let style = StyleRefinement::default();
        let id = engine.request_layout(&style, &[]);
        assert_eq!(engine.layout(id), Bounds::default());
    }

    #[test]
    fn remove_layout_id_drops_entry() {
        let mut engine = LayoutEngine::new();
        let style = StyleRefinement::default();
        let id = engine.request_layout(&style, &[]);
        engine.remove_layout_id(id);
        // After removal, layout() should return default (missing key path)
        assert_eq!(engine.layout(id), Bounds::default());
    }

    #[test]
    fn compute_layout_sets_size() {
        let mut engine = LayoutEngine::new();
        let style = StyleRefinement::default();
        let id = engine.request_layout(&style, &[]);
        let available = Size { width: Pixels(800.0), height: Pixels(600.0) };
        engine.compute_layout(id, available);
        assert_eq!(engine.layout(id).size, available);
    }
}
