//! 8 resize handles + 1 rotation handle for selected elements.
//!
//! Handle positions are computed in canvas (logical-pixel) space from the
//! element's axis-aligned bounding box.  Sizes are divided by the current
//! zoom so handles appear at a constant on-screen size regardless of zoom
//! level, matching the Excalidraw `transformHandleSizes[pointerType] / zoom`
//! formula.

#![deny(unsafe_code)]

use smallvec::SmallVec;

use nom_gpui::{Bounds, Pixels, Point};

// ── HandleKind ────────────────────────────────────────────────────────────────

/// One of the nine interactive handles surrounding a selected element.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum HandleKind {
    /// Top-centre.
    N,
    /// Bottom-centre.
    S,
    /// Right-centre.
    E,
    /// Left-centre.
    W,
    /// Top-right corner.
    Ne,
    /// Top-left corner.
    Nw,
    /// Bottom-right corner.
    Se,
    /// Bottom-left corner.
    Sw,
    /// Rotation handle (above top-centre).
    Rotation,
}

// ── PointerKind ───────────────────────────────────────────────────────────────

/// Input device that drives pointer interaction.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PointerKind {
    Mouse,
    Pen,
    Touch,
}

// ── size constants ────────────────────────────────────────────────────────────

/// Base handle size for mouse input (pixels on-screen before zoom division).
pub const HANDLE_SIZE_MOUSE: f32 = 8.0;
/// Base handle size for pen input.
pub const HANDLE_SIZE_PEN: f32 = 16.0;
/// Base handle size for touch input.
pub const HANDLE_SIZE_TOUCH: f32 = 28.0;
/// Gap between the element's top edge and the rotation handle centre.
pub const ROTATION_HANDLE_OFFSET_PX: f32 = 24.0;

fn base_handle_size(pointer: PointerKind) -> f32 {
    match pointer {
        PointerKind::Mouse => HANDLE_SIZE_MOUSE,
        PointerKind::Pen => HANDLE_SIZE_PEN,
        PointerKind::Touch => HANDLE_SIZE_TOUCH,
    }
}

// ── TransformHandles ──────────────────────────────────────────────────────────

/// Geometry for the 8 resize + 1 rotation handles around a selected element.
pub struct TransformHandles {
    /// The element's axis-aligned bounding box in canvas space.
    pub bounds: Bounds<Pixels>,
    /// Current zoom factor (1.0 = 100 %).
    pub zoom: f32,
    /// Active pointer kind.
    pub pointer: PointerKind,
    /// Handles that should not be shown (e.g. cardinal handles for small
    /// elements, frame constraints, or multi-selection).
    pub omit: SmallVec<[HandleKind; 4]>,
}

impl TransformHandles {
    /// Create a `TransformHandles` with no omitted handles.
    pub fn new(bounds: Bounds<Pixels>, zoom: f32, pointer: PointerKind) -> Self {
        Self {
            bounds,
            zoom,
            pointer,
            omit: SmallVec::new(),
        }
    }

    /// Effective handle radius in canvas-space pixels (base size ÷ zoom).
    ///
    /// Dividing by zoom keeps the on-screen handle size constant as the user
    /// zooms in/out, matching Excalidraw's `size / zoom.value` formula.
    pub fn handle_size(&self) -> f32 {
        let z = if self.zoom > 0.0 { self.zoom } else { 1.0 };
        base_handle_size(self.pointer) / z
    }

    /// Return the canvas-space centre of `kind`, or `None` if that handle is
    /// in the omit list.
    pub fn handle_center(&self, kind: HandleKind) -> Option<Point<Pixels>> {
        if self.omit.contains(&kind) {
            return None;
        }

        let b = self.bounds;
        let left = b.origin.x.0;
        let top = b.origin.y.0;
        let right = left + b.size.width.0;
        let bottom = top + b.size.height.0;
        let cx = (left + right) * 0.5;
        let cy = (top + bottom) * 0.5;

        // Rotation handle sits above the top-centre by ROTATION_HANDLE_OFFSET_PX
        // divided by zoom so it stays a constant screen-space distance from the
        // element edge.
        let rot_offset = ROTATION_HANDLE_OFFSET_PX / self.zoom.max(f32::EPSILON);

        let (x, y) = match kind {
            HandleKind::N => (cx, top),
            HandleKind::S => (cx, bottom),
            HandleKind::E => (right, cy),
            HandleKind::W => (left, cy),
            HandleKind::Ne => (right, top),
            HandleKind::Nw => (left, top),
            HandleKind::Se => (right, bottom),
            HandleKind::Sw => (left, bottom),
            HandleKind::Rotation => (cx, top - rot_offset),
        };

        Some(Point::new(Pixels(x), Pixels(y)))
    }

    /// Return the handle hit by `point` in canvas space, or `None` on miss.
    ///
    /// Uses the half-handle-size as the hit tolerance radius so the hit area
    /// scales consistently with the visual size.
    pub fn hit(&self, point: Point<Pixels>) -> Option<HandleKind> {
        let tol = self.handle_size() * 0.5;
        let tol_sq = tol * tol;

        // Check in a fixed priority order: corners first, then cardinals, then
        // rotation.  This mirrors Excalidraw's precedence.
        let candidates = [
            HandleKind::Nw,
            HandleKind::Ne,
            HandleKind::Se,
            HandleKind::Sw,
            HandleKind::N,
            HandleKind::S,
            HandleKind::E,
            HandleKind::W,
            HandleKind::Rotation,
        ];

        for kind in candidates {
            if let Some(center) = self.handle_center(kind) {
                let dx = point.x.0 - center.x.0;
                let dy = point.y.0 - center.y.0;
                if dx * dx + dy * dy <= tol_sq {
                    return Some(kind);
                }
            }
        }

        None
    }

    /// Return all visible (non-omitted) handles in a stable order.
    pub fn all_visible(&self) -> SmallVec<[HandleKind; 9]> {
        let all = [
            HandleKind::Nw,
            HandleKind::Ne,
            HandleKind::Se,
            HandleKind::Sw,
            HandleKind::N,
            HandleKind::S,
            HandleKind::E,
            HandleKind::W,
            HandleKind::Rotation,
        ];
        all.iter()
            .copied()
            .filter(|k| !self.omit.contains(k))
            .collect()
    }
}

// ── tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn unit_bounds(x: f32, y: f32, w: f32, h: f32) -> Bounds<Pixels> {
        use nom_gpui::Size;
        Bounds::new(
            Point::new(Pixels(x), Pixels(y)),
            Size::new(Pixels(w), Pixels(h)),
        )
    }

    fn pt(x: f32, y: f32) -> Point<Pixels> {
        Point::new(Pixels(x), Pixels(y))
    }

    // --- 1. handle_size scales with zoom -----------------------------------------

    #[test]
    fn handle_size_scales_with_zoom() {
        let th1 = TransformHandles::new(unit_bounds(0.0, 0.0, 100.0, 100.0), 1.0, PointerKind::Mouse);
        let th2 = TransformHandles::new(unit_bounds(0.0, 0.0, 100.0, 100.0), 2.0, PointerKind::Mouse);
        // at zoom 2 the canvas-space handle size should be half
        assert!((th1.handle_size() - 8.0).abs() < f32::EPSILON);
        assert!((th2.handle_size() - 4.0).abs() < f32::EPSILON);
    }

    // --- 2. touch vs mouse size differs ------------------------------------------

    #[test]
    fn touch_handle_larger_than_mouse() {
        let mouse = TransformHandles::new(unit_bounds(0.0, 0.0, 100.0, 100.0), 1.0, PointerKind::Mouse);
        let touch = TransformHandles::new(unit_bounds(0.0, 0.0, 100.0, 100.0), 1.0, PointerKind::Touch);
        assert!(touch.handle_size() > mouse.handle_size());
    }

    // --- 3. 8 resize handle positions correct ------------------------------------

    #[test]
    fn eight_resize_handle_positions() {
        let b = unit_bounds(0.0, 0.0, 100.0, 80.0);
        let th = TransformHandles::new(b, 1.0, PointerKind::Mouse);

        // N → top-centre
        let n = th.handle_center(HandleKind::N).unwrap();
        assert_eq!(n.x.0, 50.0);
        assert_eq!(n.y.0, 0.0);

        // S → bottom-centre
        let s = th.handle_center(HandleKind::S).unwrap();
        assert_eq!(s.x.0, 50.0);
        assert_eq!(s.y.0, 80.0);

        // E → right-centre
        let e = th.handle_center(HandleKind::E).unwrap();
        assert_eq!(e.x.0, 100.0);
        assert_eq!(e.y.0, 40.0);

        // W → left-centre
        let w = th.handle_center(HandleKind::W).unwrap();
        assert_eq!(w.x.0, 0.0);
        assert_eq!(w.y.0, 40.0);

        // Ne → top-right corner
        let ne = th.handle_center(HandleKind::Ne).unwrap();
        assert_eq!(ne.x.0, 100.0);
        assert_eq!(ne.y.0, 0.0);

        // Nw → top-left corner
        let nw = th.handle_center(HandleKind::Nw).unwrap();
        assert_eq!(nw.x.0, 0.0);
        assert_eq!(nw.y.0, 0.0);

        // Se → bottom-right corner
        let se = th.handle_center(HandleKind::Se).unwrap();
        assert_eq!(se.x.0, 100.0);
        assert_eq!(se.y.0, 80.0);

        // Sw → bottom-left corner
        let sw = th.handle_center(HandleKind::Sw).unwrap();
        assert_eq!(sw.x.0, 0.0);
        assert_eq!(sw.y.0, 80.0);
    }

    // --- 4. rotation handle above top-centre -------------------------------------

    #[test]
    fn rotation_handle_above_bounds() {
        let b = unit_bounds(0.0, 0.0, 100.0, 100.0);
        let th = TransformHandles::new(b, 1.0, PointerKind::Mouse);
        let rot = th.handle_center(HandleKind::Rotation).unwrap();
        assert_eq!(rot.x.0, 50.0, "rotation x should be centre x");
        assert!(rot.y.0 < 0.0, "rotation handle should be above top edge");
    }

    // --- 5. omit list hides handles ----------------------------------------------

    #[test]
    fn omit_hides_handle() {
        let b = unit_bounds(0.0, 0.0, 100.0, 100.0);
        let mut th = TransformHandles::new(b, 1.0, PointerKind::Mouse);
        th.omit.push(HandleKind::N);
        th.omit.push(HandleKind::S);

        assert!(th.handle_center(HandleKind::N).is_none());
        assert!(th.handle_center(HandleKind::S).is_none());
        // Non-omitted handle is still visible.
        assert!(th.handle_center(HandleKind::E).is_some());
    }

    // --- 6. hit returns None when point is outside tolerance ---------------------

    #[test]
    fn hit_miss_outside_tolerance() {
        let b = unit_bounds(0.0, 0.0, 100.0, 100.0);
        let th = TransformHandles::new(b, 1.0, PointerKind::Mouse);
        // Far from all handles
        assert!(th.hit(pt(300.0, 300.0)).is_none());
    }

    // --- 7. hit returns N when pointer is on the N handle ------------------------

    #[test]
    fn hit_north_handle() {
        let b = unit_bounds(0.0, 0.0, 100.0, 100.0);
        let th = TransformHandles::new(b, 1.0, PointerKind::Mouse);
        // N centre is (50, 0); tolerance radius is 4 px at zoom=1
        let result = th.hit(pt(50.0, 0.0));
        assert_eq!(result, Some(HandleKind::N));
    }

    // --- 8. all_visible respects omit list ----------------------------------------

    #[test]
    fn all_visible_respects_omit() {
        let b = unit_bounds(0.0, 0.0, 100.0, 100.0);
        let mut th = TransformHandles::new(b, 1.0, PointerKind::Mouse);
        th.omit.push(HandleKind::Rotation);

        let visible = th.all_visible();
        assert!(!visible.contains(&HandleKind::Rotation));
        assert_eq!(visible.len(), 8);
    }
}
