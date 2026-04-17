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

/// Subpixel-rendered glyph sprite (LCD/ClearType text). Samples from atlas
/// as an RGB subpixel mask — distinct from MonochromeSprite to allow a
/// separate blend mode in the shader.
#[derive(Clone, Copy, Debug)]
pub struct SubpixelSprite {
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
    SubpixelSprite,
    PolychromeSprite,
    Path,
}

/// Returned by [`Scene::hit_test`]: identifies the topmost primitive under a point.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct HitResult {
    /// Which primitive kind was hit.
    pub kind: PrimitiveKind,
    /// Index of the primitive in its collection (e.g. `scene.quads()[index]`).
    pub index: usize,
    /// The draw order of the hit primitive.
    pub order: DrawOrder,
}

/// Scene graph: 7 typed collections.
/// Fields are `pub(crate)` so the renderer (in the same crate) can access
/// them directly; external callers use the read-only slice accessors below.
#[derive(Debug, Default)]
pub struct Scene {
    pub(crate) shadows: Vec<Shadow>,
    pub(crate) quads: Vec<Quad>,
    pub(crate) underlines: Vec<Underline>,
    pub(crate) monochrome_sprites: Vec<MonochromeSprite>,
    pub(crate) subpixel_sprites: Vec<SubpixelSprite>,
    pub(crate) polychrome_sprites: Vec<PolychromeSprite>,
    pub(crate) paths: Vec<Path>,
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
        self.subpixel_sprites.clear();
        self.polychrome_sprites.clear();
        self.paths.clear();
    }

    pub fn is_empty(&self) -> bool {
        self.shadows.is_empty()
            && self.quads.is_empty()
            && self.underlines.is_empty()
            && self.monochrome_sprites.is_empty()
            && self.subpixel_sprites.is_empty()
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

    pub fn insert_subpixel_sprite(&mut self, s: SubpixelSprite) {
        self.subpixel_sprites.push(s);
    }

    pub fn insert_polychrome_sprite(&mut self, s: PolychromeSprite) {
        self.polychrome_sprites.push(s);
    }

    pub fn insert_path(&mut self, p: Path) {
        self.paths.push(p);
    }

    /// Return the topmost-order primitive whose bounds contain `point`, if any.
    ///
    /// Iterates all 7 primitive collections and picks the one with the highest
    /// `DrawOrder` whose `bounds` contains the point. O(N) in total primitive
    /// count — fine for MVP frames with < 10k primitives. Caller is responsible
    /// for applying clip_bounds before routing pointer events.
    pub fn hit_test(&self, point: Point<ScaledPixels>) -> Option<HitResult> {
        let mut best: Option<HitResult> = None;

        macro_rules! check_collection {
            ($coll:expr, $kind:expr) => {
                for (idx, prim) in $coll.iter().enumerate() {
                    if prim.bounds.contains(point) {
                        let candidate = HitResult {
                            kind: $kind,
                            index: idx,
                            order: prim.order,
                        };
                        let replace = match &best {
                            None => true,
                            Some(b) => candidate.order > b.order,
                        };
                        if replace {
                            best = Some(candidate);
                        }
                    }
                }
            };
        }

        check_collection!(self.shadows, PrimitiveKind::Shadow);
        check_collection!(self.quads, PrimitiveKind::Quad);
        check_collection!(self.underlines, PrimitiveKind::Underline);
        check_collection!(self.monochrome_sprites, PrimitiveKind::MonochromeSprite);
        check_collection!(self.subpixel_sprites, PrimitiveKind::SubpixelSprite);
        check_collection!(self.polychrome_sprites, PrimitiveKind::PolychromeSprite);
        check_collection!(self.paths, PrimitiveKind::Path);

        best
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
        self.subpixel_sprites
            .sort_by_key(|s| (s.order, texture_sort_key(&s.tile.texture)));
        self.polychrome_sprites
            .sort_by_key(|s| (s.order, texture_sort_key(&s.tile.texture)));
    }

    // --- Read-only slice accessors (public API) ---

    pub fn shadows(&self) -> &[Shadow] {
        &self.shadows
    }

    pub fn quads(&self) -> &[Quad] {
        &self.quads
    }

    pub fn underlines(&self) -> &[Underline] {
        &self.underlines
    }

    pub fn monochrome_sprites(&self) -> &[MonochromeSprite] {
        &self.monochrome_sprites
    }

    pub fn subpixel_sprites(&self) -> &[SubpixelSprite] {
        &self.subpixel_sprites
    }

    pub fn polychrome_sprites(&self) -> &[PolychromeSprite] {
        &self.polychrome_sprites
    }

    pub fn paths(&self) -> &[Path] {
        &self.paths
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
            subpixel_i: 0,
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
    SubpixelSprites {
        texture_id: AtlasTextureId,
        sprites: &'a [SubpixelSprite],
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
    subpixel_i: usize,
    poly_i: usize,
    path_i: usize,
}

impl<'a> Iterator for BatchIterator<'a> {
    type Item = PrimitiveBatch<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        // Peek the next draw order in each collection.
        let peeks: [(PrimitiveKind, Option<DrawOrder>); 7] = [
            (PrimitiveKind::Shadow, self.scene.shadows.get(self.shadow_i).map(|p| p.order)),
            (PrimitiveKind::Quad, self.scene.quads.get(self.quad_i).map(|p| p.order)),
            (PrimitiveKind::Underline, self.scene.underlines.get(self.underline_i).map(|p| p.order)),
            (PrimitiveKind::MonochromeSprite, self.scene.monochrome_sprites.get(self.mono_i).map(|p| p.order)),
            (PrimitiveKind::SubpixelSprite, self.scene.subpixel_sprites.get(self.subpixel_i).map(|p| p.order)),
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
            PrimitiveKind::SubpixelSprite => {
                let start = self.subpixel_i;
                let texture_id = self.scene.subpixel_sprites[start].tile.texture;
                let end = advance_while(&self.scene.subpixel_sprites, start, |s| {
                    s.order <= cutoff && s.tile.texture == texture_id
                });
                self.subpixel_i = end;
                PrimitiveBatch::SubpixelSprites {
                    texture_id,
                    sprites: &self.scene.subpixel_sprites[start..end],
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
impl_has_order!(Shadow, Quad, Underline, MonochromeSprite, SubpixelSprite, PolychromeSprite, Path);

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

    fn subpixel_tex(index: u32) -> AtlasTextureId {
        AtlasTextureId {
            kind: AtlasTextureKind::Subpixel,
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

    fn subpixel_sprite_helper(order: DrawOrder, texture_index: u32) -> SubpixelSprite {
        SubpixelSprite {
            order,
            bounds: sp_bounds(0.0, 0.0, 8.0, 8.0),
            clip_bounds: sp_bounds(0.0, 0.0, 1000.0, 1000.0),
            color: Rgba::WHITE,
            tile: AtlasTileRef {
                texture: subpixel_tex(texture_index),
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
        let orders: Vec<_> = s.quads().iter().map(|q| q.order).collect();
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

        match &batches[0] {
            PrimitiveBatch::Shadows(s) => {
                assert_eq!(s.len(), 1);
                assert_eq!(s[0].order, 1);
            }
            other => panic!("expected Shadows, got {:?}", other),
        }

        match &batches[1] {
            PrimitiveBatch::Quads(q) => {
                assert_eq!(q.len(), 1);
                assert_eq!(q[0].order, 5);
            }
            other => panic!("expected Quads, got {:?}", other),
        }

        match &batches[2] {
            PrimitiveBatch::Shadows(s) => {
                assert_eq!(s.len(), 1);
                assert_eq!(s[0].order, 10);
            }
            other => panic!("expected Shadows, got {:?}", other),
        }
    }

    /// When only one kind is present, all items are emitted in one batch.
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

    #[test]
    fn sprites_with_different_textures_break_batches() {
        let mut s = Scene::new();
        s.insert_monochrome_sprite(mono_sprite(1, 0));
        s.insert_monochrome_sprite(mono_sprite(2, 1));
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

    #[test]
    fn polychrome_sprites_with_different_textures_break_batches() {
        let mut s = Scene::new();
        s.insert_polychrome_sprite(poly_sprite(3, 0));
        s.insert_polychrome_sprite(poly_sprite(4, 1));
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

    #[test]
    fn texture_id_break_within_same_order() {
        let mut s = Scene::new();
        s.insert_monochrome_sprite(mono_sprite(5, 0));
        s.insert_monochrome_sprite(mono_sprite(5, 0));
        s.insert_monochrome_sprite(mono_sprite(5, 1));
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

    // --- iter-4 audit: ABA sprite texture pattern ---

    /// ABA texture pattern: sprites at orders 1(A), 2(B), 3(A).
    /// After finish() sorts by (order, texture), the sequence is A@1, B@2, A@3 — ABA.
    /// The batch iterator breaks on texture_id changes, so this produces 3 batches.
    /// (There is no texture-merge across different draw orders even if the texture repeats.)
    // TEST-GAP-CLOSER: adapted to SubpixelSprite + pub(crate) field refactor post-merge.
    #[test]
    fn sprite_aba_texture_pattern_produces_three_batches() {
        let mut scene = Scene::new();
        // tex index 0 = A, tex index 1 = B
        scene.insert_monochrome_sprite(mono_sprite(1, 0)); // A @ order 1
        scene.insert_monochrome_sprite(mono_sprite(2, 1)); // B @ order 2
        scene.insert_monochrome_sprite(mono_sprite(3, 0)); // A @ order 3
        scene.finish();

        // After sort_by_key((order, texture)): A@1, B@2, A@3 → ABA
        let batches: Vec<_> = scene.batches().collect();
        let sprite_batches: Vec<_> = batches
            .iter()
            .filter_map(|b| match b {
                PrimitiveBatch::MonochromeSprites { texture_id, sprites } => {
                    Some((texture_id.index, sprites.len()))
                }
                _ => None,
            })
            .collect();

        // ABA: 3 distinct batches because texture changes between every consecutive pair.
        assert_eq!(
            sprite_batches.len(),
            3,
            "ABA pattern must produce 3 batches, got {:?}",
            sprite_batches
        );
        // First and last have texture A (index 0); middle has texture B (index 1).
        assert_eq!(
            sprite_batches[0].0, sprite_batches[2].0,
            "first and last batch should share texture A"
        );
        assert_ne!(
            sprite_batches[0].0, sprite_batches[1].0,
            "first and middle batch must differ (A vs B)"
        );
        // Each batch holds exactly one sprite.
        assert_eq!(sprite_batches[0].1, 1);
        assert_eq!(sprite_batches[1].1, 1);
        assert_eq!(sprite_batches[2].1, 1);
    }

    // --- Fix 1 tests: SubpixelSprite ---

    /// SubpixelSprite and MonochromeSprite at the same order are separate batches
    /// because they are different primitive kinds.
    #[test]
    fn subpixel_sprites_separate_from_monochrome() {
        let mut s = Scene::new();
        s.insert_monochrome_sprite(mono_sprite(1, 0));
        s.insert_subpixel_sprite(subpixel_sprite_helper(1, 0));
        s.finish();

        let batches: Vec<_> = s.batches().collect();
        assert_eq!(
            batches.len(),
            2,
            "MonochromeSprite and SubpixelSprite must be separate batches even at same order"
        );

        let kinds: Vec<&str> = batches
            .iter()
            .map(|b| match b {
                PrimitiveBatch::MonochromeSprites { .. } => "mono",
                PrimitiveBatch::SubpixelSprites { .. } => "subpixel",
                _ => "other",
            })
            .collect();
        // MonochromeSprite has enum index 3, SubpixelSprite index 4 — mono wins tiebreak.
        assert_eq!(kinds, vec!["mono", "subpixel"]);
    }

    /// SubpixelSprites with different textures break into separate batches.
    #[test]
    fn subpixel_sprites_different_textures_break_batches() {
        let mut s = Scene::new();
        s.insert_subpixel_sprite(subpixel_sprite_helper(2, 0));
        s.insert_subpixel_sprite(subpixel_sprite_helper(3, 1));
        s.finish();

        let batches: Vec<_> = s.batches().collect();
        assert_eq!(batches.len(), 2, "different subpixel textures must be separate batches");
        match &batches[0] {
            PrimitiveBatch::SubpixelSprites { texture_id, sprites } => {
                assert_eq!(texture_id.index, 0);
                assert_eq!(sprites.len(), 1);
            }
            other => panic!("expected SubpixelSprites batch 0, got {:?}", other),
        }
        match &batches[1] {
            PrimitiveBatch::SubpixelSprites { texture_id, sprites } => {
                assert_eq!(texture_id.index, 1);
                assert_eq!(sprites.len(), 1);
            }
            other => panic!("expected SubpixelSprites batch 1, got {:?}", other),
        }
    }

    // --- hit_test tests ---

    fn quad_at(order: DrawOrder, x: f32, y: f32, w: f32, h: f32) -> Quad {
        Quad {
            order,
            bounds: sp_bounds(x, y, w, h),
            clip_bounds: sp_bounds(0.0, 0.0, 1000.0, 1000.0),
            corner_radii: Corners::all(ScaledPixels(0.0)),
            background: Rgba::WHITE,
            border_color: Rgba::TRANSPARENT,
            border_widths: [ScaledPixels(0.0); 4],
        }
    }

    fn pt(x: f32, y: f32) -> Point<ScaledPixels> {
        Point { x: ScaledPixels(x), y: ScaledPixels(y) }
    }

    #[test]
    fn hit_test_returns_topmost_quad_at_point() {
        let mut s = Scene::new();
        s.insert_quad(quad_at(1, 0.0, 0.0, 100.0, 100.0));
        // Hit inside the only quad.
        let result = s.hit_test(pt(50.0, 50.0));
        assert!(result.is_some());
        let r = result.unwrap();
        assert_eq!(r.kind, PrimitiveKind::Quad);
        assert_eq!(r.index, 0);
        assert_eq!(r.order, 1);
    }

    #[test]
    fn hit_test_returns_none_outside_all_bounds() {
        let mut s = Scene::new();
        s.insert_quad(quad_at(1, 0.0, 0.0, 50.0, 50.0));
        // Point outside the quad.
        let result = s.hit_test(pt(200.0, 200.0));
        assert!(result.is_none());
    }

    #[test]
    fn hit_test_chooses_highest_order_when_stacked() {
        let mut s = Scene::new();
        // Two quads covering the same region; order 5 is on top.
        s.insert_quad(quad_at(2, 0.0, 0.0, 100.0, 100.0));
        s.insert_quad(quad_at(5, 10.0, 10.0, 80.0, 80.0));
        s.insert_quad(quad_at(3, 0.0, 0.0, 100.0, 100.0));
        // Point inside all three quads.
        let result = s.hit_test(pt(50.0, 50.0));
        assert!(result.is_some());
        let r = result.unwrap();
        assert_eq!(r.order, 5, "highest order quad should win");
        assert_eq!(r.kind, PrimitiveKind::Quad);
    }

    // --- Fix 3 tests: accessor methods ---

    #[test]
    fn accessors_return_correct_slices() {
        let mut s = Scene::new();
        s.insert_shadow(shadow(1));
        s.insert_quad(quad(2));
        s.insert_underline(Underline {
            order: 3,
            bounds: sp_bounds(0.0, 0.0, 10.0, 2.0),
            clip_bounds: sp_bounds(0.0, 0.0, 1000.0, 1000.0),
            color: Rgba::BLACK,
            thickness: ScaledPixels(1.0),
            wavy: false,
        });
        s.insert_monochrome_sprite(mono_sprite(4, 0));
        s.insert_subpixel_sprite(subpixel_sprite_helper(5, 0));
        s.insert_polychrome_sprite(poly_sprite(6, 0));
        s.insert_path(Path {
            order: 7,
            bounds: sp_bounds(0.0, 0.0, 10.0, 10.0),
            clip_bounds: sp_bounds(0.0, 0.0, 1000.0, 1000.0),
            vertices: vec![],
            color: Rgba::BLACK,
        });

        assert_eq!(s.shadows().len(), 1);
        assert_eq!(s.quads().len(), 1);
        assert_eq!(s.underlines().len(), 1);
        assert_eq!(s.monochrome_sprites().len(), 1);
        assert_eq!(s.subpixel_sprites().len(), 1);
        assert_eq!(s.polychrome_sprites().len(), 1);
        assert_eq!(s.paths().len(), 1);
    }
}
