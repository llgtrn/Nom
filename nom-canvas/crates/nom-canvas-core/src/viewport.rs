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
    pub fn to_scene_transform(&self) -> [[f32; 3]; 3] {
        let tx = self.pan[0] + self.size[0] / 2.0;
        let ty = self.pan[1] + self.size[1] / 2.0;
        [
            [self.zoom, 0.0,       tx],
            [0.0,       self.zoom, ty],
            [0.0,       0.0,       1.0],
        ]
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
}
