// shaders/mod.rs — embed WGSL shader sources as compile-time string constants.
// Each constant is the full WGSL text for the corresponding pipeline stage.
// Loaded via include_str! so the strings are baked into the binary; the actual
// WGSL parse happens at runtime when wgpu compiles the pipeline.

pub const COMMON_SHADER: &str = include_str!("common.wgsl");
pub const QUAD_SHADER: &str = include_str!("quad.wgsl");
pub const MONO_SPRITE_SHADER: &str = include_str!("mono_sprite.wgsl");
pub const UNDERLINE_SHADER: &str = include_str!("underline.wgsl");

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shaders_parse_via_naga() {
        use naga::front::wgsl;
        let files = [QUAD_SHADER, MONO_SPRITE_SHADER, UNDERLINE_SHADER];
        for src in files {
            wgsl::parse_str(src).expect("wgsl parse");
        }
    }
}
