// Integration tests: nom-canvas-core ↔ nom-gpui ↔ nom-theme pipeline

use nom_canvas_core::viewport::Viewport;
use nom_canvas_core::selection::RubberBand;
use nom_canvas_core::elements::ElementBounds;
use nom_gpui::scene::{Scene, Quad, Shadow, FrostedRect};
use nom_gpui::types::{Bounds, Hsla, Pixels, Point, Size};
use nom_gpui::renderer::{LinearRgba, Renderer};
use nom_theme::tokens;

// ── 1. viewport → scene quad ─────────────────────────────────────────────────

/// Convert a canvas-space point through the viewport, build a Quad at the
/// resulting screen position, and verify it lands in the Scene.
#[test]
fn integration_viewport_to_scene_quad() {
    let vp = Viewport::new(800.0, 600.0);
    let screen = vp.canvas_to_screen([0.0, 0.0]);
    let mut scene = Scene::new();
    scene.push_quad(Quad {
        bounds: Bounds {
            origin: Point { x: Pixels(screen[0]), y: Pixels(screen[1]) },
            size: Size { width: Pixels(50.0), height: Pixels(50.0) },
        },
        ..Default::default()
    });
    assert_eq!(scene.quads.len(), 1);
}

// ── 2. theme colors in a Quad background ─────────────────────────────────────

/// Use the BG token ([f32;4]) to construct a Quad background colour value.
#[test]
fn integration_theme_colors_in_quad() {
    let bg: [f32; 4] = tokens::BG;
    // BG is a valid [f32; 4] — just verify the type and range.
    assert_eq!(bg.len(), 4);
    for ch in bg {
        assert!(ch >= 0.0 && ch <= 1.0, "channel out of range: {ch}");
    }
    // Build a Quad that uses an Hsla derived from the token.
    let color = Hsla::new(0.0, 0.0, bg[2], bg[3]);
    let mut scene = Scene::new();
    scene.push_quad(Quad {
        background: Some(color),
        ..Default::default()
    });
    assert_eq!(scene.quads.len(), 1);
}

// ── 3. scene cleared after draw ───────────────────────────────────────────────

/// Push a Quad, draw it through the Renderer, then clear; scene must be empty.
#[test]
fn integration_scene_cleared_after_draw() {
    let mut scene = Scene::new();
    scene.push_quad(Quad::default());
    let mut renderer = Renderer::new();
    renderer.draw(&mut scene);
    scene.clear();
    assert!(scene.is_empty());
}

// ── 4. zoom × 2 halves visible canvas area ───────────────────────────────────

/// At 2× zoom the visible canvas region halves in width and height.
#[test]
fn integration_viewport_zoom_affects_bounds() {
    let mut vp = Viewport::new(800.0, 600.0);
    // Zoom toward screen centre so the test is zoom-only (no pan drift).
    vp.zoom_toward(2.0, [400.0, 300.0]);
    let b = vp.visible_bounds_gpui();
    assert!(
        (b.size.width.0 - 400.0).abs() < 1e-3,
        "width at 2× zoom should be 400, got {}",
        b.size.width.0
    );
    assert!(
        (b.size.height.0 - 300.0).abs() < 1e-3,
        "height at 2× zoom should be 300, got {}",
        b.size.height.0
    );
}

// ── 5. FrostedRect built from nom-theme tokens ───────────────────────────────

/// Construct a FrostedRect using the frosted-glass tokens and push it to a Scene.
#[test]
fn integration_frosted_rect_uses_tokens() {
    let rect = FrostedRect {
        bounds: Bounds {
            origin: Point { x: Pixels(0.0), y: Pixels(0.0) },
            size: Size { width: Pixels(200.0), height: Pixels(100.0) },
        },
        blur_radius: tokens::FROSTED_BLUR_RADIUS,
        bg_alpha: tokens::FROSTED_BG_ALPHA,
        border_alpha: tokens::FROSTED_BORDER_ALPHA,
    };
    assert!((rect.blur_radius - 12.0).abs() < 1e-6);
    assert!((rect.bg_alpha - 0.85).abs() < 1e-6);
    let mut scene = Scene::new();
    scene.push_frosted_rect(rect);
    assert!(!scene.is_empty());
}

// ── 6. rubber-band selection clips to viewport ───────────────────────────────

/// A rubber-band that covers the canvas origin intersects an element there;
/// a far-off element is not visible in the default viewport.
#[test]
fn integration_selection_in_viewport() {
    let vp = Viewport::new(800.0, 600.0);

    // Rubber band centred on canvas origin.
    let mut rb = RubberBand::new([-50.0, -50.0]);
    rb.update([50.0, 50.0]);

    let inside = ElementBounds { id: 1, min: [-10.0, -10.0], max: [10.0, 10.0] };
    let outside = ElementBounds { id: 2, min: [900.0, 900.0], max: [950.0, 950.0] };

    assert!(rb.intersects(&inside));
    assert!(!rb.intersects(&outside));
    // The far-off element's centre is not visible in the default viewport.
    assert!(!vp.is_point_visible([925.0, 925.0]));
}

// ── 7. multiple quads in one scene ───────────────────────────────────────────

#[test]
fn integration_multiple_quads_in_scene() {
    let mut scene = Scene::new();
    for _ in 0..3 {
        scene.push_quad(Quad::default());
    }
    assert_eq!(scene.quads.len(), 3);
}

// ── 8. renderer draws scene, increments frame counter ────────────────────────

#[test]
fn integration_renderer_draws_scene() {
    let mut scene = Scene::new();
    scene.push_quad(Quad::default());
    let mut renderer = Renderer::new();
    renderer.draw(&mut scene);
    assert_eq!(renderer.frame_stats.frames, 1);
}

// ── 9. CTA token is distinct from BG token ───────────────────────────────────

#[test]
fn integration_theme_cta_distinct_from_bg() {
    let bg: [f32; 4] = tokens::BG;
    let cta: [f32; 4] = tokens::CTA;
    assert_ne!(
        bg, cta,
        "CTA token must differ from BG token"
    );
}

// ── 10. fresh viewport: canvas origin maps to screen centre ──────────────────

/// At zoom=1, pan=0 the canvas origin maps to the screen centre — effectively
/// an identity relationship when offset by size/2.
#[test]
fn integration_viewport_default_identity() {
    let vp = Viewport::new(800.0, 600.0);
    let screen = vp.canvas_to_screen([0.0, 0.0]);
    assert!(
        (screen[0] - 400.0).abs() < 1e-5,
        "expected screen x=400, got {}",
        screen[0]
    );
    assert!(
        (screen[1] - 300.0).abs() < 1e-5,
        "expected screen y=300, got {}",
        screen[1]
    );
}

// ── 11. shadows are in a separate vec from quads ─────────────────────────────

#[test]
fn integration_scene_shadows_separate_from_quads() {
    let mut scene = Scene::new();
    scene.push_quad(Quad::default());
    scene.push_shadow(Shadow {
        blur_radius: Pixels(tokens::FROSTED_BLUR_RADIUS),
        color: Hsla::black(),
        ..Default::default()
    });
    assert_eq!(scene.quads.len(), 1, "exactly one quad");
    assert_eq!(scene.shadows.len(), 1, "exactly one shadow");
    assert_eq!(
        scene.quads.len() + scene.shadows.len(),
        2,
        "quad and shadow stored separately"
    );
}

// ── 12. LinearRgba::from(Hsla) preserves full opacity ────────────────────────

#[test]
fn integration_linear_rgba_from_hsla() {
    let full_opacity = Hsla::new(120.0, 0.5, 0.5, 1.0);
    let linear = LinearRgba::from(full_opacity);
    // alpha channel (index 3) must be 1.0 for a fully opaque colour.
    assert!(
        (linear.0[3] - 1.0).abs() < 1e-6,
        "expected alpha=1.0, got {}",
        linear.0[3]
    );
}
