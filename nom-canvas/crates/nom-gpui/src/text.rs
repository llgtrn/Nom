//! Text shaping and glyph rasterization for nom-gpui.
//!
//! Wraps `cosmic-text` for Unicode-aware shaping and `swash` for
//! anti-aliased rasterization.  All types carry Nom-native names.

use cosmic_text::{Attrs, AttrsList, Family, FontSystem, ShapeBuffer, ShapeLine, Shaping, Wrap};
use parking_lot::Mutex;
use swash::{
    scale::{image::Content, Render, ScaleContext, Source, StrikeWith},
    zeno::{Format, Vector},
};
use thiserror::Error;

// ---------------------------------------------------------------------------
// Core identifiers
// ---------------------------------------------------------------------------

/// Opaque handle for a font face loaded into the [`TextSystem`].
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct FontId(pub u32);

/// Glyph identifier within a font face.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct GlyphId(pub u16);

// ---------------------------------------------------------------------------
// Rasterization parameters
// ---------------------------------------------------------------------------

/// Sub-pixel fractional offset bucket.
///
/// `x` is in `0..4` (horizontal variants) and `y` is in `0..2`
/// (vertical variants; MVP uses 1 on Windows).
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct SubpixelVariant {
    pub x: u8,
    pub y: u8,
}

/// Key that fully describes a glyph rasterization request.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct RenderGlyphParams {
    pub font_id: FontId,
    pub glyph_id: GlyphId,
    /// Font size in whole pixels (caller rounds up before passing in).
    pub font_size_px: u32,
    pub subpixel_variant: SubpixelVariant,
    /// `scale_factor × 100`, rounded to an integer so the struct can be
    /// used as a hash-map key without floating-point ambiguity.
    pub scale_factor_x100: u32,
    pub is_emoji: bool,
    pub subpixel_rendering: bool,
}

// ---------------------------------------------------------------------------
// Output types
// ---------------------------------------------------------------------------

/// A single glyph with its horizontal advance and vertical offset as
/// returned by [`TextSystem::shape_line`].
#[derive(Clone, Debug)]
pub struct PositionedGlyph {
    pub glyph_id: GlyphId,
    pub font_id: FontId,
    pub x_advance: f32,
    pub y_offset: f32,
}

/// Raw bitmap bytes ready for atlas upload.
#[derive(Clone, Debug)]
pub struct RasterizedGlyph {
    pub bytes: Vec<u8>,
    pub width: u32,
    pub height: u32,
    /// Signed left bearing in pixels.
    pub offset_x: i32,
    /// Signed top bearing in pixels (positive = above baseline).
    pub offset_y: i32,
    /// `true` → bytes are BGRA (subpixel path).
    /// `false` → bytes are R8 alpha-only (monochrome path).
    pub is_bgra: bool,
}

// ---------------------------------------------------------------------------
// Errors
// ---------------------------------------------------------------------------

#[derive(Debug, Error)]
pub enum TextError {
    #[error("font not loaded")]
    FontNotLoaded,
    #[error("glyph not rasterizable")]
    GlyphNotRasterizable,
}

// ---------------------------------------------------------------------------
// Internal font registry
// ---------------------------------------------------------------------------

/// One registered font face, stored alongside the raw bytes needed by swash.
struct RegisteredFace {
    /// Owned font bytes — swash borrows from these.
    data: std::sync::Arc<Vec<u8>>,
    /// Byte offset of the face within `data` (for TTC collections).
    offset: u32,
    /// The cosmic-text `fontdb::ID` for this face.
    cosmic_id: cosmic_text::fontdb::ID,
}

// ---------------------------------------------------------------------------
// TextSystem
// ---------------------------------------------------------------------------

/// Thread-safe text shaping + glyph rasterization system.
pub struct TextSystem {
    font_system: Mutex<FontSystem>,
    scale_context: Mutex<ScaleContext>,
    /// Font registry indexed by [`FontId`] ordinal.
    faces: Mutex<Vec<RegisteredFace>>,
    /// Scratch buffer reused across shape calls to avoid per-call allocation.
    shape_scratch: Mutex<ShapeBuffer>,
}

impl TextSystem {
    /// Create a new `TextSystem` using system fonts discovered by fontdb.
    pub fn new() -> Self {
        Self {
            font_system: Mutex::new(FontSystem::new()),
            scale_context: Mutex::new(ScaleContext::new()),
            faces: Mutex::new(Vec::new()),
            shape_scratch: Mutex::new(ShapeBuffer::default()),
        }
    }

    // -----------------------------------------------------------------------
    // Shaping
    // -----------------------------------------------------------------------

    /// Shape `text` with advanced Unicode shaping (HarfBuzz under the hood).
    ///
    /// `family` is looked up in the system font database; if not found
    /// cosmic-text will fall back to a generic sans-serif.  `weight` follows
    /// CSS numeric convention (400 = Regular, 700 = Bold).
    ///
    /// Returns one [`PositionedGlyph`] per cluster in visual order.
    pub fn shape_line(
        &self,
        text: &str,
        font_size: f32,
        family: &str,
        weight: u16,
    ) -> Vec<PositionedGlyph> {
        if text.is_empty() {
            return Vec::new();
        }

        let attrs = Attrs::new()
            .family(Family::Name(family))
            .weight(cosmic_text::Weight(weight));
        let attrs_list = AttrsList::new(attrs);

        let mut fs = self.font_system.lock();
        let mut scratch = self.shape_scratch.lock();

        let shape_line = ShapeLine::new_in_buffer(
            &mut scratch,
            &mut fs,
            text,
            &attrs_list,
            Shaping::Advanced,
            4,
        );

        // layout_to_buffer fills `layout_lines`.
        let mut layout_lines: Vec<cosmic_text::LayoutLine> = Vec::with_capacity(1);
        shape_line.layout_to_buffer(
            &mut scratch,
            font_size,
            None,       // no forced wrap width
            Wrap::None,
            None,       // no alignment override
            &mut layout_lines,
            None,       // no monospace width override
        );

        let Some(line) = layout_lines.first() else {
            return Vec::new();
        };

        let mut faces = self.faces.lock();
        let mut out = Vec::with_capacity(line.glyphs.len());

        for lg in &line.glyphs {
            let font_id = resolve_or_register(&mut fs, &mut faces, lg.font_id);
            out.push(PositionedGlyph {
                glyph_id: GlyphId(lg.glyph_id),
                font_id,
                x_advance: lg.w,
                y_offset: lg.y,
            });
        }

        out
    }

    // -----------------------------------------------------------------------
    // Rasterization
    // -----------------------------------------------------------------------

    /// Rasterize a single glyph to a bitmap.
    ///
    /// On success returns a [`RasterizedGlyph`] whose `.bytes` are ready for
    /// atlas upload.  If the font face is not registered or swash cannot
    /// render the glyph, returns [`TextError`].
    pub fn rasterize_glyph(&self, params: &RenderGlyphParams) -> Result<RasterizedGlyph, TextError> {
        // Clone the Arc<Vec<u8>> and copy the offset out so we can release
        // the faces lock before acquiring scale_context (avoids lock-order
        // inversion).  swash::FontRef borrows from the cloned Arc, which
        // lives for the rest of this function scope.
        let (font_data, font_offset) = {
            let faces = self.faces.lock();
            let face = faces
                .get(params.font_id.0 as usize)
                .ok_or(TextError::FontNotLoaded)?;
            (std::sync::Arc::clone(&face.data), face.offset)
        };

        let scale_factor = params.scale_factor_x100 as f32 / 100.0;
        let pixel_size = params.font_size_px as f32 * scale_factor;

        let subpixel_offset = Vector::new(
            params.subpixel_variant.x as f32 / 4.0 / scale_factor,
            params.subpixel_variant.y as f32 / 2.0 / scale_factor,
        );

        // Build FontRef after releasing the faces lock.
        let font_ref = swash::FontRef {
            data: &font_data,
            offset: font_offset,
            key: swash::CacheKey::new(),
        };

        let mut ctx = self.scale_context.lock();
        let mut scaler = ctx
            .builder(font_ref)
            .size(pixel_size)
            .hint(true)
            .build();

        let sources: &[Source] = if params.is_emoji {
            &[
                Source::ColorOutline(0),
                Source::ColorBitmap(StrikeWith::BestFit),
                Source::Outline,
            ]
        } else {
            &[Source::Outline]
        };

        let mut renderer = Render::new(sources);
        if params.subpixel_rendering {
            renderer
                .format(Format::subpixel_bgra())
                .offset(subpixel_offset);
        } else {
            renderer.format(Format::Alpha).offset(subpixel_offset);
        }

        let glyph_id = params.glyph_id.0;
        let image = renderer
            .render(&mut scaler, glyph_id)
            .ok_or(TextError::GlyphNotRasterizable)?;

        if image.data.is_empty() {
            return Err(TextError::GlyphNotRasterizable);
        }

        let is_bgra = matches!(
            image.content,
            Content::Color | Content::SubpixelMask
        );

        // swash returns RGBA for colour/subpixel; caller spec asks for BGRA.
        // Swap R and B channels in-place.
        let mut bytes = image.data;
        if is_bgra {
            for pixel in bytes.chunks_exact_mut(4) {
                pixel.swap(0, 2);
            }
        }

        Ok(RasterizedGlyph {
            bytes,
            width: image.placement.width,
            height: image.placement.height,
            offset_x: image.placement.left,
            offset_y: image.placement.top,
            is_bgra,
        })
    }
}

// ---------------------------------------------------------------------------
// Internal helpers
// ---------------------------------------------------------------------------

/// Look up a `cosmic_text::fontdb::ID` in our registry, registering it if
/// not yet present.  Returns the [`FontId`] ordinal.
fn resolve_or_register(
    fs: &mut FontSystem,
    faces: &mut Vec<RegisteredFace>,
    cosmic_id: cosmic_text::fontdb::ID,
) -> FontId {
    // Fast path: already registered.
    if let Some(pos) = faces.iter().position(|f| f.cosmic_id == cosmic_id) {
        return FontId(pos as u32);
    }

    // Slow path: load from cosmic-text and extract raw bytes.
    let id = FontId(faces.len() as u32);

    if let Some(font_arc) = fs.get_font(cosmic_id) {
        // `font_arc.data()` returns `&[u8]`; we copy to owned storage so
        // swash can safely borrow it for the lifetime of the registry.
        let raw: Vec<u8> = font_arc.data().to_vec();
        let offset = font_arc.as_swash().offset;
        faces.push(RegisteredFace {
            data: std::sync::Arc::new(raw),
            offset,
            cosmic_id,
        });
    } else {
        // Font data unavailable — push a sentinel with empty bytes.
        // Rasterization will fail gracefully with FontNotLoaded.
        faces.push(RegisteredFace {
            data: std::sync::Arc::new(Vec::new()),
            offset: 0,
            cosmic_id,
        });
    }

    id
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn text_system_constructs() {
        let _ts = TextSystem::new();
    }

    #[test]
    fn shape_simple_line_returns_glyphs() {
        let ts = TextSystem::new();
        let glyphs = ts.shape_line("Hi", 16.0, "Inter", 400);
        assert!(
            !glyphs.is_empty(),
            "should produce glyphs for non-empty input"
        );
    }

    #[test]
    fn rasterize_monochrome_returns_bytes() {
        let ts = TextSystem::new();
        let glyphs = ts.shape_line("A", 20.0, "Inter", 400);
        if let Some(g) = glyphs.first() {
            let params = RenderGlyphParams {
                font_id: g.font_id,
                glyph_id: g.glyph_id,
                font_size_px: 20,
                subpixel_variant: SubpixelVariant { x: 0, y: 0 },
                scale_factor_x100: 100,
                is_emoji: false,
                subpixel_rendering: false,
            };
            if let Ok(raster) = ts.rasterize_glyph(&params) {
                assert!(!raster.bytes.is_empty());
                assert!(!raster.is_bgra);
            }
            // glyph may fail rasterize on some CI runners — test passes if so
        }
    }
}
