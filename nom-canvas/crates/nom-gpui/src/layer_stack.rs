/// Classifies a layer by its rendering role within the compositor stack.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LayerKind {
    Background,
    Content,
    Overlay,
    Popup,
    Debug,
}

impl LayerKind {
    /// Returns the base z-order index for this kind.
    pub fn z_order(&self) -> u8 {
        match self {
            LayerKind::Background => 0,
            LayerKind::Content => 1,
            LayerKind::Overlay => 2,
            LayerKind::Popup => 3,
            LayerKind::Debug => 255,
        }
    }

    /// Returns true if this kind always renders on top of normal content.
    pub fn is_always_on_top(&self) -> bool {
        matches!(self, LayerKind::Popup | LayerKind::Debug)
    }
}

/// Newtype wrapper around a u32 layer identifier.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LayerId(pub u32);

impl LayerId {
    /// Returns true when this is the root (sentinel) layer with id 0.
    pub fn is_root(&self) -> bool {
        self.0 == 0
    }

    /// Returns a human-readable string representation of the id.
    pub fn id_str(&self) -> String {
        format!("layer:{}", self.0)
    }
}

/// A single compositable layer with identity, kind, visibility, and opacity.
#[derive(Debug, Clone)]
pub struct Layer {
    pub id: LayerId,
    pub kind: LayerKind,
    pub visible: bool,
    pub opacity: f32,
}

impl Layer {
    /// Returns true only when both `visible` is set and `opacity` is non-zero.
    pub fn is_visible(&self) -> bool {
        self.visible && self.opacity > 0.0
    }

    /// Combines kind z-order with layer id to produce a stable sort key.
    ///
    /// Formula: `kind.z_order() as u32 * 1000 + id.0`
    pub fn effective_z(&self) -> u32 {
        self.kind.z_order() as u32 * 1000 + self.id.0
    }
}

/// An ordered stack of [`Layer`] values that models the compositor draw list.
#[derive(Debug, Default)]
pub struct LayerStack {
    pub layers: Vec<Layer>,
}

impl LayerStack {
    /// Creates an empty stack.
    pub fn new() -> Self {
        LayerStack { layers: Vec::new() }
    }

    /// Appends a layer to the top of the stack.
    pub fn push(&mut self, layer: Layer) {
        self.layers.push(layer);
    }

    /// Removes and returns the top-most layer, or `None` when empty.
    pub fn pop(&mut self) -> Option<Layer> {
        self.layers.pop()
    }

    /// Returns references to all visible layers, sorted by effective z ascending.
    pub fn visible_layers(&self) -> Vec<&Layer> {
        let mut visible: Vec<&Layer> =
            self.layers.iter().filter(|l| l.is_visible()).collect();
        visible.sort_by_key(|l| l.effective_z());
        visible
    }

    /// Returns the total number of layers (visible or not).
    pub fn depth(&self) -> usize {
        self.layers.len()
    }
}

/// Stateless helper that performs compositor-level queries on a [`LayerStack`].
pub struct LayerCompositor;

impl LayerCompositor {
    /// Returns the top-most visible layer (highest effective z), if any.
    pub fn top_visible(stack: &LayerStack) -> Option<&Layer> {
        stack
            .layers
            .iter()
            .filter(|l| l.is_visible())
            .max_by_key(|l| l.effective_z())
    }

    /// Counts layers whose `z_order` matches the given kind's `z_order`.
    pub fn count_by_kind(stack: &LayerStack, kind: &LayerKind) -> usize {
        stack
            .layers
            .iter()
            .filter(|l| l.kind.z_order() == kind.z_order())
            .count()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // 1. LayerKind::z_order ordering is strictly ascending for the main variants.
    #[test]
    fn layer_kind_z_order_ordering() {
        assert!(LayerKind::Background.z_order() < LayerKind::Content.z_order());
        assert!(LayerKind::Content.z_order() < LayerKind::Overlay.z_order());
        assert!(LayerKind::Overlay.z_order() < LayerKind::Popup.z_order());
        assert!(LayerKind::Popup.z_order() < LayerKind::Debug.z_order());
        assert_eq!(LayerKind::Debug.z_order(), 255);
    }

    // 2. is_always_on_top is true only for Popup and Debug.
    #[test]
    fn layer_kind_is_always_on_top() {
        assert!(!LayerKind::Background.is_always_on_top());
        assert!(!LayerKind::Content.is_always_on_top());
        assert!(!LayerKind::Overlay.is_always_on_top());
        assert!(LayerKind::Popup.is_always_on_top());
        assert!(LayerKind::Debug.is_always_on_top());
    }

    // 3. LayerId::is_root is true only when the inner value is 0.
    #[test]
    fn layer_id_is_root() {
        assert!(LayerId(0).is_root());
        assert!(!LayerId(1).is_root());
        assert!(!LayerId(42).is_root());
    }

    // 4. Layer::is_visible returns false when opacity is 0.0.
    #[test]
    fn layer_is_visible_false_when_opacity_zero() {
        let layer = Layer {
            id: LayerId(1),
            kind: LayerKind::Content,
            visible: true,
            opacity: 0.0,
        };
        assert!(!layer.is_visible());
    }

    // 5. Layer::effective_z follows the documented formula.
    #[test]
    fn layer_effective_z_formula() {
        let layer = Layer {
            id: LayerId(7),
            kind: LayerKind::Overlay, // z_order = 2
            visible: true,
            opacity: 1.0,
        };
        // expected: 2 * 1000 + 7 = 2007
        assert_eq!(layer.effective_z(), 2007);
    }

    // 6. LayerStack::push increases depth by one per call.
    #[test]
    fn layer_stack_push_depth() {
        let mut stack = LayerStack::new();
        assert_eq!(stack.depth(), 0);

        stack.push(Layer { id: LayerId(1), kind: LayerKind::Background, visible: true, opacity: 1.0 });
        assert_eq!(stack.depth(), 1);

        stack.push(Layer { id: LayerId(2), kind: LayerKind::Content, visible: true, opacity: 0.5 });
        assert_eq!(stack.depth(), 2);
    }

    // 7. visible_layers returns only visible layers sorted by effective_z ascending.
    #[test]
    fn layer_stack_visible_layers_sorted() {
        let mut stack = LayerStack::new();
        // Overlay id=5 → effective_z = 2*1000+5 = 2005
        stack.push(Layer { id: LayerId(5), kind: LayerKind::Overlay, visible: true, opacity: 1.0 });
        // Background id=1 → effective_z = 0*1000+1 = 1  (lowest)
        stack.push(Layer { id: LayerId(1), kind: LayerKind::Background, visible: true, opacity: 1.0 });
        // Invisible Content layer — must be excluded
        stack.push(Layer { id: LayerId(2), kind: LayerKind::Content, visible: false, opacity: 1.0 });

        let visible = stack.visible_layers();
        assert_eq!(visible.len(), 2);
        assert_eq!(visible[0].id.0, 1);   // Background comes first (z=1)
        assert_eq!(visible[1].id.0, 5);   // Overlay comes second (z=2005)
    }

    // 8. LayerCompositor::top_visible returns the layer with the highest effective_z.
    #[test]
    fn layer_compositor_top_visible() {
        let mut stack = LayerStack::new();
        stack.push(Layer { id: LayerId(1), kind: LayerKind::Background, visible: true, opacity: 1.0 });
        stack.push(Layer { id: LayerId(2), kind: LayerKind::Popup, visible: true, opacity: 1.0 });
        stack.push(Layer { id: LayerId(3), kind: LayerKind::Content, visible: true, opacity: 1.0 });

        let top = LayerCompositor::top_visible(&stack).expect("should have a top visible layer");
        // Popup id=2 → z = 3*1000+2 = 3002 (highest)
        assert_eq!(top.id.0, 2);
    }

    // 9. LayerCompositor::count_by_kind counts only layers of the matching kind.
    #[test]
    fn layer_compositor_count_by_kind() {
        let mut stack = LayerStack::new();
        stack.push(Layer { id: LayerId(1), kind: LayerKind::Content, visible: true, opacity: 1.0 });
        stack.push(Layer { id: LayerId(2), kind: LayerKind::Content, visible: false, opacity: 0.0 });
        stack.push(Layer { id: LayerId(3), kind: LayerKind::Overlay, visible: true, opacity: 1.0 });

        assert_eq!(LayerCompositor::count_by_kind(&stack, &LayerKind::Content), 2);
        assert_eq!(LayerCompositor::count_by_kind(&stack, &LayerKind::Overlay), 1);
        assert_eq!(LayerCompositor::count_by_kind(&stack, &LayerKind::Background), 0);
    }
}
