// subpixel_sprite.wgsl — LCD subpixel-positioned text for nom-gpui
//
// Renders atlas glyphs where the atlas encodes per-subpixel (R/G/B) coverage
// values in the tile's RGB channels.  The fragment averages the three subpixel
// coverages into a single luminance alpha so the pipeline can be compiled
// under wgpu 22 / naga 22, which do not yet support the @blend_src dual-source
// blending WGSL extension.
//
// When wgpu gains @blend_src support (wgpu ≥ 23 with the enable directive)
// this shader should be upgraded to emit two blend sources for true per-channel
// subpixel blending:
//   @location(0) @blend_src(0) foreground: vec4<f32>,
//   @location(0) @blend_src(1) alpha:      vec4<f32>,
// and the pipeline blend state updated to Src1 / OneMinusSrc1.
//
// For now the pipeline is created only when ctx.dual_source_blending is true
// (matching the hardware capability), but uses standard premultiplied-alpha
// blending.  The subpixel coverage information is preserved for future use.
//
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

// ---- subpixel sprite types ---------------------------------------------- //

// Per-instance data for a subpixel-positioned atlas glyph.
// Matches MonoSpriteInstance layout so the same CPU-side buffer type is reused.
// uv_min / uv_max index into an atlas where R/G/B hold per-subpixel coverage
// and A is the scalar (mean) coverage for the alpha channel.
struct SubpixelSpriteInstance {
    bounds:      Rect,
    clip_bounds: Rect,
    color:       vec4<f32>,
    uv_min:      vec2<f32>,
    uv_max:      vec2<f32>,
}

struct SubpixelSpriteVaryings {
    @builtin(position)              position:    vec4<f32>,
    @location(0)                    atlas_uv:    vec2<f32>,
    @location(1) @interpolate(flat) color:       vec4<f32>,
    @location(2)                    clip_dist:   vec4<f32>,
}

// ---- bindings ----------------------------------------------------------- //

@group(0) @binding(0) var<uniform>       globals:   RenderParams;
@group(1) @binding(0) var<storage, read> instances: array<SubpixelSpriteInstance>;
@group(1) @binding(1) var atlas_tex: texture_2d<f32>;
@group(1) @binding(2) var atlas_smp: sampler;

// ---- vertex stage ------------------------------------------------------- //

@vertex
fn vs_subpixel_sprite(
    @builtin(vertex_index)   vid: u32,
    @builtin(instance_index) iid: u32,
) -> SubpixelSpriteVaryings {
    let inst     = instances[iid];
    let uv       = unit_vertex(vid);
    let pos      = uv * inst.bounds.size + inst.bounds.origin;
    let atlas_uv = inst.uv_min + uv * (inst.uv_max - inst.uv_min);

    var out: SubpixelSpriteVaryings;
    out.position  = to_ndc(pos, globals.viewport_size);
    out.atlas_uv  = atlas_uv;
    out.color     = inst.color;
    out.clip_dist = clip_distances_rect(pos, inst.clip_bounds);
    return out;
}

// ---- fragment stage ----------------------------------------------------- //

@fragment
fn fs_subpixel_sprite(v: SubpixelSpriteVaryings) -> @location(0) vec4<f32> {
    // Sample before clip discard to keep derivative helpers valid.
    let tile = textureSample(atlas_tex, atlas_smp, v.atlas_uv);

    if any(v.clip_dist < vec4<f32>(0.0)) {
        return vec4<f32>(0.0);
    }

    // Average the three subpixel (R/G/B) coverages into a single alpha value.
    // When wgpu gains @blend_src support this should be split into two outputs
    // for true per-channel LCD subpixel blending.
    let sub_coverage = dot(tile.rgb, vec3<f32>(1.0 / 3.0));
    let alpha = v.color.a * sub_coverage;

    // Premultiplied-alpha output (standard blend path on wgpu 22).
    let rgb_mult = select(1.0, alpha, globals.premultiplied_alpha != 0u);
    return vec4<f32>(v.color.rgb * rgb_mult, alpha);
}
