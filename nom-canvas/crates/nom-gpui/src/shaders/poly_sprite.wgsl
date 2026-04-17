// poly_sprite.wgsl — polychrome RGBA atlas sprite renderer for nom-gpui
// Samples BGRA (stored as Bgra8Unorm) from the atlas — each tile is already
// premultiplied.  Supports an optional grayscale conversion pass.
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

// ---- poly sprite types -------------------------------------------------- //

// Per-instance data for a polychrome (full-RGBA) atlas sprite.
// uv_min / uv_max are normalised UV coordinates into the Bgra8Unorm atlas.
// grayscale: when 1u, convert the sampled RGB to luminance before output.
struct PolySpriteInstance {
    bounds:      Rect,
    clip_bounds: Rect,
    uv_min:      vec2<f32>,
    uv_max:      vec2<f32>,
    grayscale:   u32,
    _pad:        vec3<u32>,
}

struct PolySpriteVaryings {
    @builtin(position)              position:   vec4<f32>,
    @location(0)                    atlas_uv:   vec2<f32>,
    @location(1)                    clip_dist:  vec4<f32>,
    @location(2) @interpolate(flat) instance_id: u32,
}

// ---- bindings ----------------------------------------------------------- //

@group(0) @binding(0) var<uniform>       globals:   RenderParams;
@group(1) @binding(0) var<storage, read> instances: array<PolySpriteInstance>;
@group(1) @binding(1) var atlas_tex: texture_2d<f32>;
@group(1) @binding(2) var atlas_smp: sampler;

// ---- vertex stage ------------------------------------------------------- //

@vertex
fn vs_poly_sprite(
    @builtin(vertex_index)   vid: u32,
    @builtin(instance_index) iid: u32,
) -> PolySpriteVaryings {
    let inst     = instances[iid];
    let uv       = unit_vertex(vid);
    let pos      = uv * inst.bounds.size + inst.bounds.origin;
    let atlas_uv = inst.uv_min + uv * (inst.uv_max - inst.uv_min);

    var out: PolySpriteVaryings;
    out.position    = to_ndc(pos, globals.viewport_size);
    out.atlas_uv    = atlas_uv;
    out.clip_dist   = clip_distances_rect(pos, inst.clip_bounds);
    out.instance_id = iid;
    return out;
}

// ---- fragment stage ----------------------------------------------------- //

@fragment
fn fs_poly_sprite(v: PolySpriteVaryings) -> @location(0) vec4<f32> {
    // Sample before clip discard so derivative helpers remain well-defined.
    var sampled = textureSample(atlas_tex, atlas_smp, v.atlas_uv);

    if any(v.clip_dist < vec4<f32>(0.0)) {
        return vec4<f32>(0.0);
    }

    let inst = instances[v.instance_id];

    // Optional grayscale conversion: compute BT.601 luminance.
    if inst.grayscale != 0u {
        let lum = dot(sampled.rgb, vec3<f32>(0.299, 0.587, 0.114));
        sampled = vec4<f32>(lum, lum, lum, sampled.a);
    }

    // Atlas tiles are already premultiplied, so output directly.
    // If the renderer is NOT in premultiplied mode we un-multiply then re-multiply
    // with 1.0, which is a no-op — just pass through.
    return sampled;
}
