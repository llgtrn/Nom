// mono_sprite.wgsl — monochrome atlas sprite (text glyph) renderer for nom-gpui
// Samples R channel from an R8Unorm atlas and tints by a flat color.
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

// ---- mono sprite types -------------------------------------------------- //

// Per-instance data for a monochrome atlas sprite.
// uv_min / uv_max are normalised UV coordinates into the R8Unorm atlas texture.
// transform_row0: (rotation_scale[0][0], rotation_scale[0][1], translation.x, _pad)
// transform_row1: (rotation_scale[1][0], rotation_scale[1][1], translation.y, _pad)
// The 2×2 rotation/scale matrix and translation encode a 2D affine transform applied
// around the sprite center (center-based semantics).
struct MonoSpriteInstance {
    bounds:          Rect,
    clip_bounds:     Rect,
    color:           vec4<f32>,
    uv_min:          vec2<f32>,
    uv_max:          vec2<f32>,
    transform_row0:  vec4<f32>,
    transform_row1:  vec4<f32>,
}

struct MonoSpriteVaryings {
    @builtin(position)              position:    vec4<f32>,
    @location(0)                    atlas_uv:    vec2<f32>,
    @location(1) @interpolate(flat) color:       vec4<f32>,
    @location(2)                    clip_dist:   vec4<f32>,
}

// ---- bindings ------------------------------------------------------------ //

@group(0) @binding(0) var<uniform>       globals:   RenderParams;
@group(1) @binding(0) var<storage, read> instances: array<MonoSpriteInstance>;
@group(1) @binding(1) var atlas_tex: texture_2d<f32>;
@group(1) @binding(2) var atlas_smp: sampler;

// ---- vertex stage -------------------------------------------------------- //

@vertex
fn vs_mono_sprite(
    @builtin(vertex_index)   vid: u32,
    @builtin(instance_index) iid: u32,
) -> MonoSpriteVaryings {
    let inst  = instances[iid];
    let uv    = unit_vertex(vid);
    let pos_local = uv * inst.bounds.size + inst.bounds.origin;

    // Apply the 2D affine transform around the sprite center (center-based semantics).
    // rotation_scale rows are packed in transform_row0.xy / transform_row1.xy,
    // translation is in transform_row0.z / transform_row1.z.
    let center = inst.bounds.origin + inst.bounds.size * 0.5;
    let rs_r0  = inst.transform_row0.xy;
    let rs_r1  = inst.transform_row1.xy;
    let transl = vec2<f32>(inst.transform_row0.z, inst.transform_row1.z);
    let local  = pos_local - center;
    let rotated = vec2<f32>(dot(rs_r0, local), dot(rs_r1, local));
    let pos    = rotated + center + transl;

    // Interpolate UV coordinates from uv_min to uv_max across the quad.
    let atlas_uv = inst.uv_min + uv * (inst.uv_max - inst.uv_min);

    var out: MonoSpriteVaryings;
    out.position  = to_ndc(pos, globals.viewport_size);
    out.atlas_uv  = atlas_uv;
    out.color     = inst.color;
    out.clip_dist = clip_distances_rect(pos, inst.clip_bounds);
    return out;
}

// ---- fragment stage ------------------------------------------------------ //

@fragment
fn fs_mono_sprite(v: MonoSpriteVaryings) -> @location(0) vec4<f32> {
    // Use derivatives before clip discard so derivative helpers stay valid.
    let coverage = textureSample(atlas_tex, atlas_smp, v.atlas_uv).r;

    if any(v.clip_dist < vec4<f32>(0.0)) {
        return vec4<f32>(0.0);
    }

    // Premultiplied output: rgb = color.rgb * color.a * coverage, a = color.a * coverage
    let alpha = v.color.a * coverage;
    let rgb_mult = select(1.0, alpha, globals.premultiplied_alpha != 0u);
    return vec4<f32>(v.color.rgb * rgb_mult, alpha);
}
