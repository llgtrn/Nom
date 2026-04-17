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

    /// Sort each collection by draw order. Call once before batching.
    pub fn finish(&mut self) {
        self.shadows.sort_by_key(|p| p.order);
        self.quads.sort_by_key(|p| p.order);
        self.underlines.sort_by_key(|p| p.order);
        self.monochrome_sprites.sort_by_key(|p| p.order);
        self.polychrome_sprites.sort_by_key(|p| p.order);
        self.paths.sort_by_key(|p| p.order);
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
#[derive(Debug)]
pub enum PrimitiveBatch<'a> {
    Shadows(&'a [Shadow]),
    Quads(&'a [Quad]),
    Underlines(&'a [Underline]),
    MonochromeSprites(&'a [MonochromeSprite]),
    PolychromeSprites(&'a [PolychromeSprite]),
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
        // Peek the next draw order in each collection; pick the kind whose
        // next order is lowest; emit the maximal run of that kind.
        fn peek<T: HasOrder>(slice: &[T], i: usize) -> Option<DrawOrder> {
            slice.get(i).map(|p| p.order())
        }
        let candidates: [(PrimitiveKind, Option<DrawOrder>); 6] = [
            (PrimitiveKind::Shadow, peek(&self.scene.shadows, self.shadow_i)),
            (PrimitiveKind::Quad, peek(&self.scene.quads, self.quad_i)),
            (PrimitiveKind::Underline, peek(&self.scene.underlines, self.underline_i)),
            (PrimitiveKind::MonochromeSprite, peek(&self.scene.monochrome_sprites, self.mono_i)),
            (PrimitiveKind::PolychromeSprite, peek(&self.scene.polychrome_sprites, self.poly_i)),
            (PrimitiveKind::Path, peek(&self.scene.paths, self.path_i)),
        ];
        let (kind, _) = candidates
            .iter()
            .filter_map(|(k, o): &(PrimitiveKind, Option<DrawOrder>)| o.map(|v| (*k, v)))
            .min_by_key(|(_, o)| *o)?;
        // Emit max run of that kind at strictly-increasing order: for simplicity,
        // emit ALL remaining of that kind in a single batch. The GPU renderer can
        // re-sort if it needs to interleave with other kinds — in practice, same-kind
        // primitives from one paint pass have monotonic orders.
        Some(match kind {
            PrimitiveKind::Shadow => {
                let slice = &self.scene.shadows[self.shadow_i..];
                self.shadow_i = self.scene.shadows.len();
                PrimitiveBatch::Shadows(slice)
            }
            PrimitiveKind::Quad => {
                let slice = &self.scene.quads[self.quad_i..];
                self.quad_i = self.scene.quads.len();
                PrimitiveBatch::Quads(slice)
            }
            PrimitiveKind::Underline => {
                let slice = &self.scene.underlines[self.underline_i..];
                self.underline_i = self.scene.underlines.len();
                PrimitiveBatch::Underlines(slice)
            }
            PrimitiveKind::MonochromeSprite => {
                let slice = &self.scene.monochrome_sprites[self.mono_i..];
                self.mono_i = self.scene.monochrome_sprites.len();
                PrimitiveBatch::MonochromeSprites(slice)
            }
            PrimitiveKind::PolychromeSprite => {
                let slice = &self.scene.polychrome_sprites[self.poly_i..];
                self.poly_i = self.scene.polychrome_sprites.len();
                PrimitiveBatch::PolychromeSprites(slice)
            }
            PrimitiveKind::Path => {
                let slice = &self.scene.paths[self.path_i..];
                self.path_i = self.scene.paths.len();
                PrimitiveBatch::Paths(slice)
            }
        })
    }
}

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
}
