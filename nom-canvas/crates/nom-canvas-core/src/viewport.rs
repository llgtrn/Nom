use nom_gpui::types::{Bounds, Pixels, Point, Size};

/// Infinite-canvas viewport: maps between screen and canvas coordinate systems.
///
/// Coordinate convention (matches Excalidraw):
///   screen_to_canvas(pt) = (pt - size/2 - pan) / zoom
///   canvas_to_screen(pt) = pt * zoom + pan + size/2
#[derive(Debug, Clone)]
pub struct Viewport {
    /// Zoom level, clamped to [0.1, 32.0]
    pub zoom: f32,
    /// Canvas pan offset (screen pixels)
    pub pan: [f32; 2],
    /// Screen dimensions in pixels
    pub size: [f32; 2],
}

impl Viewport {
    pub fn new(width: f32, height: f32) -> Self {
        Self {
            zoom: 1.0,
            pan: [0.0, 0.0],
            size: [width, height],
        }
    }

    /// Convert a screen-space point to canvas-space.
    pub fn screen_to_canvas(&self, pt: [f32; 2]) -> [f32; 2] {
        [
            (pt[0] - self.size[0] / 2.0 - self.pan[0]) / self.zoom,
            (pt[1] - self.size[1] / 2.0 - self.pan[1]) / self.zoom,
        ]
    }

    /// Convert a canvas-space point to screen-space.
    pub fn canvas_to_screen(&self, pt: [f32; 2]) -> [f32; 2] {
        [
            pt[0] * self.zoom + self.pan[0] + self.size[0] / 2.0,
            pt[1] * self.zoom + self.pan[1] + self.size[1] / 2.0,
        ]
    }

    /// Returns the canvas-space bounding box visible on screen — used for culling.
    /// Returns `(top_left, bottom_right)` in canvas coordinates.
    pub fn visible_bounds(&self) -> ([f32; 2], [f32; 2]) {
        let top_left = self.screen_to_canvas([0.0, 0.0]);
        let bot_right = self.screen_to_canvas(self.size);
        (top_left, bot_right)
    }

    /// Returns the visible canvas region as a `nom_gpui::types::Bounds<Pixels>`.
    ///
    /// The origin is the top-left canvas coordinate mapped from screen `(0, 0)`,
    /// and the size is the canvas area covered by the current viewport at the
    /// current zoom level.  Use this when passing bounds to nom_gpui rendering.
    pub fn visible_bounds_gpui(&self) -> Bounds<Pixels> {
        let top_left = self.screen_to_canvas([0.0, 0.0]);
        let bot_right = self.screen_to_canvas(self.size);
        let w = bot_right[0] - top_left[0];
        let h = bot_right[1] - top_left[1];
        Bounds {
            origin: Point {
                x: Pixels(top_left[0]),
                y: Pixels(top_left[1]),
            },
            size: Size {
                width: Pixels(w),
                height: Pixels(h),
            },
        }
    }

    /// Zoom toward a screen-space cursor position so the canvas point under the
    /// cursor stays fixed on screen (standard pinch-to-zoom / scroll-wheel behaviour).
    pub fn zoom_toward(&mut self, new_zoom: f32, cursor: [f32; 2]) {
        let canvas_pt = self.screen_to_canvas(cursor);
        self.zoom = new_zoom.clamp(0.1, 32.0);
        // Re-derive pan so `canvas_pt` maps back to `cursor` at the new zoom.
        self.pan = [
            cursor[0] - self.size[0] / 2.0 - canvas_pt[0] * self.zoom,
            cursor[1] - self.size[1] / 2.0 - canvas_pt[1] * self.zoom,
        ];
    }

    /// Translate the viewport by a screen-space delta.
    pub fn pan_by(&mut self, delta: [f32; 2]) {
        self.pan[0] += delta[0];
        self.pan[1] += delta[1];
    }

    /// Reset to 1× zoom, no pan.
    pub fn reset(&mut self) {
        self.zoom = 1.0;
        self.pan = [0.0, 0.0];
    }

    /// Returns a 3×3 column-major affine matrix that maps canvas coords to screen:
    ///
    /// ```text
    /// [[zoom, 0,    pan_x],
    ///  [0,    zoom, pan_y],
    ///  [0,    0,    1    ]]
    /// ```
    ///
    /// The translation incorporates `size/2` so the canvas origin appears at
    /// screen centre (consistent with `canvas_to_screen`).
    ///
    /// Returns the affine 3x3 transform for use with nom_gpui::scene rendering.
    /// Maps canvas coordinates to screen coordinates via pan and zoom.
    /// Pair with `visible_bounds_gpui()` to pass the clip region to the renderer.
    pub fn to_scene_transform(&self) -> [[f32; 3]; 3] {
        let tx = self.pan[0] + self.size[0] / 2.0;
        let ty = self.pan[1] + self.size[1] / 2.0;
        [[self.zoom, 0.0, tx], [0.0, self.zoom, ty], [0.0, 0.0, 1.0]]
    }

    /// Applies `to_scene_transform` to a canvas-space point, returning a
    /// screen-space point.  Equivalent to `canvas_to_screen` but expressed
    /// through the matrix.
    pub fn apply_transform(&self, pt: [f32; 2]) -> [f32; 2] {
        let m = self.to_scene_transform();
        [
            m[0][0] * pt[0] + m[0][1] * pt[1] + m[0][2],
            m[1][0] * pt[0] + m[1][1] * pt[1] + m[1][2],
        ]
    }

    /// Returns `true` if the canvas-space point `pt` maps to a screen-space
    /// position that lies within the viewport bounds `[0, size[0]) × [0, size[1])`.
    pub fn is_point_visible(&self, pt: [f32; 2]) -> bool {
        let screen = self.apply_transform(pt);
        screen[0] >= 0.0
            && screen[0] <= self.size[0]
            && screen[1] >= 0.0
            && screen[1] <= self.size[1]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Screen centre should map to canvas origin when pan=[0,0] zoom=1.
    #[test]
    fn screen_center_maps_to_canvas_origin() {
        let vp = Viewport::new(800.0, 600.0);
        let canvas = vp.screen_to_canvas([400.0, 300.0]);
        assert!((canvas[0]).abs() < 1e-5, "x={}", canvas[0]);
        assert!((canvas[1]).abs() < 1e-5, "y={}", canvas[1]);
    }

    /// screen_to_canvas(canvas_to_screen(pt)) should be identity.
    #[test]
    fn round_trip_identity() {
        let vp = Viewport::new(1024.0, 768.0);
        let pt = [123.0_f32, -456.0_f32];
        let screen = vp.canvas_to_screen(pt);
        let back = vp.screen_to_canvas(screen);
        assert!((back[0] - pt[0]).abs() < 1e-4, "x round-trip error");
        assert!((back[1] - pt[1]).abs() < 1e-4, "y round-trip error");
    }

    /// canvas_to_screen(screen_to_canvas(pt)) should be identity.
    #[test]
    fn round_trip_reverse_identity() {
        let vp = Viewport::new(800.0, 600.0);
        let screen_pt = [200.0_f32, 150.0_f32];
        let canvas = vp.screen_to_canvas(screen_pt);
        let back = vp.canvas_to_screen(canvas);
        assert!((back[0] - screen_pt[0]).abs() < 1e-4);
        assert!((back[1] - screen_pt[1]).abs() < 1e-4);
    }

    /// zoom_toward clamps to 0.1.
    #[test]
    fn zoom_toward_clamps_min() {
        let mut vp = Viewport::new(800.0, 600.0);
        vp.zoom_toward(0.0, [400.0, 300.0]);
        assert!((vp.zoom - 0.1).abs() < 1e-6);
    }

    /// zoom_toward clamps to 32.0.
    #[test]
    fn zoom_toward_clamps_max() {
        let mut vp = Viewport::new(800.0, 600.0);
        vp.zoom_toward(999.0, [400.0, 300.0]);
        assert!((vp.zoom - 32.0).abs() < 1e-6);
    }

    /// After zoom_toward, the canvas point originally under the cursor stays put.
    #[test]
    fn zoom_toward_preserves_cursor_canvas_point() {
        let mut vp = Viewport::new(800.0, 600.0);
        let cursor = [300.0_f32, 200.0_f32];
        let canvas_before = vp.screen_to_canvas(cursor);
        vp.zoom_toward(2.0, cursor);
        let canvas_after = vp.screen_to_canvas(cursor);
        assert!((canvas_after[0] - canvas_before[0]).abs() < 1e-4);
        assert!((canvas_after[1] - canvas_before[1]).abs() < 1e-4);
    }

    #[test]
    fn pan_by_shifts_pan() {
        let mut vp = Viewport::new(800.0, 600.0);
        vp.pan_by([10.0, -5.0]);
        assert!((vp.pan[0] - 10.0).abs() < 1e-6);
        assert!((vp.pan[1] - (-5.0)).abs() < 1e-6);
    }

    #[test]
    fn reset_restores_defaults() {
        let mut vp = Viewport::new(800.0, 600.0);
        vp.zoom_toward(4.0, [100.0, 100.0]);
        vp.pan_by([50.0, 50.0]);
        vp.reset();
        assert!((vp.zoom - 1.0).abs() < 1e-6);
        assert!((vp.pan[0]).abs() < 1e-6);
        assert!((vp.pan[1]).abs() < 1e-6);
    }

    #[test]
    fn viewport_to_scene_transform_identity() {
        // At zoom=1, pan=[0,0], size=[800,600]: matrix maps canvas origin to
        // screen centre (400, 300).
        let vp = Viewport::new(800.0, 600.0);
        let m = vp.to_scene_transform();
        // Top-left 2×2 is identity * zoom
        assert!((m[0][0] - 1.0).abs() < 1e-6);
        assert!((m[1][1] - 1.0).abs() < 1e-6);
        assert!((m[0][1]).abs() < 1e-6);
        assert!((m[1][0]).abs() < 1e-6);
        // Translation is size/2
        assert!((m[0][2] - 400.0).abs() < 1e-4);
        assert!((m[1][2] - 300.0).abs() < 1e-4);
        assert!((m[2][2] - 1.0).abs() < 1e-6);
    }

    #[test]
    fn viewport_apply_transform_with_pan() {
        let mut vp = Viewport::new(800.0, 600.0);
        vp.pan_by([50.0, -20.0]);
        // Canvas origin should map to screen centre + pan
        let screen = vp.apply_transform([0.0, 0.0]);
        assert!((screen[0] - 450.0).abs() < 1e-4, "x={}", screen[0]);
        assert!((screen[1] - 280.0).abs() < 1e-4, "y={}", screen[1]);
        // apply_transform must match canvas_to_screen
        let pt = [10.0_f32, -5.0];
        let via_matrix = vp.apply_transform(pt);
        let direct = vp.canvas_to_screen(pt);
        assert!((via_matrix[0] - direct[0]).abs() < 1e-4);
        assert!((via_matrix[1] - direct[1]).abs() < 1e-4);
    }

    #[test]
    fn viewport_is_point_visible_in_bounds() {
        let vp = Viewport::new(800.0, 600.0);
        // Canvas origin maps to screen centre — always visible.
        assert!(vp.is_point_visible([0.0, 0.0]));
    }

    #[test]
    fn viewport_is_point_visible_out_of_bounds() {
        let vp = Viewport::new(800.0, 600.0);
        // Canvas point far off to the right.
        // At zoom=1, x_screen = canvas_x + 400; canvas_x=1000 → screen_x=1400 > 800.
        assert!(!vp.is_point_visible([1000.0, 0.0]));
    }

    #[test]
    fn visible_bounds_covers_screen() {
        let vp = Viewport::new(800.0, 600.0);
        let (tl, br) = vp.visible_bounds();
        // At zoom=1, pan=0 the canvas bounds should be [-400,-300] to [400,300]
        assert!((tl[0] - (-400.0)).abs() < 1e-4);
        assert!((tl[1] - (-300.0)).abs() < 1e-4);
        assert!((br[0] - 400.0).abs() < 1e-4);
        assert!((br[1] - 300.0).abs() < 1e-4);
    }

    // ── nom_gpui integration ─────────────────────────────────────────────────

    #[test]
    fn visible_bounds_gpui_matches_raw_at_default() {
        use nom_gpui::types::{Pixels, Point};
        let vp = Viewport::new(800.0, 600.0);
        let b = vp.visible_bounds_gpui();
        // At zoom=1, pan=0: origin at (-400, -300), size 800×600.
        assert!(
            (b.origin.x.0 - (-400.0)).abs() < 1e-4,
            "origin.x={}",
            b.origin.x.0
        );
        assert!(
            (b.origin.y.0 - (-300.0)).abs() < 1e-4,
            "origin.y={}",
            b.origin.y.0
        );
        assert!(
            (b.size.width.0 - 800.0).abs() < 1e-4,
            "width={}",
            b.size.width.0
        );
        assert!(
            (b.size.height.0 - 600.0).abs() < 1e-4,
            "height={}",
            b.size.height.0
        );
        // The nom_gpui Bounds::contains check works for the canvas origin.
        let canvas_origin = Point {
            x: Pixels(0.0),
            y: Pixels(0.0),
        };
        assert!(
            b.contains(&canvas_origin),
            "canvas origin must be inside visible bounds"
        );
    }

    #[test]
    fn visible_bounds_gpui_shrinks_with_zoom() {
        let mut vp = Viewport::new(800.0, 600.0);
        vp.zoom_toward(2.0, [400.0, 300.0]);
        let b = vp.visible_bounds_gpui();
        // At 2× zoom the visible canvas area halves in each dimension.
        assert!(
            (b.size.width.0 - 400.0).abs() < 1e-3,
            "width at 2x zoom = {}",
            b.size.width.0
        );
        assert!(
            (b.size.height.0 - 300.0).abs() < 1e-3,
            "height at 2x zoom = {}",
            b.size.height.0
        );
    }

    #[test]
    fn visible_bounds_gpui_contains_checks_nom_gpui_point() {
        use nom_gpui::types::{Bounds, Pixels, Point};
        let vp = Viewport::new(800.0, 600.0);
        let bounds: Bounds<Pixels> = vp.visible_bounds_gpui();
        // The canvas-space point (200, 100) should lie inside the default viewport.
        let inside = Point {
            x: Pixels(200.0),
            y: Pixels(100.0),
        };
        assert!(bounds.contains(&inside));
        // A far-off point should be outside.
        let outside = Point {
            x: Pixels(1000.0),
            y: Pixels(1000.0),
        };
        assert!(!bounds.contains(&outside));
    }

    /// Zoom in 3× then zoom out 3× returns to original zoom level.
    #[test]
    fn viewport_zoom_sequence() {
        let mut vp = Viewport::new(800.0, 600.0);
        let cursor = [400.0_f32, 300.0_f32];
        let original_zoom = vp.zoom;
        vp.zoom_toward(3.0, cursor);
        assert!((vp.zoom - 3.0).abs() < 1e-5);
        vp.zoom_toward(original_zoom, cursor);
        assert!((vp.zoom - original_zoom).abs() < 1e-5);
    }

    /// Pan by (10, 20) then verify world_to_screen offset matches.
    #[test]
    fn viewport_pan_delta() {
        let mut vp = Viewport::new(800.0, 600.0);
        vp.pan_by([10.0, 20.0]);
        // Canvas origin should now map to screen centre + pan.
        let screen = vp.canvas_to_screen([0.0, 0.0]);
        assert!((screen[0] - 410.0).abs() < 1e-4, "x={}", screen[0]);
        assert!((screen[1] - 320.0).abs() < 1e-4, "y={}", screen[1]);
    }

    /// scale_factor equivalent: zoom=2.0 doubles screen distance from centre.
    #[test]
    fn viewport_scale_factor_affects_pixels() {
        let mut vp = Viewport::new(800.0, 600.0);
        vp.zoom_toward(2.0, [400.0, 300.0]);
        // A canvas point at (50, 0) should appear 100 px to the right of screen centre.
        let screen = vp.canvas_to_screen([50.0, 0.0]);
        // centre_x = 400, so screen.x = 50*2 + 400 = 500
        assert!((screen[0] - 500.0).abs() < 1e-3, "x={}", screen[0]);
    }

    /// reset() returns to identity transform (zoom=1, pan=0).
    #[test]
    fn viewport_reset() {
        let mut vp = Viewport::new(1024.0, 768.0);
        vp.zoom_toward(5.0, [512.0, 384.0]);
        vp.pan_by([100.0, -50.0]);
        vp.reset();
        assert!((vp.zoom - 1.0).abs() < 1e-6, "zoom after reset={}", vp.zoom);
        assert!((vp.pan[0]).abs() < 1e-6, "pan.x after reset={}", vp.pan[0]);
        assert!((vp.pan[1]).abs() < 1e-6, "pan.y after reset={}", vp.pan[1]);
    }

    /// After fitting to a rect, the rect's corners are visible.
    #[test]
    fn viewport_fit_rect() {
        // A canvas rect [100, 100] → [300, 200]; fit the viewport to show it.
        // We implement fit manually: pick zoom to fit width and height, then
        // adjust pan so the rect centre maps to screen centre.
        let mut vp = Viewport::new(800.0, 600.0);
        let (rx, ry, rw, rh) = (100.0_f32, 100.0_f32, 200.0_f32, 100.0_f32);
        let zoom_x = vp.size[0] / rw;
        let zoom_y = vp.size[1] / rh;
        let new_zoom = zoom_x.min(zoom_y).clamp(0.1, 32.0);
        let cx = rx + rw / 2.0;
        let cy = ry + rh / 2.0;
        vp.zoom = new_zoom;
        vp.pan = [-cx * new_zoom, -cy * new_zoom];
        // The rect's top-left and bottom-right should both be visible on screen.
        assert!(
            vp.is_point_visible([rx, ry]),
            "top-left of rect must be visible"
        );
        assert!(
            vp.is_point_visible([rx + rw, ry + rh]),
            "bottom-right of rect must be visible"
        );
    }

    /// world_bounds equivalent: visible_bounds covers expected canvas area.
    #[test]
    fn viewport_world_bounds() {
        let mut vp = Viewport::new(800.0, 600.0);
        vp.zoom_toward(2.0, [400.0, 300.0]);
        let (tl, br) = vp.visible_bounds();
        let w = br[0] - tl[0];
        let h = br[1] - tl[1];
        // At 2× zoom the visible canvas area is half the screen size.
        assert!((w - 400.0).abs() < 1e-3, "visible width at 2x zoom = {}", w);
        assert!(
            (h - 300.0).abs() < 1e-3,
            "visible height at 2x zoom = {}",
            h
        );
    }

    /// zoom centered at (100, 100) leaves that screen point fixed on canvas.
    #[test]
    fn viewport_zoom_at_point() {
        let mut vp = Viewport::new(800.0, 600.0);
        let cursor = [100.0_f32, 100.0_f32];
        let canvas_before = vp.screen_to_canvas(cursor);
        vp.zoom_toward(3.0, cursor);
        let canvas_after = vp.screen_to_canvas(cursor);
        assert!(
            (canvas_after[0] - canvas_before[0]).abs() < 1e-3,
            "canvas x under cursor changed: before={} after={}",
            canvas_before[0],
            canvas_after[0]
        );
        assert!(
            (canvas_after[1] - canvas_before[1]).abs() < 1e-3,
            "canvas y under cursor changed: before={} after={}",
            canvas_before[1],
            canvas_after[1]
        );
    }

    /// screen_to_canvas at zoom=2.0 halves the displacement from screen centre.
    #[test]
    fn screen_to_canvas_at_nonunit_zoom() {
        let mut vp = Viewport::new(800.0, 600.0);
        vp.zoom_toward(2.0, [400.0, 300.0]);
        // Screen point 100px right of centre = (500, 300).
        // canvas_x = (500 - 400 - pan_x) / 2.0 = 50.0
        let canvas = vp.screen_to_canvas([500.0, 300.0]);
        assert!((canvas[0] - 50.0).abs() < 1e-3, "canvas.x={}", canvas[0]);
        assert!((canvas[1]).abs() < 1e-3, "canvas.y={}", canvas[1]);
    }

    /// canvas_to_screen at zoom=2.0 doubles displacement from canvas origin.
    #[test]
    fn canvas_to_screen_at_nonunit_zoom() {
        let mut vp = Viewport::new(800.0, 600.0);
        vp.zoom_toward(2.0, [400.0, 300.0]);
        // Canvas point (100, 0) → screen_x = 100*2 + pan_x + 400.
        // At zoom=2 centered at screen centre, pan=0 → screen_x = 200 + 400 = 600.
        let screen = vp.canvas_to_screen([100.0, 0.0]);
        assert!((screen[0] - 600.0).abs() < 1e-3, "screen.x={}", screen[0]);
        assert!((screen[1] - 300.0).abs() < 1e-3, "screen.y={}", screen[1]);
    }

    /// Aspect ratio: at zoom=1, visible_bounds width/height = screen width/height.
    #[test]
    fn aspect_ratio_preserved_at_zoom1() {
        let vp = Viewport::new(1200.0, 400.0);
        let (tl, br) = vp.visible_bounds();
        let w = br[0] - tl[0];
        let h = br[1] - tl[1];
        let aspect = w / h;
        let expected = 1200.0_f32 / 400.0_f32;
        assert!(
            (aspect - expected).abs() < 1e-3,
            "aspect ratio mismatch: got {} expected {}",
            aspect,
            expected
        );
    }

    /// Aspect ratio is preserved at zoom=3.
    #[test]
    fn aspect_ratio_preserved_at_zoom3() {
        let mut vp = Viewport::new(900.0, 300.0);
        vp.zoom_toward(3.0, [450.0, 150.0]);
        let (tl, br) = vp.visible_bounds();
        let w = br[0] - tl[0];
        let h = br[1] - tl[1];
        let aspect = w / h;
        let expected = 900.0_f32 / 300.0_f32; // 3:1 regardless of zoom
        assert!(
            (aspect - expected).abs() < 1e-3,
            "aspect ratio at zoom=3: {} vs expected {}",
            aspect,
            expected
        );
    }

    /// fit_rect: after fitting a small rect with margin, both corners are visible.
    #[test]
    fn fit_rect_with_margin() {
        let mut vp = Viewport::new(800.0, 600.0);
        let margin = 20.0_f32;
        let (rx, ry, rw, rh) = (50.0_f32, 50.0_f32, 100.0_f32, 80.0_f32);
        // Compute zoom so rect + margin fills the viewport.
        let zoom_x = (vp.size[0] - 2.0 * margin) / rw;
        let zoom_y = (vp.size[1] - 2.0 * margin) / rh;
        let new_zoom = zoom_x.min(zoom_y).clamp(0.1, 32.0);
        let cx = rx + rw / 2.0;
        let cy = ry + rh / 2.0;
        vp.zoom = new_zoom;
        vp.pan = [-cx * new_zoom, -cy * new_zoom];
        assert!(
            vp.is_point_visible([rx, ry]),
            "top-left of rect with margin must be visible"
        );
        assert!(
            vp.is_point_visible([rx + rw, ry + rh]),
            "bottom-right of rect with margin must be visible"
        );
    }

    /// Pan moves only along one axis when delta has one zero component.
    #[test]
    fn pan_by_single_axis() {
        let mut vp = Viewport::new(800.0, 600.0);
        vp.pan_by([0.0, 30.0]);
        assert!((vp.pan[0]).abs() < 1e-6, "pan.x should remain 0");
        assert!((vp.pan[1] - 30.0).abs() < 1e-6, "pan.y={}", vp.pan[1]);
    }

    /// Multiple successive pans accumulate correctly.
    #[test]
    fn pan_accumulates() {
        let mut vp = Viewport::new(800.0, 600.0);
        vp.pan_by([10.0, 5.0]);
        vp.pan_by([20.0, -15.0]);
        assert!((vp.pan[0] - 30.0).abs() < 1e-6, "pan.x={}", vp.pan[0]);
        assert!((vp.pan[1] - (-10.0)).abs() < 1e-6, "pan.y={}", vp.pan[1]);
    }

    /// zoom_toward with new_zoom == current zoom changes nothing (idempotent).
    #[test]
    fn zoom_toward_idempotent_at_same_zoom() {
        let mut vp = Viewport::new(800.0, 600.0);
        let cursor = [300.0, 250.0];
        let canvas_before = vp.screen_to_canvas(cursor);
        vp.zoom_toward(1.0, cursor);
        assert!((vp.zoom - 1.0).abs() < 1e-6);
        let canvas_after = vp.screen_to_canvas(cursor);
        assert!((canvas_after[0] - canvas_before[0]).abs() < 1e-4);
        assert!((canvas_after[1] - canvas_before[1]).abs() < 1e-4);
    }

    /// Viewport at non-default size: screen centre maps to canvas origin.
    #[test]
    fn screen_centre_maps_to_canvas_origin_large_viewport() {
        let vp = Viewport::new(1920.0, 1080.0);
        let canvas = vp.screen_to_canvas([960.0, 540.0]);
        assert!((canvas[0]).abs() < 1e-4, "x={}", canvas[0]);
        assert!((canvas[1]).abs() < 1e-4, "y={}", canvas[1]);
    }

    /// A canvas point far off-screen is not visible.
    #[test]
    fn far_canvas_point_not_visible() {
        let vp = Viewport::new(800.0, 600.0);
        assert!(!vp.is_point_visible([5000.0, 5000.0]));
        assert!(!vp.is_point_visible([-5000.0, -5000.0]));
    }

    /// zoom_toward at minimum clamp (0.1) still preserves cursor point.
    #[test]
    fn zoom_toward_min_clamp_preserves_cursor() {
        let mut vp = Viewport::new(800.0, 600.0);
        let cursor = [200.0_f32, 150.0];
        let canvas_before = vp.screen_to_canvas(cursor);
        vp.zoom_toward(0.0001, cursor);
        assert!((vp.zoom - 0.1).abs() < 1e-6, "zoom must clamp to 0.1");
        let canvas_after = vp.screen_to_canvas(cursor);
        assert!(
            (canvas_after[0] - canvas_before[0]).abs() < 1e-3,
            "canvas x under cursor changed at min zoom"
        );
        assert!(
            (canvas_after[1] - canvas_before[1]).abs() < 1e-3,
            "canvas y under cursor changed at min zoom"
        );
    }

    /// zoom_toward at maximum clamp (32.0) still preserves cursor point.
    #[test]
    fn zoom_toward_max_clamp_preserves_cursor() {
        let mut vp = Viewport::new(800.0, 600.0);
        let cursor = [600.0_f32, 400.0];
        let canvas_before = vp.screen_to_canvas(cursor);
        vp.zoom_toward(1000.0, cursor);
        assert!((vp.zoom - 32.0).abs() < 1e-6, "zoom must clamp to 32.0");
        let canvas_after = vp.screen_to_canvas(cursor);
        assert!(
            (canvas_after[0] - canvas_before[0]).abs() < 1e-3,
            "canvas x under cursor changed at max zoom"
        );
        assert!(
            (canvas_after[1] - canvas_before[1]).abs() < 1e-3,
            "canvas y under cursor changed at max zoom"
        );
    }

    /// visible_bounds at minimum zoom covers a very large canvas area.
    #[test]
    fn visible_bounds_large_at_min_zoom() {
        let mut vp = Viewport::new(800.0, 600.0);
        vp.zoom_toward(0.1, [400.0, 300.0]);
        let (tl, br) = vp.visible_bounds();
        let w = br[0] - tl[0];
        let h = br[1] - tl[1];
        // At 0.1x zoom the visible canvas width = 800 / 0.1 = 8000
        assert!(w > 7000.0, "visible width at min zoom should be huge, got {}", w);
        assert!(h > 5000.0, "visible height at min zoom should be huge, got {}", h);
    }

    /// visible_bounds at maximum zoom covers a very small canvas area.
    #[test]
    fn visible_bounds_small_at_max_zoom() {
        let mut vp = Viewport::new(800.0, 600.0);
        vp.zoom_toward(32.0, [400.0, 300.0]);
        let (tl, br) = vp.visible_bounds();
        let w = br[0] - tl[0];
        let h = br[1] - tl[1];
        // At 32x zoom the visible canvas width = 800 / 32 = 25
        assert!(w < 30.0, "visible width at max zoom should be tiny, got {}", w);
        assert!(h < 25.0, "visible height at max zoom should be tiny, got {}", h);
    }

    /// pan_by with negative delta moves viewport in negative direction.
    #[test]
    fn pan_by_negative_delta() {
        let mut vp = Viewport::new(800.0, 600.0);
        vp.pan_by([-30.0, -20.0]);
        assert!((vp.pan[0] - (-30.0)).abs() < 1e-6);
        assert!((vp.pan[1] - (-20.0)).abs() < 1e-6);
        // Canvas origin maps to screen centre + pan
        let screen = vp.canvas_to_screen([0.0, 0.0]);
        assert!((screen[0] - 370.0).abs() < 1e-4, "screen.x={}", screen[0]);
        assert!((screen[1] - 280.0).abs() < 1e-4, "screen.y={}", screen[1]);
    }

    /// apply_transform at zoom=4 matches canvas_to_screen.
    #[test]
    fn apply_transform_matches_canvas_to_screen_at_zoom4() {
        let mut vp = Viewport::new(1024.0, 768.0);
        vp.zoom_toward(4.0, [512.0, 384.0]);
        let pt = [25.0_f32, -10.0];
        let via_matrix = vp.apply_transform(pt);
        let direct = vp.canvas_to_screen(pt);
        assert!((via_matrix[0] - direct[0]).abs() < 1e-3);
        assert!((via_matrix[1] - direct[1]).abs() < 1e-3);
    }

    /// screen_to_canvas and canvas_to_screen are inverses at arbitrary pan+zoom.
    #[test]
    fn round_trip_at_arbitrary_pan_zoom() {
        let mut vp = Viewport::new(1280.0, 720.0);
        vp.zoom_toward(1.5, [640.0, 360.0]);
        vp.pan_by([77.0, -33.0]);
        let canvas_pt = [123.4_f32, -56.7];
        let screen = vp.canvas_to_screen(canvas_pt);
        let back = vp.screen_to_canvas(screen);
        assert!((back[0] - canvas_pt[0]).abs() < 1e-3);
        assert!((back[1] - canvas_pt[1]).abs() < 1e-3);
    }

    /// Viewport size 1×1 (degenerate): screen centre still maps to canvas origin.
    #[test]
    fn degenerate_1x1_viewport() {
        let vp = Viewport::new(1.0, 1.0);
        let canvas = vp.screen_to_canvas([0.5, 0.5]);
        assert!(canvas[0].abs() < 1e-5);
        assert!(canvas[1].abs() < 1e-5);
    }

    /// Successive zoom_toward calls compose correctly.
    #[test]
    fn zoom_toward_composition() {
        let mut vp = Viewport::new(800.0, 600.0);
        let cursor = [400.0_f32, 300.0];
        vp.zoom_toward(2.0, cursor);
        vp.zoom_toward(4.0, cursor);
        assert!((vp.zoom - 4.0).abs() < 1e-5, "final zoom should be 4.0");
        // Canvas point under cursor should still be the original canvas point.
        // At default zoom=1, screen centre maps to canvas (0,0).
        let canvas_under = vp.screen_to_canvas(cursor);
        assert!(
            canvas_under[0].abs() < 1e-3,
            "canvas x under cursor after two zooms: {}",
            canvas_under[0]
        );
        assert!(
            canvas_under[1].abs() < 1e-3,
            "canvas y under cursor after two zooms: {}",
            canvas_under[1]
        );
    }

    /// Viewport serialization round-trip: zoom+pan values survive a clone.
    #[test]
    fn viewport_serialization_round_trip() {
        let mut vp = Viewport::new(1024.0, 768.0);
        vp.zoom_toward(2.5, [512.0, 384.0]);
        vp.pan_by([33.0, -17.0]);
        let vp2 = vp.clone();
        assert!((vp2.zoom - vp.zoom).abs() < 1e-6, "zoom must survive clone");
        assert!((vp2.pan[0] - vp.pan[0]).abs() < 1e-6, "pan.x must survive clone");
        assert!((vp2.pan[1] - vp.pan[1]).abs() < 1e-6, "pan.y must survive clone");
        assert!((vp2.size[0] - vp.size[0]).abs() < 1e-6, "size.w must survive clone");
        assert!((vp2.size[1] - vp.size[1]).abs() < 1e-6, "size.h must survive clone");
    }

    /// After zoom_toward then inverse zoom, the viewport returns to original zoom.
    #[test]
    fn viewport_transform_then_inverse_returns_original() {
        let mut vp = Viewport::new(800.0, 600.0);
        let cursor = [400.0_f32, 300.0];
        let original_zoom = vp.zoom;
        let original_pan = vp.pan;
        vp.zoom_toward(3.0, cursor);
        vp.zoom_toward(original_zoom, cursor);
        assert!((vp.zoom - original_zoom).abs() < 1e-5, "zoom must return to original");
        // Pan should also return to original when re-zooming to original level at screen centre.
        assert!((vp.pan[0] - original_pan[0]).abs() < 1e-3, "pan.x must return");
        assert!((vp.pan[1] - original_pan[1]).abs() < 1e-3, "pan.y must return");
    }

    /// Scroll wheel accumulation: multiple small zoom steps accumulate correctly.
    #[test]
    fn scroll_wheel_accumulation() {
        let mut vp = Viewport::new(800.0, 600.0);
        let cursor = [400.0_f32, 300.0];
        // Simulate 5 scroll-wheel steps of 0.1 zoom each, starting from 1.0.
        let step_zooms = [1.1_f32, 1.2, 1.3, 1.4, 1.5];
        for &z in &step_zooms {
            vp.zoom_toward(z, cursor);
        }
        assert!((vp.zoom - 1.5).abs() < 1e-5, "accumulated zoom should be 1.5, got {}", vp.zoom);
    }

    /// Fit-to-rect with aspect preservation: the smaller dimension fits and the
    /// rect's extremes are visible.
    #[test]
    fn fit_to_rect_aspect_preservation() {
        let mut vp = Viewport::new(800.0, 400.0); // 2:1 viewport
        // A square canvas rect.
        let (rx, ry, rw, rh) = (0.0_f32, 0.0, 100.0, 100.0);
        let zoom_x = vp.size[0] / rw; // 8.0
        let zoom_y = vp.size[1] / rh; // 4.0
        let new_zoom = zoom_x.min(zoom_y).clamp(0.1, 32.0); // 4.0 — height limited
        let cx = rx + rw / 2.0;
        let cy = ry + rh / 2.0;
        vp.zoom = new_zoom;
        vp.pan = [-cx * new_zoom, -cy * new_zoom];
        // All four corners must be visible.
        assert!(vp.is_point_visible([rx, ry]), "top-left must be visible");
        assert!(vp.is_point_visible([rx + rw, ry]), "top-right must be visible");
        assert!(vp.is_point_visible([rx, ry + rh]), "bottom-left must be visible");
        assert!(vp.is_point_visible([rx + rw, ry + rh]), "bottom-right must be visible");
        // The zoom chosen is the height-limited one (4.0), not width-limited (8.0).
        assert!((vp.zoom - zoom_y).abs() < 1e-5, "zoom must be height-limited");
    }

    /// Viewport size changes: visible_bounds correctly reflects new canvas area.
    #[test]
    fn viewport_size_change_updates_visible_bounds() {
        let vp_small = Viewport::new(400.0, 300.0);
        let vp_large = Viewport::new(800.0, 600.0);
        let (_, br_small) = vp_small.visible_bounds();
        let (_, br_large) = vp_large.visible_bounds();
        // Larger screen → larger visible canvas area.
        assert!(br_large[0] > br_small[0], "larger viewport shows more canvas width");
        assert!(br_large[1] > br_small[1], "larger viewport shows more canvas height");
    }

    /// Pan then reset: visible_bounds returns to the default after reset.
    #[test]
    fn viewport_pan_then_reset_visible_bounds() {
        let mut vp = Viewport::new(800.0, 600.0);
        vp.pan_by([200.0, 100.0]);
        vp.reset();
        let (tl, br) = vp.visible_bounds();
        // At zoom=1, pan=0: canvas bounds are [-400,-300] to [400,300].
        assert!((tl[0] - (-400.0)).abs() < 1e-4, "tl.x after reset={}", tl[0]);
        assert!((br[0] - 400.0).abs() < 1e-4, "br.x after reset={}", br[0]);
    }

    /// apply_transform matches canvas_to_screen at various zoom levels.
    #[test]
    fn apply_transform_matches_canvas_to_screen_various_zooms() {
        for zoom in &[0.5_f32, 1.0, 2.0, 8.0] {
            let mut vp = Viewport::new(800.0, 600.0);
            vp.zoom_toward(*zoom, [400.0, 300.0]);
            let pt = [42.0_f32, -17.0];
            let via_matrix = vp.apply_transform(pt);
            let direct = vp.canvas_to_screen(pt);
            assert!(
                (via_matrix[0] - direct[0]).abs() < 1e-3,
                "x mismatch at zoom={}: matrix={} direct={}",
                zoom, via_matrix[0], direct[0]
            );
            assert!(
                (via_matrix[1] - direct[1]).abs() < 1e-3,
                "y mismatch at zoom={}: matrix={} direct={}",
                zoom, via_matrix[1], direct[1]
            );
        }
    }

    /// visible_bounds_gpui at high zoom covers tiny canvas area.
    #[test]
    fn visible_bounds_gpui_at_high_zoom_is_tiny() {
        let mut vp = Viewport::new(800.0, 600.0);
        vp.zoom_toward(16.0, [400.0, 300.0]);
        let b = vp.visible_bounds_gpui();
        // At 16x zoom, visible width = 800/16 = 50 px canvas.
        assert!(b.size.width.0 < 60.0, "visible width at 16x zoom should be < 60, got {}", b.size.width.0);
    }

    /// screen_to_canvas with pan: offset shifts the canvas mapping.
    #[test]
    fn screen_to_canvas_with_pan_offset() {
        let mut vp = Viewport::new(800.0, 600.0);
        vp.pan_by([100.0, 0.0]);
        // Screen centre is (400, 300). With pan_x=100, canvas_x = (400 - 400 - 100)/1 = -100.
        let canvas = vp.screen_to_canvas([400.0, 300.0]);
        assert!((canvas[0] - (-100.0)).abs() < 1e-4, "canvas.x={}", canvas[0]);
        assert!((canvas[1]).abs() < 1e-4, "canvas.y={}", canvas[1]);
    }

    /// canvas_to_screen with pan: offset shifts screen mapping accordingly.
    #[test]
    fn canvas_to_screen_with_pan_offset() {
        let mut vp = Viewport::new(800.0, 600.0);
        vp.pan_by([-50.0, 0.0]);
        // Canvas origin: screen_x = 0*1 + (-50) + 400 = 350.
        let screen = vp.canvas_to_screen([0.0, 0.0]);
        assert!((screen[0] - 350.0).abs() < 1e-4, "screen.x={}", screen[0]);
    }
}
