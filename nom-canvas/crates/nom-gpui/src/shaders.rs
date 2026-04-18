#![deny(unsafe_code)]

/// Real WGSL quad vertex+fragment shader.
///
/// Uses instance attributes (QuadIn) for per-quad position, size, and color.
/// A global uniform provides the viewport dimensions for pixel→clip transform.
pub const QUAD_VERT_WGSL: &str = r#"
struct GlobalUniforms {
    viewport: vec4<f32>,  // x, y, width, height
}
@group(0) @binding(0) var<uniform> globals: GlobalUniforms;

struct QuadIn {
    @location(0) pos_size: vec4<f32>,         // x, y, w, h
    @location(1) color: vec4<f32>,             // rgba
    @location(2) border_color: vec4<f32>,      // rgba
    @location(3) corner_radius: vec4<f32>,     // tl, tr, br, bl
    @location(4) border_thickness: vec4<f32>,  // thickness, reserved...
}

struct VertOut {
    @builtin(position) clip_pos: vec4<f32>,
    @location(0) color: vec4<f32>,
}

@vertex
fn vs_main(@builtin(vertex_index) vi: u32, quad: QuadIn) -> VertOut {
    // 6 verts for 2 triangles forming a quad
    let corners = array<vec2<f32>, 6>(
        vec2(0.0, 0.0), vec2(1.0, 0.0), vec2(1.0, 1.0),
        vec2(0.0, 0.0), vec2(1.0, 1.0), vec2(0.0, 1.0),
    );
    let local = corners[vi];
    let world_x = quad.pos_size.x + local.x * quad.pos_size.z;
    let world_y = quad.pos_size.y + local.y * quad.pos_size.w;
    let vw = globals.viewport.z;
    let vh = globals.viewport.w;
    let clip_x = (world_x / vw) * 2.0 - 1.0;
    let clip_y = 1.0 - (world_y / vh) * 2.0;
    var out: VertOut;
    out.clip_pos = vec4<f32>(clip_x, clip_y, 0.0, 1.0);
    out.color = quad.color;
    return out;
}

@fragment
fn fs_main(in: VertOut) -> @location(0) vec4<f32> {
    return in.color;
}
"#;

/// WGSL quad fragment shader (matches the combined quad shader entry points).
pub const QUAD_FRAG_WGSL: &str = r#"
struct VertOut {
    @builtin(position) clip_pos: vec4<f32>,
    @location(0) color: vec4<f32>,
}

@fragment
fn fs_main(in: VertOut) -> @location(0) vec4<f32> {
    return in.color;
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
        assert!(!QUAD_VERT_WGSL.is_empty());
    }

    #[test]
    fn quad_frag_nonempty() {
        assert!(!QUAD_FRAG_WGSL.is_empty());
    }

    #[test]
    fn sprite_shaders_nonempty() {
        assert!(!SPRITE_VERT_WGSL.is_empty());
        assert!(!SPRITE_FRAG_WGSL.is_empty());
    }

    #[test]
    fn shadow_shaders_nonempty() {
        assert!(!SHADOW_VERT_WGSL.is_empty());
        assert!(!SHADOW_FRAG_WGSL.is_empty());
    }

    #[test]
    fn path_shaders_nonempty() {
        assert!(!PATH_VERT_WGSL.is_empty());
        assert!(!PATH_FRAG_WGSL.is_empty());
    }

    #[test]
    fn underline_shaders_nonempty() {
        assert!(!UNDERLINE_VERT_WGSL.is_empty());
        assert!(!UNDERLINE_FRAG_WGSL.is_empty());
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

    // ---- New tests ----

    #[test]
    fn shaders_all_contain_vertex_or_fragment() {
        // The 8 pipeline shaders (4 pairs) each contain @vertex or @fragment.
        let pipeline_shaders = [
            QUAD_VERT_WGSL,
            QUAD_FRAG_WGSL,
            SPRITE_VERT_WGSL,
            SPRITE_FRAG_WGSL,
            SHADOW_VERT_WGSL,
            SHADOW_FRAG_WGSL,
            PATH_VERT_WGSL,
            PATH_FRAG_WGSL,
        ];
        for shader in &pipeline_shaders {
            assert!(
                shader.contains("@vertex") || shader.contains("@fragment"),
                "shader missing @vertex or @fragment"
            );
        }
    }

    #[test]
    fn shaders_no_empty() {
        let pipeline_shaders = [
            QUAD_VERT_WGSL,
            QUAD_FRAG_WGSL,
            SPRITE_VERT_WGSL,
            SPRITE_FRAG_WGSL,
            SHADOW_VERT_WGSL,
            SHADOW_FRAG_WGSL,
            PATH_VERT_WGSL,
            PATH_FRAG_WGSL,
        ];
        for shader in &pipeline_shaders {
            assert!(
                !shader.trim().is_empty(),
                "pipeline shader must not be empty"
            );
        }
    }

    #[test]
    fn pipeline_count_matches_pipeline_kind() {
        // PIPELINE_COUNT == 8 corresponds to 4 (quad/sprite/shadow/path) × 2 (vert+frag)
        assert_eq!(PIPELINE_COUNT, 8);
    }
}
