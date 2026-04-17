use crate::types::*;
use crate::styled::{StyleRefinement, Styled};

// ---------------------------------------------------------------------------
// LayoutRegistry — hands out unique, incrementing LayoutIds
// ---------------------------------------------------------------------------

/// Registry that issues unique `LayoutId`s.
///
/// Each call to `next_id` returns a monotonically increasing ID. This ensures
/// that different elements receive different IDs even in the stub (no-taffy)
/// implementation.
pub struct LayoutRegistry {
    next: u64,
}

impl LayoutRegistry {
    pub fn new() -> Self {
        Self { next: 1 }
    }

    pub fn next_id(&mut self) -> LayoutId {
        let id = LayoutId(self.next);
        self.next += 1;
        id
    }
}

impl Default for LayoutRegistry {
    fn default() -> Self {
        Self::new()
    }
}

// ---------------------------------------------------------------------------
// WindowContext
// ---------------------------------------------------------------------------

/// Context provided to elements during layout/paint phases.
pub struct WindowContext {
    pub scale_factor: f32,
    pub viewport_size: Vec2,
    layout_registry: LayoutRegistry,
}

impl WindowContext {
    pub fn new(scale_factor: f32, viewport_size: Vec2) -> Self {
        Self { scale_factor, viewport_size, layout_registry: LayoutRegistry::new() }
    }

    pub fn rem_size(&self) -> Pixels { Pixels(16.0 * self.scale_factor) }

    /// Delegates to taffy via the layout engine (stub implementation).
    /// Returns a unique, non-zero `LayoutId` per call.
    pub fn request_layout(
        &mut self,
        _style: &StyleRefinement,
        _children: &[LayoutId],
    ) -> LayoutId {
        self.layout_registry.next_id()
    }

    pub fn layout(&self, _id: LayoutId) -> Bounds<Pixels> {
        Bounds::default()
    }
}

// ---------------------------------------------------------------------------
// Element trait — three-phase GPU element lifecycle
// ---------------------------------------------------------------------------

/// The core GPU element trait — three phases per frame.
/// Pattern: Zed GPUI Element (APP/zed-main/crates/gpui/src/element.rs)
///
/// Phase 1: `request_layout` — register taffy node, return `LayoutId` + opaque state.
/// Phase 2: `prepaint`       — register hitboxes, prepare data (NO GPU calls).
/// Phase 3: `paint`          — emit primitives to Scene.
pub trait Element {
    type State;

    /// Phase 1: Request layout computation. Returns `(LayoutId, State)`.
    /// Called during layout traversal before any painting.
    fn request_layout(
        &mut self,
        global_id: Option<&GlobalElementId>,
        cx: &mut WindowContext,
    ) -> (LayoutId, Self::State);

    /// Phase 2: Preparation (hit testing, cursor changes). NO GPU calls.
    /// `bounds` = computed layout bounds from Phase 1.
    fn prepaint(
        &mut self,
        global_id: Option<&GlobalElementId>,
        bounds: Bounds<Pixels>,
        state: &mut Self::State,
        cx: &mut WindowContext,
    );

    /// Phase 3: Emit GPU primitives to Scene.
    fn paint(
        &mut self,
        global_id: Option<&GlobalElementId>,
        bounds: Bounds<Pixels>,
        state: &mut Self::State,
        cx: &mut WindowContext,
    );
}

// ---------------------------------------------------------------------------
// AnyElement — type-erased element for heterogeneous collections
// ---------------------------------------------------------------------------

/// Type-erased element for storage in heterogeneous collections.
pub trait AnyElement {
    fn request_layout_dyn(
        &mut self,
        global_id: Option<&GlobalElementId>,
        cx: &mut WindowContext,
    ) -> LayoutId;

    fn prepaint_dyn(
        &mut self,
        global_id: Option<&GlobalElementId>,
        bounds: Bounds<Pixels>,
        cx: &mut WindowContext,
    );

    fn paint_dyn(
        &mut self,
        global_id: Option<&GlobalElementId>,
        bounds: Bounds<Pixels>,
        cx: &mut WindowContext,
    );
}

// ---------------------------------------------------------------------------
// Div — canonical example element
// ---------------------------------------------------------------------------

/// A flex container element — the canonical building block.
pub struct Div {
    pub style: StyleRefinement,
    pub children: Vec<Box<dyn AnyElement>>,
}

impl Div {
    pub fn new() -> Self {
        Self {
            style: StyleRefinement::default(),
            children: Vec::new(),
        }
    }

    pub fn child(mut self, child: impl AnyElement + 'static) -> Self {
        self.children.push(Box::new(child));
        self
    }
}

impl Default for Div {
    fn default() -> Self { Self::new() }
}

impl Styled for Div {
    fn style(&mut self) -> &mut StyleRefinement { &mut self.style }
}

impl Element for Div {
    type State = ();

    fn request_layout(
        &mut self,
        _global_id: Option<&GlobalElementId>,
        cx: &mut WindowContext,
    ) -> (LayoutId, ()) {
        let id = cx.request_layout(&self.style, &[]);
        (id, ())
    }

    fn prepaint(
        &mut self,
        _global_id: Option<&GlobalElementId>,
        _bounds: Bounds<Pixels>,
        _state: &mut (),
        _cx: &mut WindowContext,
    ) {
        // Hit-test registration would happen here in a real impl
    }

    fn paint(
        &mut self,
        _global_id: Option<&GlobalElementId>,
        _bounds: Bounds<Pixels>,
        _state: &mut (),
        _cx: &mut WindowContext,
    ) {
        // Emit background quad to Scene in a real impl
    }
}

// AnyElement blanket adapter for Div so it can be stored as Box<dyn AnyElement>
impl AnyElement for Div {
    fn request_layout_dyn(
        &mut self,
        global_id: Option<&GlobalElementId>,
        cx: &mut WindowContext,
    ) -> LayoutId {
        let (id, _state) = Element::request_layout(self, global_id, cx);
        id
    }

    fn prepaint_dyn(
        &mut self,
        global_id: Option<&GlobalElementId>,
        bounds: Bounds<Pixels>,
        cx: &mut WindowContext,
    ) {
        let mut state = ();
        Element::prepaint(self, global_id, bounds, &mut state, cx);
    }

    fn paint_dyn(
        &mut self,
        global_id: Option<&GlobalElementId>,
        bounds: Bounds<Pixels>,
        cx: &mut WindowContext,
    ) {
        let mut state = ();
        Element::paint(self, global_id, bounds, &mut state, cx);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_cx() -> WindowContext {
        WindowContext::new(2.0, Vec2::new(1920.0, 1080.0))
    }

    #[test]
    fn div_new_constructs_empty() {
        let div = Div::new();
        assert_eq!(div.children.len(), 0);
        assert!(div.style.background.is_none());
    }

    #[test]
    fn div_child_adds_element() {
        let child = Div::new();
        let parent = Div::new().child(child);
        assert_eq!(parent.children.len(), 1);
    }

    #[test]
    fn global_element_id_push_pop() {
        let mut gid = GlobalElementId::new();
        gid.push(ElementId::new(1));
        gid.push(ElementId::new(2));
        assert_eq!(gid.pop(), Some(ElementId::new(2)));
        assert_eq!(gid.pop(), Some(ElementId::new(1)));
        assert_eq!(gid.pop(), None);
    }

    #[test]
    fn div_request_layout_returns_layout_id() {
        let mut div = Div::new();
        let mut cx = make_cx();
        let (id, _state) = Element::request_layout(&mut div, None, &mut cx);
        // Registry starts at 1, so first id must be non-zero.
        assert_ne!(id, LayoutId(0), "layout id must be non-zero");
    }

    #[test]
    fn window_context_rem_size_scales_with_dpi() {
        let cx = WindowContext::new(2.0, Vec2::new(800.0, 600.0));
        assert_eq!(cx.rem_size(), Pixels(32.0));
    }

    #[test]
    fn layout_ids_are_unique() {
        let mut cx = make_cx();
        let style = crate::styled::StyleRefinement::default();

        let id1 = cx.request_layout(&style, &[]);
        let id2 = cx.request_layout(&style, &[]);
        let id3 = cx.request_layout(&style, &[]);

        assert_ne!(id1, id2, "id1 and id2 must differ");
        assert_ne!(id2, id3, "id2 and id3 must differ");
        assert_ne!(id1, id3, "id1 and id3 must differ");
    }

    #[test]
    fn layout_registry_starts_at_one() {
        let mut registry = LayoutRegistry::new();
        assert_eq!(registry.next_id(), LayoutId(1));
        assert_eq!(registry.next_id(), LayoutId(2));
        assert_eq!(registry.next_id(), LayoutId(3));
    }
}
