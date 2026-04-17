//! Ergonomic Scene-builder with auto draw-order assignment.
//!
//! Users call `push_quad` / `push_shadow` / etc. in paint order; the builder
//! tracks a monotonically-incrementing DrawOrder internally so callers
//! don't have to thread it by hand.
#![deny(unsafe_code)]

use crate::bounds_tree::DrawOrder;
use crate::geometry::{Bounds, Pixels};

/// Strategy for assigning draw orders.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OrderingStrategy {
    /// Monotonically increasing — later pushes paint on top.
    Sequential,
    /// Each z_layer bucket gets its own range (100..200 for layer 1, 200..300
    /// for layer 2, etc.). Within a layer, sequential.
    Layered,
}

/// Ergonomic builder that auto-manages `DrawOrder` while collecting primitives.
///
/// Call the `push_*` methods in paint order; each returns the assigned
/// `DrawOrder`. When done, inspect via [`SceneBuilder::collected`].
pub struct SceneBuilder {
    next_order: u32,
    current_layer: u32,
    strategy: OrderingStrategy,
    collected: Vec<CollectedPrimitive>,
}

/// A primitive collected by [`SceneBuilder`] before GPU submission.
#[derive(Clone, Debug, PartialEq)]
pub enum CollectedPrimitive {
    Quad { bounds: Bounds<Pixels>, order: DrawOrder },
    Shadow { bounds: Bounds<Pixels>, blur_radius_px: f32, order: DrawOrder },
    MonoSprite { bounds: Bounds<Pixels>, order: DrawOrder },
    Underline { bounds: Bounds<Pixels>, order: DrawOrder },
}

impl SceneBuilder {
    /// Create a new builder with `Sequential` strategy and order counter at 0.
    pub fn new() -> Self {
        Self {
            next_order: 0,
            current_layer: 0,
            strategy: OrderingStrategy::Sequential,
            collected: Vec::new(),
        }
    }

    /// Set the ordering strategy (builder-chain style).
    pub fn with_strategy(mut self, strategy: OrderingStrategy) -> Self {
        self.strategy = strategy;
        self
    }

    /// Change the active z-layer.
    ///
    /// Under `Layered` strategy the counter jumps to `layer * 100` if it has
    /// not already passed that base. Under `Sequential` the counter is
    /// unchanged (layer is recorded but has no numeric effect).
    pub fn set_layer(&mut self, layer: u32) {
        self.current_layer = layer;
        if self.strategy == OrderingStrategy::Layered {
            let base = layer.saturating_mul(100);
            if self.next_order < base {
                self.next_order = base;
            }
        }
    }

    /// Consume the next order value and advance the counter.
    fn next(&mut self) -> DrawOrder {
        let order = self.next_order;
        self.next_order = self.next_order.saturating_add(1);
        order
    }

    /// Push a quad primitive; returns its assigned `DrawOrder`.
    pub fn push_quad(&mut self, bounds: Bounds<Pixels>) -> DrawOrder {
        let order = self.next();
        self.collected.push(CollectedPrimitive::Quad { bounds, order });
        order
    }

    /// Push a shadow primitive; returns its assigned `DrawOrder`.
    pub fn push_shadow(&mut self, bounds: Bounds<Pixels>, blur_radius_px: f32) -> DrawOrder {
        let order = self.next();
        self.collected
            .push(CollectedPrimitive::Shadow { bounds, blur_radius_px, order });
        order
    }

    /// Push a monochrome sprite primitive; returns its assigned `DrawOrder`.
    pub fn push_mono_sprite(&mut self, bounds: Bounds<Pixels>) -> DrawOrder {
        let order = self.next();
        self.collected.push(CollectedPrimitive::MonoSprite { bounds, order });
        order
    }

    /// Push an underline primitive; returns its assigned `DrawOrder`.
    pub fn push_underline(&mut self, bounds: Bounds<Pixels>) -> DrawOrder {
        let order = self.next();
        self.collected.push(CollectedPrimitive::Underline { bounds, order });
        order
    }

    /// Number of collected primitives.
    pub fn len(&self) -> usize {
        self.collected.len()
    }

    /// `true` when no primitives have been pushed yet.
    pub fn is_empty(&self) -> bool {
        self.collected.is_empty()
    }

    /// Slice of all collected primitives in push order.
    pub fn collected(&self) -> &[CollectedPrimitive] {
        &self.collected
    }

    /// `DrawOrder` values of all collected primitives in push order.
    pub fn orders(&self) -> Vec<DrawOrder> {
        self.collected
            .iter()
            .map(|p| match p {
                CollectedPrimitive::Quad { order, .. } => *order,
                CollectedPrimitive::Shadow { order, .. } => *order,
                CollectedPrimitive::MonoSprite { order, .. } => *order,
                CollectedPrimitive::Underline { order, .. } => *order,
            })
            .collect()
    }
}

impl Default for SceneBuilder {
    fn default() -> Self {
        Self::new()
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::geometry::{Point, Size};

    fn px_bounds(x: f32, y: f32, w: f32, h: f32) -> Bounds<Pixels> {
        Bounds::new(
            Point::new(Pixels(x), Pixels(y)),
            Size::new(Pixels(w), Pixels(h)),
        )
    }

    fn b() -> Bounds<Pixels> {
        px_bounds(0.0, 0.0, 10.0, 10.0)
    }

    // 1. new() — starts empty with counter at 0
    #[test]
    fn new_starts_empty() {
        let sb = SceneBuilder::new();
        assert!(sb.is_empty());
        assert_eq!(sb.len(), 0);
    }

    // 2. push_quad returns monotonically increasing DrawOrders (0, 1, 2)
    #[test]
    fn push_quad_monotonic_orders() {
        let mut sb = SceneBuilder::new();
        let o0 = sb.push_quad(b());
        let o1 = sb.push_quad(b());
        let o2 = sb.push_quad(b());
        assert_eq!(o0, 0);
        assert_eq!(o1, 1);
        assert_eq!(o2, 2);
    }

    // 3. len + is_empty reflect pushes
    #[test]
    fn len_and_is_empty() {
        let mut sb = SceneBuilder::new();
        assert!(sb.is_empty());
        sb.push_quad(b());
        assert!(!sb.is_empty());
        assert_eq!(sb.len(), 1);
        sb.push_shadow(b(), 4.0);
        assert_eq!(sb.len(), 2);
    }

    // 4. orders() returns all orders in push sequence
    #[test]
    fn orders_in_push_sequence() {
        let mut sb = SceneBuilder::new();
        sb.push_quad(b());
        sb.push_shadow(b(), 2.0);
        sb.push_mono_sprite(b());
        sb.push_underline(b());
        assert_eq!(sb.orders(), vec![0, 1, 2, 3]);
    }

    // 5. set_layer under Layered strategy jumps next_order to layer base
    #[test]
    fn layered_strategy_jumps_to_base() {
        let mut sb = SceneBuilder::new().with_strategy(OrderingStrategy::Layered);
        sb.set_layer(1);
        let o = sb.push_quad(b());
        assert_eq!(o, 100, "layer 1 base should be 100");

        sb.set_layer(2);
        let o2 = sb.push_quad(b());
        assert_eq!(o2, 200, "layer 2 base should be 200");
    }

    // 6. set_layer under Sequential does NOT change next_order
    #[test]
    fn sequential_strategy_ignores_layer() {
        let mut sb = SceneBuilder::new(); // Sequential by default
        sb.push_quad(b()); // order 0
        sb.push_quad(b()); // order 1
        sb.set_layer(5);   // should not jump to 500
        let o = sb.push_quad(b());
        assert_eq!(o, 2, "Sequential ignores set_layer — order must continue from 2");
    }

    // 7. with_strategy builder chain
    #[test]
    fn with_strategy_chain() {
        let sb = SceneBuilder::new().with_strategy(OrderingStrategy::Layered);
        assert_eq!(sb.strategy, OrderingStrategy::Layered);
    }

    // 8. push_shadow preserves blur_radius_px
    #[test]
    fn push_shadow_blur_radius_preserved() {
        let mut sb = SceneBuilder::new();
        let bounds = px_bounds(5.0, 5.0, 20.0, 20.0);
        sb.push_shadow(bounds, 8.5);
        match &sb.collected()[0] {
            CollectedPrimitive::Shadow { blur_radius_px, .. } => {
                assert!((blur_radius_px - 8.5).abs() < f32::EPSILON);
            }
            other => panic!("expected Shadow, got {other:?}"),
        }
    }

    // 9. push_mono_sprite and push_underline also assign incrementing orders
    #[test]
    fn mono_sprite_and_underline_increment() {
        let mut sb = SceneBuilder::new();
        let a = sb.push_mono_sprite(b());
        let b_ = sb.push_underline(b());
        assert_eq!(a, 0);
        assert_eq!(b_, 1);
    }

    // 10. collected() contains all four variant types
    #[test]
    fn collected_contains_all_variants() {
        let mut sb = SceneBuilder::new();
        sb.push_quad(b());
        sb.push_shadow(b(), 1.0);
        sb.push_mono_sprite(b());
        sb.push_underline(b());
        let prims = sb.collected();
        assert_eq!(prims.len(), 4);
        assert!(matches!(prims[0], CollectedPrimitive::Quad { .. }));
        assert!(matches!(prims[1], CollectedPrimitive::Shadow { .. }));
        assert!(matches!(prims[2], CollectedPrimitive::MonoSprite { .. }));
        assert!(matches!(prims[3], CollectedPrimitive::Underline { .. }));
    }

    // 11. saturating_add: at u32::MAX the counter stays at MAX
    #[test]
    fn saturating_next_at_max() {
        let mut sb = SceneBuilder::new();
        sb.next_order = u32::MAX;
        let o = sb.push_quad(b());
        assert_eq!(o, u32::MAX);
        // A second push: counter saturated at MAX, returns MAX again.
        let o2 = sb.push_quad(b());
        assert_eq!(o2, u32::MAX);
    }

    // 12. default() equals new()
    #[test]
    fn default_equals_new() {
        let sb = SceneBuilder::default();
        assert!(sb.is_empty());
        assert_eq!(sb.strategy, OrderingStrategy::Sequential);
    }
}
