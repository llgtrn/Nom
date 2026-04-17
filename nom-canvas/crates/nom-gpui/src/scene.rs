use crate::types::{
    AtlasTile, Bounds, ContentMask, Corners, Edges, Hsla, PathVertex, Pixels, Point,
    TransformationMatrix,
};

// ---------------------------------------------------------------------------
// GPU primitives
// ---------------------------------------------------------------------------

/// Rounded rectangle with optional fill and border.
#[derive(Debug, Clone, Default)]
pub struct Quad {
    pub bounds: Bounds<Pixels>,
    pub background: Option<Hsla>,
    pub border_color: Option<Hsla>,
    pub border_widths: Edges<Pixels>,
    pub corner_radii: Corners<Pixels>,
    pub content_mask: ContentMask<Pixels>,
}

/// Single-color glyph sprite (text rendering).
#[derive(Debug, Clone, Default)]
pub struct MonochromeSprite {
    pub bounds: Bounds<Pixels>,
    pub content_mask: ContentMask<Pixels>,
    pub color: Hsla,
    pub tile: AtlasTile,
    pub transformation: TransformationMatrix,
}

/// Multi-color sprite (emoji, images).
#[derive(Debug, Clone, Default)]
pub struct PolychromeSprite {
    pub bounds: Bounds<Pixels>,
    pub content_mask: ContentMask<Pixels>,
    pub corner_radii: Corners<Pixels>,
    pub tile: AtlasTile,
    pub grayscale: bool,
}

/// Vector path (bezier curves, connectors).
#[derive(Debug, Clone, Default)]
pub struct Path {
    pub bounds: Bounds<Pixels>,
    pub color: Hsla,
    pub vertices: Vec<PathVertex<Pixels>>,
    pub content_mask: ContentMask<Pixels>,
}

/// Drop shadow (elevation effect).
#[derive(Debug, Clone, Default)]
pub struct Shadow {
    pub bounds: Bounds<Pixels>,
    pub corner_radii: Corners<Pixels>,
    pub blur_radius: Pixels,
    pub color: Hsla,
    pub content_mask: ContentMask<Pixels>,
}

/// Text underline decoration.
#[derive(Debug, Clone, Default)]
pub struct Underline {
    pub origin: Point<Pixels>,
    pub width: Pixels,
    pub thickness: Pixels,
    pub color: Option<Hsla>,
    pub wavy: bool,
    pub content_mask: ContentMask<Pixels>,
}

/// Frosted-glass surface — a blurred, semi-transparent panel region.
///
/// Carries the three frosted-glass token values from `nom_theme::tokens`:
/// `FROSTED_BLUR_RADIUS`, `FROSTED_BG_ALPHA`, `FROSTED_BORDER_ALPHA`.
/// The GPU back-end is expected to execute a Gaussian-blur pre-pass over the
/// framebuffer region described by `bounds` with the given `blur_radius`
/// before compositing the background at `bg_alpha` opacity and the border
/// at `border_alpha` opacity.
#[derive(Debug, Clone, Default)]
pub struct FrostedRect {
    pub bounds: Bounds<Pixels>,
    pub blur_radius: f32,
    pub bg_alpha: f32,
    pub border_alpha: f32,
}

// ---------------------------------------------------------------------------
// Scene — accumulated primitives for one frame
// ---------------------------------------------------------------------------

/// A complete scene for one frame — accumulated primitives, sorted and batched
/// before GPU submission.
#[derive(Debug, Default)]
pub struct Scene {
    pub quads: Vec<Quad>,
    pub monochrome_sprites: Vec<MonochromeSprite>,
    pub polychrome_sprites: Vec<PolychromeSprite>,
    pub paths: Vec<Path>,
    pub shadows: Vec<Shadow>,
    pub underlines: Vec<Underline>,
    pub frosted_rects: Vec<FrostedRect>,
}

impl Scene {
    pub fn new() -> Self { Self::default() }

    /// Painter's algorithm: sort by type for minimal texture switches, no depth
    /// buffer.
    ///
    /// Order: shadows → quads → paths → mono sprites (sorted by texture_id) →
    /// poly sprites → underlines.
    pub fn sort_and_batch(&mut self) {
        // Shadows are processed first (pre-render blur pass) — stable order is fine.
        self.shadows.sort_by_key(|_| 0u8);
        // Sort sprites by texture_id to minimise GPU texture-bind switches.
        self.monochrome_sprites.sort_by_key(|s| s.tile.texture_id);
        self.polychrome_sprites.sort_by_key(|s| s.tile.texture_id);
    }

    pub fn push_quad(&mut self, q: Quad) { self.quads.push(q); }

    pub fn push_sprite(&mut self, s: MonochromeSprite) { self.monochrome_sprites.push(s); }

    pub fn push_poly_sprite(&mut self, s: PolychromeSprite) { self.polychrome_sprites.push(s); }

    pub fn push_path(&mut self, p: Path) { self.paths.push(p); }

    pub fn push_shadow(&mut self, s: Shadow) { self.shadows.push(s); }

    pub fn push_underline(&mut self, u: Underline) { self.underlines.push(u); }

    pub fn push_frosted_rect(&mut self, r: FrostedRect) { self.frosted_rects.push(r); }

    pub fn clear(&mut self) {
        self.quads.clear();
        self.monochrome_sprites.clear();
        self.polychrome_sprites.clear();
        self.paths.clear();
        self.shadows.clear();
        self.underlines.clear();
        self.frosted_rects.clear();
    }

    pub fn is_empty(&self) -> bool {
        self.quads.is_empty()
            && self.monochrome_sprites.is_empty()
            && self.polychrome_sprites.is_empty()
            && self.paths.is_empty()
            && self.shadows.is_empty()
            && self.underlines.is_empty()
            && self.frosted_rects.is_empty()
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{AtlasBounds, Hsla, Pixels, TransformationMatrix};

    #[test]
    fn scene_new_is_empty() {
        let scene = Scene::new();
        assert!(scene.is_empty());
    }

    #[test]
    fn push_quad_adds_to_quads() {
        let mut scene = Scene::new();
        assert_eq!(scene.quads.len(), 0);
        scene.push_quad(Quad {
            background: Some(Hsla::white()),
            ..Default::default()
        });
        assert_eq!(scene.quads.len(), 1);
        assert!(!scene.is_empty());
    }

    #[test]
    fn sort_and_batch_does_not_panic_with_multiple_sprites() {
        let mut scene = Scene::new();

        let make_sprite = |texture_id: u32| MonochromeSprite {
            tile: crate::types::AtlasTile {
                texture_id,
                bounds: AtlasBounds::default(),
                padding: 0.0,
            },
            color: Hsla::black(),
            transformation: TransformationMatrix::identity(),
            ..Default::default()
        };

        scene.push_sprite(make_sprite(3));
        scene.push_sprite(make_sprite(1));
        scene.push_sprite(make_sprite(2));

        // Must not panic; sprites should be ordered by texture_id afterwards.
        scene.sort_and_batch();

        let ids: Vec<u32> =
            scene.monochrome_sprites.iter().map(|s| s.tile.texture_id).collect();
        assert_eq!(ids, vec![1, 2, 3]);

        // Shadow sort should also be stable.
        scene.push_shadow(Shadow {
            blur_radius: Pixels(4.0),
            color: Hsla::black(),
            ..Default::default()
        });
        scene.sort_and_batch(); // must not panic
    }

    #[test]
    fn clear_empties_all_buckets() {
        let mut scene = Scene::new();
        scene.push_quad(Quad::default());
        scene.push_path(Path::default());
        scene.clear();
        assert!(scene.is_empty());
    }

    #[test]
    fn scene_push_quad_adds_to_quads() {
        let mut scene = Scene::new();
        scene.push_quad(Quad {
            background: Some(Hsla::new(120.0, 0.5, 0.5, 1.0)),
            ..Default::default()
        });
        scene.push_quad(Quad::default());
        assert_eq!(scene.quads.len(), 2);
        assert!(!scene.is_empty());
    }

    #[test]
    fn scene_sort_and_batch_stable() {
        let mut scene = Scene::new();
        // Push sprites in reverse texture_id order; sort_and_batch must produce
        // ascending order without losing any entries.
        for id in [5u32, 2, 8, 1, 3] {
            scene.push_sprite(crate::scene::MonochromeSprite {
                tile: crate::types::AtlasTile {
                    texture_id: id,
                    bounds: AtlasBounds::default(),
                    padding: 0.0,
                },
                color: Hsla::white(),
                transformation: TransformationMatrix::identity(),
                ..Default::default()
            });
        }
        scene.sort_and_batch();
        let ids: Vec<u32> =
            scene.monochrome_sprites.iter().map(|s| s.tile.texture_id).collect();
        assert_eq!(ids, vec![1, 2, 3, 5, 8]);
    }

    #[test]
    fn scene_push_path_adds_to_paths() {
        let mut scene = Scene::new();
        scene.push_path(Path {
            color: Hsla::black(),
            ..Default::default()
        });
        assert_eq!(scene.paths.len(), 1);
        scene.push_path(Path::default());
        assert_eq!(scene.paths.len(), 2);
    }

    #[test]
    fn scene_clear_resets_all() {
        let mut scene = Scene::new();
        scene.push_quad(Quad::default());
        scene.push_path(Path::default());
        scene.push_shadow(crate::scene::Shadow::default());
        scene.push_underline(crate::scene::Underline::default());
        assert!(!scene.is_empty());
        scene.clear();
        assert!(scene.is_empty());
        assert_eq!(scene.quads.len(), 0);
        assert_eq!(scene.paths.len(), 0);
        assert_eq!(scene.shadows.len(), 0);
        assert_eq!(scene.underlines.len(), 0);
    }

    #[test]
    fn scene_sort_and_batch_stable_order() {
        // Push quads in reverse order; sort_and_batch must not reorder quads
        // (only sprites are sorted by texture_id), and must not panic.
        let mut scene = Scene::new();
        for i in [3u8, 2, 1] {
            scene.push_quad(Quad {
                background: Some(Hsla::new(i as f32 * 30.0, 0.5, 0.5, 1.0)),
                ..Default::default()
            });
        }
        let first_color = scene.quads[0].background;
        scene.sort_and_batch();
        // Quads are not reordered by sort_and_batch — first entry stays first.
        assert_eq!(scene.quads[0].background, first_color);
        assert_eq!(scene.quads.len(), 3);
    }

    #[test]
    fn scene_paths_bucket() {
        let mut scene = Scene::new();
        scene.push_path(Path {
            color: Hsla::black(),
            ..Default::default()
        });
        assert_eq!(scene.paths.len(), 1);
    }

    #[test]
    fn scene_underlines_bucket() {
        let mut scene = Scene::new();
        scene.push_underline(Underline {
            wavy: true,
            ..Default::default()
        });
        assert_eq!(scene.underlines.len(), 1);
    }

    #[test]
    fn scene_multiple_primitives_mix() {
        let mut scene = Scene::new();
        scene.push_quad(Quad::default());
        scene.push_shadow(Shadow::default());
        scene.push_path(Path::default());
        assert_eq!(scene.quads.len(), 1);
        assert_eq!(scene.shadows.len(), 1);
        assert_eq!(scene.paths.len(), 1);
    }

    #[test]
    fn scene_frosted_rect_distinct_from_quad() {
        let mut scene = Scene::new();
        scene.push_frosted_rect(FrostedRect {
            blur_radius: 8.0,
            bg_alpha: 0.7,
            border_alpha: 0.3,
            ..Default::default()
        });
        scene.push_quad(Quad::default());
        assert_eq!(scene.frosted_rects.len(), 1);
        assert_eq!(scene.quads.len(), 1);
    }

    #[test]
    fn scene_clear_resets_all_buckets() {
        let mut scene = Scene::new();
        scene.push_quad(Quad::default());
        scene.push_shadow(Shadow::default());
        scene.push_path(Path::default());
        scene.push_underline(Underline::default());
        scene.push_frosted_rect(FrostedRect::default());
        scene.push_sprite(MonochromeSprite {
            tile: crate::types::AtlasTile {
                texture_id: 1,
                bounds: AtlasBounds::default(),
                padding: 0.0,
            },
            color: Hsla::white(),
            transformation: TransformationMatrix::identity(),
            ..Default::default()
        });
        assert!(!scene.is_empty());
        scene.clear();
        assert!(scene.is_empty());
        assert_eq!(scene.quads.len(), 0);
        assert_eq!(scene.shadows.len(), 0);
        assert_eq!(scene.paths.len(), 0);
        assert_eq!(scene.underlines.len(), 0);
        assert_eq!(scene.frosted_rects.len(), 0);
        assert_eq!(scene.monochrome_sprites.len(), 0);
    }

    #[test]
    fn scene_shadow_blur_radius() {
        let mut scene = Scene::new();
        scene.push_shadow(Shadow {
            blur_radius: Pixels(12.0),
            color: Hsla::black(),
            ..Default::default()
        });
        assert_eq!(scene.shadows[0].blur_radius, Pixels(12.0));
    }
}
