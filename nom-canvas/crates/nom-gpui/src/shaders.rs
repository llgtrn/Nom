#![deny(unsafe_code)]

/// Minimal WGSL quad vertex shader stub.
pub const QUAD_VERT_WGSL: &str = r#"
@vertex
fn vs_main(@builtin(vertex_index) vi: u32) -> @builtin(position) vec4<f32> {
    return vec4<f32>(0.0, 0.0, 0.0, 1.0);
}
"#;

/// Minimal WGSL quad fragment shader stub.
pub const QUAD_FRAG_WGSL: &str = r#"
@fragment
fn fs_main() -> @location(0) vec4<f32> {
    return vec4<f32>(1.0, 0.0, 0.0, 1.0);
}
"#;

/// Minimal WGSL sprite vertex shader stub.
pub const SPRITE_VERT_WGSL: &str = r#"
@vertex
fn vs_main(@builtin(vertex_index) vi: u32) -> @builtin(position) vec4<f32> {
    return vec4<f32>(0.0, 0.0, 0.0, 1.0);
}
"#;

/// Minimal WGSL sprite fragment shader stub.
pub const SPRITE_FRAG_WGSL: &str = r#"
@fragment
fn fs_main() -> @location(0) vec4<f32> {
    return vec4<f32>(0.0, 1.0, 0.0, 1.0);
}
"#;

/// Minimal WGSL shadow vertex shader stub.
pub const SHADOW_VERT_WGSL: &str = r#"
@vertex
fn vs_main(@builtin(vertex_index) vi: u32) -> @builtin(position) vec4<f32> {
    return vec4<f32>(0.0, 0.0, 0.0, 1.0);
}
"#;

/// Minimal WGSL shadow fragment shader stub.
pub const SHADOW_FRAG_WGSL: &str = r#"
@fragment
fn fs_main() -> @location(0) vec4<f32> {
    return vec4<f32>(0.0, 0.0, 0.0, 0.5);
}
"#;

/// Minimal WGSL path vertex shader stub.
pub const PATH_VERT_WGSL: &str = r#"
@vertex
fn vs_main(@builtin(vertex_index) vi: u32) -> @builtin(position) vec4<f32> {
    return vec4<f32>(0.0, 0.0, 0.0, 1.0);
}
"#;

/// Minimal WGSL path fragment shader stub.
pub const PATH_FRAG_WGSL: &str = r#"
@fragment
fn fs_main() -> @location(0) vec4<f32> {
    return vec4<f32>(0.0, 0.0, 1.0, 1.0);
}
"#;

/// Minimal WGSL underline vertex shader stub.
pub const UNDERLINE_VERT_WGSL: &str = r#"
@vertex
fn vs_main(@builtin(vertex_index) vi: u32) -> @builtin(position) vec4<f32> {
    return vec4<f32>(0.0, 0.0, 0.0, 1.0);
}
"#;

/// Minimal WGSL underline fragment shader stub.
pub const UNDERLINE_FRAG_WGSL: &str = r#"
@fragment
fn fs_main() -> @location(0) vec4<f32> {
    return vec4<f32>(1.0, 1.0, 0.0, 1.0);
}
"#;

/// Number of render pipelines defined by the shader set.
pub const PIPELINE_COUNT: usize = 8;

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn quad_vert_nonempty() {
        assert!(QUAD_VERT_WGSL.len() > 0);
    }

    #[test]
    fn quad_frag_nonempty() {
        assert!(QUAD_FRAG_WGSL.len() > 0);
    }

    #[test]
    fn sprite_shaders_nonempty() {
        assert!(SPRITE_VERT_WGSL.len() > 0);
        assert!(SPRITE_FRAG_WGSL.len() > 0);
    }

    #[test]
    fn shadow_shaders_nonempty() {
        assert!(SHADOW_VERT_WGSL.len() > 0);
        assert!(SHADOW_FRAG_WGSL.len() > 0);
    }

    #[test]
    fn path_shaders_nonempty() {
        assert!(PATH_VERT_WGSL.len() > 0);
        assert!(PATH_FRAG_WGSL.len() > 0);
    }

    #[test]
    fn underline_shaders_nonempty() {
        assert!(UNDERLINE_VERT_WGSL.len() > 0);
        assert!(UNDERLINE_FRAG_WGSL.len() > 0);
    }

    #[test]
    fn pipeline_count_is_8() {
        assert_eq!(PIPELINE_COUNT, 8);
    }

    #[test]
    fn all_shaders_contain_fn() {
        let shaders = [
            QUAD_VERT_WGSL,
            QUAD_FRAG_WGSL,
            SPRITE_VERT_WGSL,
            SPRITE_FRAG_WGSL,
            SHADOW_VERT_WGSL,
            SHADOW_FRAG_WGSL,
            PATH_VERT_WGSL,
            PATH_FRAG_WGSL,
            UNDERLINE_VERT_WGSL,
            UNDERLINE_FRAG_WGSL,
        ];
        for shader in &shaders {
            assert!(
                shader.contains("@vertex") || shader.contains("@fragment"),
                "Shader missing @vertex or @fragment annotation: {shader}"
            );
        }
    }
}
