//! Scene graph: typed collections of paint primitives, batched by type + z-order.
//!
//! Replicates Zed GPUI's 8-vec pattern — each primitive kind has its own
//! homogeneous `Vec<T>` for cache-friendly batched submission to the GPU.
//! `DrawOrder` from [`BoundsTree`](crate::bounds_tree) orders draws across types.

use crate::atlas::AtlasTextureId;
use crate::bounds_tree::DrawOrder;
use crate::color::Rgba;
use crate::geometry::{Bounds, Corners, Point, ScaledPixels, TransformationMatrix};

/// Filled/stroked rounded rectangle with optional shadow/border.
#[derive(Clone, Copy, Debug)]
pub struct Quad {
    pub order: DrawOrder,
    pub bounds: Bounds<ScaledPixels>,
    pub clip_bounds: Bounds<ScaledPixels>,
    pub corner_radii: Corners<ScaledPixels>,
    pub background: Rgba,
    pub border_color: Rgba,
    pub border_widths: [ScaledPixels; 4], // top, right, bottom, left
}

/// Drop shadow (rendered separately from Quad for blur pass).
#[derive(Clone, Copy, Debug)]
pub struct Shadow {
    pub order: DrawOrder,
    pub bounds: Bounds<ScaledPixels>,
    pub clip_bounds: Bounds<ScaledPixels>,
    pub corner_radii: Corners<ScaledPixels>,
    pub color: Rgba,
    pub blur_radius: ScaledPixels,
}

/// Single-color glyph sprite (text). Samples from atlas as R8 mask.
#[derive(Clone, Copy, Debug)]
pub struct MonochromeSprite {
    pub order: DrawOrder,
    pub bounds: Bounds<ScaledPixels>,
    pub clip_bounds: Bounds<ScaledPixels>,
    pub color: Rgba,
    pub tile: AtlasTileRef,
    pub transform: TransformationMatrix,
}

/// Multi-color sprite (emoji, raster image). Samples from atlas as RGBA.
#[derive(Clone, Copy, Debug)]
pub struct PolychromeSprite {
    pub order: DrawOrder,
    pub bounds: Bounds<ScaledPixels>,
    pub clip_bounds: Bounds<ScaledPixels>,
    pub tile: AtlasTileRef,
    pub grayscale: bool,
    pub transform: TransformationMatrix,
}

/// Reference to a sprite tile inside an atlas texture.
#[derive(Clone, Copy, Debug)]
pub struct AtlasTileRef {
    pub texture: AtlasTextureId,
    /// UV rect in [0,1] coordinates.
    pub uv: [f32; 4],
}

/// Text underline (separate primitive for anti-aliased thin strokes).
#[derive(Clone, Copy, Debug)]
pub struct Underline {
    pub order: DrawOrder,
    pub bounds: Bounds<ScaledPixels>,
    pub clip_bounds: Bounds<ScaledPixels>,
    pub color: Rgba,
    pub thickness: ScaledPixels,
    pub wavy: bool,
}

/// Filled bezier/polygon path.
#[derive(Clone, Debug)]
pub struct Path {
    pub order: DrawOrder,
    pub bounds: Bounds<ScaledPixels>,
    pub clip_bounds: Bounds<ScaledPixels>,
    pub vertices: Vec<Point<ScaledPixels>>,
    pub color: Rgba,
}

/// Enum discriminant for batched iteration.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PrimitiveKind {
    Shadow,
    Quad,
    Underline,
    MonochromeSprite,
    PolychromeSprite,
    Path,
}

/// Scene graph: 6 typed collections + optional log for replay/debug.
#[derive(Debug, Default)]
pub struct Scene {
    pub shadows: Vec<Shadow>,
    pub quads: Vec<Quad>,
    pub underlines: Vec<Underline>,
    pub monochrome_sprites: Vec<MonochromeSprite>,
    pub polychrome_sprites: Vec<PolychromeSprite>,
    pub paths: Vec<Path>,
}

/// Stable sort key for `AtlasTextureId` (no `Ord` impl on the type itself).
fn texture_sort_key(t: &AtlasTextureId) -> (u8, u32) {
    use crate::atlas::AtlasTextureKind;
    let kind_ord = match t.kind {
        AtlasTextureKind::Monochrome => 0u8,
        AtlasTextureKind::Subpixel => 1u8,
        AtlasTextureKind::Polychrome => 2u8,
    };
    (kind_ord, t.index)
}

impl Scene {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn clear(&mut self) {
        self.shadows.clear();
        self.quads.clear();
        self.underlines.clear();
        self.monochrome_sprites.clear();
        self.polychrome_sprites.clear();
        self.paths.clear();
    }

    pub fn is_empty(&self) -> bool {
        self.shadows.is_empty()
            && self.quads.is_empty()
            && self.underlines.is_empty()
            && self.monochrome_sprites.is_empty()
            && self.polychrome_sprites.is_empty()
            && self.paths.is_empty()
    }

    pub fn insert_shadow(&mut self, s: Shadow) {
        self.shadows.push(s);
    }

    pub fn insert_quad(&mut self, q: Quad) {
        self.quads.push(q);
    }

    pub fn insert_underline(&mut self, u: Underline) {
        self.underlines.push(u);
    }

    pub fn insert_monochrome_sprite(&mut self, s: MonochromeSprite) {
        self.monochrome_sprites.push(s);
    }

    pub fn insert_polychrome_sprite(&mut self, s: PolychromeSprite) {
        self.polychrome_sprites.push(s);
    }

    pub fn insert_path(&mut self, p: Path) {
        self.paths.push(p);
    }

    /// Sort each collection by draw order, and sprite collections additionally
    /// by texture so that same-texture sprites are contiguous within a z-band.
    /// Call once before batching.
    pub fn finish(&mut self) {
        self.shadows.sort_by_key(|p| p.order);
        self.quads.sort_by_key(|p| p.order);
        self.underlines.sort_by_key(|p| p.order);
        self.paths.sort_by_key(|p| p.order);
        // Sprite collections: primary sort by order, secondary by texture so the
        // batch iterator can break on texture_id changes within a single z-band.
        self.monochrome_sprites
            .sort_by_key(|s| (s.order, texture_sort_key(&s.tile.texture)));
        self.polychrome_sprites
            .sort_by_key(|s| (s.order, texture_sort_key(&s.tile.texture)));
    }

    /// Iterator that produces `PrimitiveBatch`es in z-order: each batch is a
    /// contiguous run of primitives of the same kind, so the renderer can
    /// issue a single `draw_indexed` per batch.
    pub fn batches(&self) -> BatchIterator<'_> {
        BatchIterator {
            scene: self,
            shadow_i: 0,
            quad_i: 0,
            underline_i: 0,
            mono_i: 0,
            poly_i: 0,
            path_i: 0,
        }
    }
}

/// A contiguous run of same-kind primitives at consecutive draw orders.
/// Sprite variants carry the `texture_id` so the renderer can bind the right
/// atlas without scanning the slice.
#[derive(Debug)]
pub enum PrimitiveBatch<'a> {
    Shadows(&'a [Shadow]),
    Quads(&'a [Quad]),
    Underlines(&'a [Underline]),
    MonochromeSprites {
        texture_id: AtlasTextureId,
        sprites: &'a [MonochromeSprite],
    },
    PolychromeSprites {
        texture_id: AtlasTextureId,
        sprites: &'a [PolychromeSprite],
    },
    Paths(&'a [Path]),
}

pub struct BatchIterator<'a> {
    scene: &'a Scene,
    shadow_i: usize,
    quad_i: usize,
    underline_i: usize,
    mono_i: usize,
    poly_i: usize,
    path_i: usize,
}

impl<'a> Iterator for BatchIterator<'a> {
    type Item = PrimitiveBatch<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        // Peek the next draw order in each collection.
        let peeks: [(PrimitiveKind, Option<DrawOrder>); 6] = [
            (PrimitiveKind::Shadow, self.scene.shadows.get(self.shadow_i).map(|p| p.order)),
            (PrimitiveKind::Quad, self.scene.quads.get(self.quad_i).map(|p| p.order)),
            (PrimitiveKind::Underline, self.scene.underlines.get(self.underline_i).map(|p| p.order)),
            (PrimitiveKind::MonochromeSprite, self.scene.monochrome_sprites.get(self.mono_i).map(|p| p.order)),
            (PrimitiveKind::PolychromeSprite, self.scene.polychrome_sprites.get(self.poly_i).map(|p| p.order)),
            (PrimitiveKind::Path, self.scene.paths.get(self.path_i).map(|p| p.order)),
        ];

        // Pick the kind with the lowest next order (stable tiebreak: enum
        // declaration order via the array index).
        let (selected_idx, (kind, _)) = peeks
            .iter()
            .enumerate()
            .filter_map(|(i, (k, o))| o.map(|v| (i, (*k, v))))
            .min_by_key(|(_, (_, o))| *o)?;

        // cutoff = min order among ALL OTHER kinds (u32::MAX when all others empty).
        // We may emit items while item.order <= cutoff without reordering with
        // another kind's primitives.
        let cutoff: DrawOrder = peeks
            .iter()
            .enumerate()
            .filter(|(i, _)| *i != selected_idx)
            .filter_map(|(_, (_, o))| *o)
            .min()
            .unwrap_or(u32::MAX);

        Some(match kind {
            PrimitiveKind::Shadow => {
                let start = self.shadow_i;
                let end = advance_while(&self.scene.shadows, start, |s| s.order <= cutoff);
                self.shadow_i = end;
                PrimitiveBatch::Shadows(&self.scene.shadows[start..end])
            }
            PrimitiveKind::Quad => {
                let start = self.quad_i;
                let end = advance_while(&self.scene.quads, start, |q| q.order <= cutoff);
                self.quad_i = end;
                PrimitiveBatch::Quads(&self.scene.quads[start..end])
            }
            PrimitiveKind::Underline => {
                let start = self.underline_i;
                let end = advance_while(&self.scene.underlines, start, |u| u.order <= cutoff);
                self.underline_i = end;
                PrimitiveBatch::Underlines(&self.scene.underlines[start..end])
            }
            PrimitiveKind::MonochromeSprite => {
                let start = self.mono_i;
                let texture_id = self.scene.monochrome_sprites[start].tile.texture;
                let end = advance_while(&self.scene.monochrome_sprites, start, |s| {
                    s.order <= cutoff && s.tile.texture == texture_id
                });
                self.mono_i = end;
                PrimitiveBatch::MonochromeSprites {
                    texture_id,
                    sprites: &self.scene.monochrome_sprites[start..end],
                }
            }
            PrimitiveKind::PolychromeSprite => {
                let start = self.poly_i;
                let texture_id = self.scene.polychrome_sprites[start].tile.texture;
                let end = advance_while(&self.scene.polychrome_sprites, start, |s| {
                    s.order <= cutoff && s.tile.texture == texture_id
                });
                self.poly_i = end;
                PrimitiveBatch::PolychromeSprites {
                    texture_id,
                    sprites: &self.scene.polychrome_sprites[start..end],
                }
            }
            PrimitiveKind::Path => {
                let start = self.path_i;
                let end = advance_while(&self.scene.paths, start, |p| p.order <= cutoff);
                self.path_i = end;
                PrimitiveBatch::Paths(&self.scene.paths[start..end])
            }
        })
    }
}

/// Advance cursor from `start` while `predicate` holds, returning the new end
/// index (exclusive). Always advances by at least 1 (the item at `start` is
/// assumed to already satisfy the batch conditions).
fn advance_while<T, F: Fn(&T) -> bool>(slice: &[T], start: usize, predicate: F) -> usize {
    let mut end = start + 1; // always consume at least the current item
    while end < slice.len() && predicate(&slice[end]) {
        end += 1;
    }
    end
}

#[allow(dead_code)]
trait HasOrder {
    fn order(&self) -> DrawOrder;
}

macro_rules! impl_has_order {
    ($($t:ty),*) => {
        $(impl HasOrder for $t {
            fn order(&self) -> DrawOrder { self.order }
        })*
    };
}
impl_has_order!(Shadow, Quad, Underline, MonochromeSprite, PolychromeSprite, Path);

#[cfg(test)]
mod tests {
    use super::*;
    use crate::atlas::{AtlasTextureKind, AtlasTextureId};
    use crate::geometry::Size;

    fn sp_bounds(x: f32, y: f32, w: f32, h: f32) -> Bounds<ScaledPixels> {
        Bounds {
            origin: Point {
                x: ScaledPixels(x),
                y: ScaledPixels(y),
            },
            size: Size {
                width: ScaledPixels(w),
                height: ScaledPixels(h),
            },
        }
    }

    fn quad(order: DrawOrder) -> Quad {
        Quad {
            order,
            bounds: sp_bounds(0.0, 0.0, 10.0, 10.0),
            clip_bounds: sp_bounds(0.0, 0.0, 1000.0, 1000.0),
            corner_radii: Corners::all(ScaledPixels(0.0)),
            background: Rgba::WHITE,
            border_color: Rgba::TRANSPARENT,
            border_widths: [ScaledPixels(0.0); 4],
        }
    }

    fn shadow(order: DrawOrder) -> Shadow {
        Shadow {
            order,
            bounds: sp_bounds(0.0, 0.0, 10.0, 10.0),
            clip_bounds: sp_bounds(0.0, 0.0, 1000.0, 1000.0),
            corner_radii: Corners::all(ScaledPixels(0.0)),
            color: Rgba::BLACK,
            blur_radius: ScaledPixels(4.0),
        }
    }

    fn tex(index: u32) -> AtlasTextureId {
        AtlasTextureId {
            kind: AtlasTextureKind::Monochrome,
            index,
        }
    }

    fn poly_tex(index: u32) -> AtlasTextureId {
        AtlasTextureId {
            kind: AtlasTextureKind::Polychrome,
            index,
        }
    }

    fn mono_sprite(order: DrawOrder, texture_index: u32) -> MonochromeSprite {
        MonochromeSprite {
            order,
            bounds: sp_bounds(0.0, 0.0, 8.0, 8.0),
            clip_bounds: sp_bounds(0.0, 0.0, 1000.0, 1000.0),
            color: Rgba::WHITE,
            tile: AtlasTileRef {
                texture: tex(texture_index),
                uv: [0.0, 0.0, 1.0, 1.0],
            },
            transform: TransformationMatrix::IDENTITY,
        }
    }

    fn poly_sprite(order: DrawOrder, texture_index: u32) -> PolychromeSprite {
        PolychromeSprite {
            order,
            bounds: sp_bounds(0.0, 0.0, 8.0, 8.0),
            clip_bounds: sp_bounds(0.0, 0.0, 1000.0, 1000.0),
            tile: AtlasTileRef {
                texture: poly_tex(texture_index),
                uv: [0.0, 0.0, 1.0, 1.0],
            },
            grayscale: false,
            transform: TransformationMatrix::IDENTITY,
        }
    }

    // --- Original tests (must still pass) ---

    #[test]
    fn empty_scene_has_no_batches() {
        let s = Scene::new();
        assert_eq!(s.batches().count(), 0);
    }

    #[test]
    fn finish_sorts_by_order() {
        let mut s = Scene::new();
        s.insert_quad(quad(3));
        s.insert_quad(quad(1));
        s.insert_quad(quad(2));
        s.finish();
        let orders: Vec<_> = s.quads.iter().map(|q| q.order).collect();
        assert_eq!(orders, vec![1, 2, 3]);
    }

    #[test]
    fn batches_picks_lowest_order_kind_first() {
        let mut s = Scene::new();
        s.insert_quad(quad(5));
        s.insert_shadow(shadow(1));
        s.finish();
        let kinds: Vec<_> = s
            .batches()
            .map(|b| match b {
                PrimitiveBatch::Shadows(_) => "shadow",
                PrimitiveBatch::Quads(_) => "quad",
                _ => "other",
            })
            .collect();
        assert_eq!(kinds, vec!["shadow", "quad"]);
    }

    #[test]
    fn clear_resets_everything() {
        let mut s = Scene::new();
        s.insert_quad(quad(0));
        s.insert_shadow(shadow(1));
        assert!(!s.is_empty());
        s.clear();
        assert!(s.is_empty());
    }

    // --- New tests for Bug 1 fix: interleaved z-order ---

    /// Interleaved shadow/quad: shadow@1, quad@5, shadow@10 must emit
    /// three separate batches in correct z-order, not one big shadow batch.
    #[test]
    fn interleaved_kinds_emit_correct_z_order() {
        let mut s = Scene::new();
        s.insert_shadow(shadow(1));
        s.insert_quad(quad(5));
        s.insert_shadow(shadow(10));
        s.finish();

        let batches: Vec<_> = s.batches().collect();
        assert_eq!(batches.len(), 3, "expected 3 batches for interleaved shadow/quad/shadow");

        // Batch 0: shadow@1 (stops because quad@5 has lower order than shadow@10)
        match &batches[0] {
            PrimitiveBatch::Shadows(s) => {
                assert_eq!(s.len(), 1);
                assert_eq!(s[0].order, 1);
            }
            other => panic!("expected Shadows, got {:?}", other),
        }

        // Batch 1: quad@5
        match &batches[1] {
            PrimitiveBatch::Quads(q) => {
                assert_eq!(q.len(), 1);
                assert_eq!(q[0].order, 5);
            }
            other => panic!("expected Quads, got {:?}", other),
        }

        // Batch 2: shadow@10
        match &batches[2] {
            PrimitiveBatch::Shadows(s) => {
                assert_eq!(s.len(), 1);
                assert_eq!(s[0].order, 10);
            }
            other => panic!("expected Shadows, got {:?}", other),
        }
    }

    /// When only one kind is present, all items are emitted in one batch
    /// (no other kind to interleave with, cutoff = u32::MAX).
    #[test]
    fn single_kind_emits_all_in_one_batch() {
        let mut s = Scene::new();
        s.insert_quad(quad(1));
        s.insert_quad(quad(2));
        s.insert_quad(quad(3));
        s.finish();

        let batches: Vec<_> = s.batches().collect();
        assert_eq!(batches.len(), 1, "all quads should be one batch with no interleaving");
        match &batches[0] {
            PrimitiveBatch::Quads(q) => assert_eq!(q.len(), 3),
            other => panic!("expected Quads, got {:?}", other),
        }
    }

    // --- New tests for Bug 2 fix: sprite texture_id batching ---

    /// Sprites with different textures at different orders break into separate batches.
    #[test]
    fn sprites_with_different_textures_break_batches() {
        let mut s = Scene::new();
        // Two sprites, same order, different textures
        s.insert_monochrome_sprite(mono_sprite(1, 0));
        s.insert_monochrome_sprite(mono_sprite(2, 1)); // different texture
        s.finish();

        let batches: Vec<_> = s.batches().collect();
        assert_eq!(batches.len(), 2, "different textures must produce separate batches");
        match &batches[0] {
            PrimitiveBatch::MonochromeSprites { texture_id, sprites } => {
                assert_eq!(texture_id.index, 0);
                assert_eq!(sprites.len(), 1);
            }
            other => panic!("expected MonochromeSprites, got {:?}", other),
        }
        match &batches[1] {
            PrimitiveBatch::MonochromeSprites { texture_id, sprites } => {
                assert_eq!(texture_id.index, 1);
                assert_eq!(sprites.len(), 1);
            }
            other => panic!("expected MonochromeSprites, got {:?}", other),
        }
    }

    /// Polychrome sprites with different textures break into separate batches.
    #[test]
    fn polychrome_sprites_with_different_textures_break_batches() {
        let mut s = Scene::new();
        s.insert_polychrome_sprite(poly_sprite(3, 0));
        s.insert_polychrome_sprite(poly_sprite(4, 1)); // different texture
        s.finish();

        let batches: Vec<_> = s.batches().collect();
        assert_eq!(batches.len(), 2, "polychrome different textures must be separate batches");
        match &batches[0] {
            PrimitiveBatch::PolychromeSprites { texture_id, sprites } => {
                assert_eq!(texture_id.index, 0);
                assert_eq!(sprites.len(), 1);
            }
            other => panic!("expected PolychromeSprites batch 0, got {:?}", other),
        }
        match &batches[1] {
            PrimitiveBatch::PolychromeSprites { texture_id, sprites } => {
                assert_eq!(texture_id.index, 1);
                assert_eq!(sprites.len(), 1);
            }
            other => panic!("expected PolychromeSprites batch 1, got {:?}", other),
        }
    }

    /// Two sprites at the same order with the same texture stay in one batch;
    /// a third sprite at the same order but different texture breaks into a new batch.
    #[test]
    fn texture_id_break_within_same_order() {
        let mut s = Scene::new();
        s.insert_monochrome_sprite(mono_sprite(5, 0)); // tex 0
        s.insert_monochrome_sprite(mono_sprite(5, 0)); // tex 0 — same batch
        s.insert_monochrome_sprite(mono_sprite(5, 1)); // tex 1 — new batch
        s.finish();

        let batches: Vec<_> = s.batches().collect();
        assert_eq!(batches.len(), 2, "texture change at same order must break batch");
        match &batches[0] {
            PrimitiveBatch::MonochromeSprites { texture_id, sprites } => {
                assert_eq!(texture_id.index, 0);
                assert_eq!(sprites.len(), 2);
            }
            other => panic!("expected MonochromeSprites batch 0, got {:?}", other),
        }
        match &batches[1] {
            PrimitiveBatch::MonochromeSprites { texture_id, sprites } => {
                assert_eq!(texture_id.index, 1);
                assert_eq!(sprites.len(), 1);
            }
            other => panic!("expected MonochromeSprites batch 1, got {:?}", other),
        }
    }
}
