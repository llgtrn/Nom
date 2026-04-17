// common.wgsl — shared types and helpers for nom-gpui shaders
// Used by duplication (each shader embeds a copy) to keep loading simple.

// Viewport-level uniform, bound at group(0) binding(0) in every pipeline.
struct RenderParams {
    viewport_size: vec2<f32>,
    premultiplied_alpha: u32,
    _padding: u32,
}

// Axis-aligned rectangle in screen pixels.
struct Rect {
    origin: vec2<f32>,
    size: vec2<f32>,
}

// Convert a screen-space position to NDC clip coordinates.
// Y is flipped because screen-space Y increases downward.
fn to_ndc(pos: vec2<f32>, vp: vec2<f32>) -> vec4<f32> {
    let ndc = pos / vp * vec2<f32>(2.0, -2.0) + vec2<f32>(-1.0, 1.0);
    return vec4<f32>(ndc, 0.0, 1.0);
}

// Returns the unit-quad vertex for a triangle-strip quad (vertices 0-3).
//   vertex 0 → (0,0)   vertex 1 → (1,0)
//   vertex 2 → (0,1)   vertex 3 → (1,1)
fn unit_vertex(vertex_id: u32) -> vec2<f32> {
    return vec2<f32>(f32(vertex_id & 1u), 0.5 * f32(vertex_id & 2u));
}

// Maps a unit-quad vertex into the pixel rectangle described by `r`.
fn rect_position(vertex_id: u32, r: Rect) -> vec2<f32> {
    return unit_vertex(vertex_id) * r.size + r.origin;
}

// Clip-distance helper: returns (dist_left, dist_right, dist_top, dist_bottom).
// Positive when the pixel is inside the corresponding edge of `clip`.
fn clip_distances(pos: vec2<f32>, clip: Rect) -> vec4<f32> {
    let tl = pos - clip.origin;
    let br = clip.origin + clip.size - pos;
    return vec4<f32>(tl.x, br.x, tl.y, br.y);
}

// Emit premultiplied-alpha color given a linear RGBA + coverage factor.
fn premul_blend(color: vec4<f32>, params: RenderParams, coverage: f32) -> vec4<f32> {
    let a = color.a * coverage;
    let multiplier = select(1.0, a, params.premultiplied_alpha != 0u);
    return vec4<f32>(color.rgb * multiplier, a);
}
