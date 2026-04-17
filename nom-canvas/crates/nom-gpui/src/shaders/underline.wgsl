// underline.wgsl — straight underline stroke renderer for nom-gpui
// Renders a flat-colored horizontal stroke within `bounds`.
// Wavy underlines are deferred to batch-3.
// Embeds the common header inline.

// ---- common header (duplicated) ---------------------------------------- //

struct RenderParams {
    viewport_size: vec2<f32>,
    premultiplied_alpha: u32,
    _padding: u32,
}

struct Rect {
    origin: vec2<f32>,
    size: vec2<f32>,
}

fn to_ndc(pos: vec2<f32>, vp: vec2<f32>) -> vec4<f32> {
    let ndc = pos / vp * vec2<f32>(2.0, -2.0) + vec2<f32>(-1.0, 1.0);
    return vec4<f32>(ndc, 0.0, 1.0);
}

fn unit_vertex(vertex_id: u32) -> vec2<f32> {
    return vec2<f32>(f32(vertex_id & 1u), 0.5 * f32(vertex_id & 2u));
}

fn rect_position(vertex_id: u32, r: Rect) -> vec2<f32> {
    return unit_vertex(vertex_id) * r.size + r.origin;
}

fn clip_distances_rect(pos: vec2<f32>, clip: Rect) -> vec4<f32> {
    let tl = pos - clip.origin;
    let br = clip.origin + clip.size - pos;
    return vec4<f32>(tl.x, br.x, tl.y, br.y);
}

// ---- underline types ---------------------------------------------------- //

// Per-instance data for a straight underline stroke.
// thickness is the stroke width in pixels; _pad aligns the struct to 16 bytes.
struct UnderlineInstance {
    bounds:      Rect,
    clip_bounds: Rect,
    color:       vec4<f32>,
    thickness:   f32,
    _pad:        vec3<f32>,
}

struct UnderlineVaryings {
    @builtin(position)              position:  vec4<f32>,
    @location(0) @interpolate(flat) color:     vec4<f32>,
    @location(1)                    clip_dist: vec4<f32>,
}

// ---- bindings ------------------------------------------------------------ //

@group(0) @binding(0) var<uniform>       globals:   RenderParams;
@group(1) @binding(0) var<storage, read> instances: array<UnderlineInstance>;

// ---- vertex stage -------------------------------------------------------- //

@vertex
fn vs_underline(
    @builtin(vertex_index)   vid: u32,
    @builtin(instance_index) iid: u32,
) -> UnderlineVaryings {
    let inst = instances[iid];
    let pos  = rect_position(vid, inst.bounds);

    var out: UnderlineVaryings;
    out.position  = to_ndc(pos, globals.viewport_size);
    out.color     = inst.color;
    out.clip_dist = clip_distances_rect(pos, inst.clip_bounds);
    return out;
}

// ---- fragment stage ------------------------------------------------------ //

@fragment
fn fs_underline(v: UnderlineVaryings) -> @location(0) vec4<f32> {
    if any(v.clip_dist < vec4<f32>(0.0)) {
        return vec4<f32>(0.0);
    }

    // Straight underline: output the flat tint color as premultiplied.
    let alpha    = v.color.a;
    let rgb_mult = select(1.0, alpha, globals.premultiplied_alpha != 0u);
    return vec4<f32>(v.color.rgb * rgb_mult, alpha);
}
