// shaders/mod.rs — embed WGSL shader sources as compile-time string constants.
// Each constant is the full WGSL text for the corresponding pipeline stage.
// Loaded via include_str! so the strings are baked into the binary; the actual
// WGSL parse happens at runtime when wgpu compiles the pipeline.

pub const COMMON_SHADER: &str = include_str!("common.wgsl");
pub const QUAD_SHADER: &str = include_str!("quad.wgsl");
pub const MONO_SPRITE_SHADER: &str = include_str!("mono_sprite.wgsl");
pub const UNDERLINE_SHADER: &str = include_str!("underline.wgsl");

// batch-3 additions
pub const SHADOW_SHADER: &str = include_str!("shadow.wgsl");
pub const POLY_SPRITE_SHADER: &str = include_str!("poly_sprite.wgsl");
pub const SUBPIXEL_SPRITE_SHADER: &str = include_str!("subpixel_sprite.wgsl");

// batch-4: two-pass bezier path rendering
pub const PATH_SHADER: &str = include_str!("path.wgsl");

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    /// Parse all seven shaders (batch-2 + batch-3) via naga's standalone WGSL
    /// parser.  The subpixel sprite shader uses standard premultiplied-alpha
    /// blending on wgpu 22 (no @blend_src / enable directive required) so it
    /// passes the naga parser like all other shaders.
    #[test]
    fn shaders_parse_via_naga() {
        use naga::front::wgsl;
        let files = [
            QUAD_SHADER,
            MONO_SPRITE_SHADER,
            UNDERLINE_SHADER,
            SHADOW_SHADER,
            POLY_SPRITE_SHADER,
            SUBPIXEL_SPRITE_SHADER,
            PATH_SHADER,
        ];
        for src in files {
            wgsl::parse_str(src).expect("wgsl parse");
        }
    }

    /// Confirm the subpixel sprite shader documents the wgpu 22 @blend_src
    /// upgrade path in its comments.  This is a lightweight content check so
    /// the upgrade note is never accidentally deleted.
    #[test]
    fn subpixel_sprite_shader_documents_blend_src_upgrade() {
        assert!(!SUBPIXEL_SPRITE_SHADER.is_empty());
        assert!(
            SUBPIXEL_SPRITE_SHADER.contains("@blend_src"),
            "subpixel_sprite.wgsl must document the @blend_src upgrade path in comments"
        );
    }
}
