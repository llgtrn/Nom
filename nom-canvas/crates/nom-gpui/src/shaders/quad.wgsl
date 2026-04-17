// quad.wgsl — rounded rectangle with per-edge border for nom-gpui
// Embeds the common header inline so the pipeline can load this file stand-alone.

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

// ---- quad types ---------------------------------------------------------- //

// Per-instance data for a rounded-rectangle quad.
// corner_radii: (top-left, top-right, bottom-right, bottom-left) radii in px.
// border_widths: (top, right, bottom, left) border widths in px.
struct QuadInstance {
    bounds:        Rect,
    clip_bounds:   Rect,
    corner_radii:  vec4<f32>,
    background:    vec4<f32>,
    border_color:  vec4<f32>,
    border_widths: vec4<f32>,
}

struct QuadVaryings {
    @builtin(position)            position:       vec4<f32>,
    @location(0) @interpolate(flat) instance_id:  u32,
    @location(1)                    clip_dist:    vec4<f32>,
}

// ---- bindings ------------------------------------------------------------ //

@group(0) @binding(0) var<uniform>       globals:   RenderParams;
@group(1) @binding(0) var<storage, read> instances: array<QuadInstance>;

// ---- SDF helpers --------------------------------------------------------- //

// Corner radius for the quadrant containing `center_to_point`.
fn pick_radius(center_to_point: vec2<f32>, radii: vec4<f32>) -> f32 {
    // radii: (tl, tr, br, bl)
    if center_to_point.x < 0.0 {
        return select(radii.w, radii.x, center_to_point.y < 0.0);
    } else {
        return select(radii.z, radii.y, center_to_point.y < 0.0);
    }
}

// Signed distance to the outside edge of the rounded rectangle.
// Positive = outside, negative = inside.
fn rounded_rect_sdf(pixel: vec2<f32>, r: Rect, radii: vec4<f32>) -> f32 {
    let half_size       = r.size * 0.5;
    let center          = r.origin + half_size;
    let center_to_point = pixel - center;
    let radius          = pick_radius(center_to_point, radii);
    let corner_to_point = abs(center_to_point) - half_size;
    let q               = corner_to_point + radius;

    if radius == 0.0 {
        return max(corner_to_point.x, corner_to_point.y);
    }
    // Standard 2-D rounded-box SDF
    return length(max(q, vec2<f32>(0.0))) + min(max(q.x, q.y), 0.0) - radius;
}

// ---- vertex stage -------------------------------------------------------- //

@vertex
fn vs_quad(
    @builtin(vertex_index)   vid: u32,
    @builtin(instance_index) iid: u32,
) -> QuadVaryings {
    let inst = instances[iid];
    let pos  = rect_position(vid, inst.bounds);

    var out: QuadVaryings;
    out.position    = to_ndc(pos, globals.viewport_size);
    out.instance_id = iid;
    out.clip_dist   = clip_distances_rect(pos, inst.clip_bounds);
    return out;
}

// ---- fragment stage ------------------------------------------------------ //

@fragment
fn fs_quad(v: QuadVaryings) -> @location(0) vec4<f32> {
    // Discard pixels outside the clip rectangle.
    if any(v.clip_dist < vec4<f32>(0.0)) {
        return vec4<f32>(0.0);
    }

    let inst        = instances[v.instance_id];
    let pixel       = v.position.xy;
    let aa          = 0.5; // half-pixel AA threshold

    // Outer SDF — positive outside the rounded rect.
    let outer_dist  = rounded_rect_sdf(pixel, inst.bounds, inst.corner_radii);
    // Discard pixels fully outside the outer edge.
    if outer_dist > aa {
        return vec4<f32>(0.0);
    }

    // Choose border widths for this quadrant.
    let half_size       = inst.bounds.size * 0.5;
    let center          = inst.bounds.origin + half_size;
    let center_to_point = pixel - center;
    let bw = vec2<f32>(
        select(inst.border_widths.y, inst.border_widths.w, center_to_point.x < 0.0),
        select(inst.border_widths.z, inst.border_widths.x, center_to_point.y < 0.0),
    );

    // Inner rect (background area) = bounds inset by border widths.
    let inner_bounds = Rect(
        inst.bounds.origin + vec2<f32>(inst.border_widths.w, inst.border_widths.x),
        inst.bounds.size   - vec2<f32>(inst.border_widths.w + inst.border_widths.y,
                                       inst.border_widths.x + inst.border_widths.z),
    );
    // Shrink corner radii by border width so inner corners stay smooth.
    let inner_radii = max(inst.corner_radii - vec4<f32>(max(bw.x, bw.y)), vec4<f32>(0.0));
    let inner_dist  = rounded_rect_sdf(pixel, inner_bounds, inner_radii);

    // Coverage for outer edge (AA).
    let outer_coverage = saturate(aa - outer_dist);

    // Determine fill vs border blend.
    // inner_dist < -aa → fully inside inner → pure background
    // inner_dist >  aa → fully in border region → pure border
    let border_blend = saturate((inner_dist + aa) / (2.0 * aa));

    var color = mix(inst.background, inst.border_color, border_blend);

    // Apply premultiplied alpha output.
    let alpha = color.a * outer_coverage;
    let rgb_mult = select(1.0, alpha, globals.premultiplied_alpha != 0u);
    return vec4<f32>(color.rgb * rgb_mult, alpha);
}
