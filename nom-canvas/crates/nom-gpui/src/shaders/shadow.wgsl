// shadow.wgsl — rounded-rect drop shadow with Gaussian-falloff approximation for nom-gpui
// Renders a shadow behind a rounded rectangle by computing the SDF distance
// and applying a smooth Gaussian-approximated falloff driven by blur_radius.
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

// ---- shadow types ------------------------------------------------------- //

// Per-instance data for a rounded-rect shadow.
// bounds:       {origin.xy, size.xy} of the shadow quad (already expanded by blur_radius).
// clip_bounds:  scissor rectangle in screen pixels.
// corner_radii: (top-left, top-right, bottom-right, bottom-left) in pixels.
// color:        premultiplied RGBA shadow tint (typically opaque black with low alpha).
// blur_radius:  1-sigma Gaussian radius in pixels; 0 = hard-edge shadow.
struct ShadowInstance {
    bounds:       Rect,
    clip_bounds:  Rect,
    corner_radii: vec4<f32>,
    color:        vec4<f32>,
    blur_radius:  f32,
    _pad:         vec3<f32>,
}

struct ShadowVaryings {
    @builtin(position)              position:  vec4<f32>,
    @location(0) @interpolate(flat) instance_id: u32,
    @location(1)                    clip_dist: vec4<f32>,
}

// ---- bindings ----------------------------------------------------------- //

@group(0) @binding(0) var<uniform>       globals:   RenderParams;
@group(1) @binding(0) var<storage, read> instances: array<ShadowInstance>;

// ---- SDF helpers -------------------------------------------------------- //

fn pick_radius(center_to_point: vec2<f32>, radii: vec4<f32>) -> f32 {
    // radii: (tl, tr, br, bl)
    if center_to_point.x < 0.0 {
        return select(radii.w, radii.x, center_to_point.y < 0.0);
    } else {
        return select(radii.z, radii.y, center_to_point.y < 0.0);
    }
}

// Signed distance to the outside edge of a rounded rectangle.
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
    return length(max(q, vec2<f32>(0.0))) + min(max(q.x, q.y), 0.0) - radius;
}

// Gaussian-falloff approximation using a rational-polynomial erfc substitute.
// The standard erf approximation via:  erf(x) ≈ 1 − (a1·t + a2·t² + a3·t³)·e^(−x²)
// where t = 1/(1 + 0.47047·|x|)  (Abramowitz & Stegun 7.1.26, max error 2.5e-5).
// We use erfc(x/σ√2)/2 as the complementary survival function for the shadow tail.
fn gauss_shadow_coverage(sdf_dist: f32, sigma: f32) -> f32 {
    // Hard-edge fallback when blur is negligible.
    if sigma < 0.001 {
        return select(1.0, 0.0, sdf_dist > 0.0);
    }

    let x = sdf_dist / (sigma * 1.4142135623730951); // divide by sigma * sqrt(2)

    // Abramowitz & Stegun erfc approximation (A&S 7.1.26).
    let abs_x = abs(x);
    let t      = 1.0 / (1.0 + 0.47047 * abs_x);
    let poly   = t * (0.3480242 + t * (-0.0958798 + t * 0.7478556));
    let erfc_pos = poly * exp(-abs_x * abs_x);

    // erfc is symmetric: erfc(-x) = 2 - erfc(x).
    let erfc_x = select(2.0 - erfc_pos, erfc_pos, x >= 0.0);

    // Shadow coverage = erfc(x) / 2 → fades from 1 (deep inside) to 0 (far outside).
    return saturate(erfc_x * 0.5);
}

// ---- vertex stage ------------------------------------------------------- //

@vertex
fn vs_shadow(
    @builtin(vertex_index)   vid: u32,
    @builtin(instance_index) iid: u32,
) -> ShadowVaryings {
    let inst = instances[iid];
    let pos  = rect_position(vid, inst.bounds);

    var out: ShadowVaryings;
    out.position    = to_ndc(pos, globals.viewport_size);
    out.instance_id = iid;
    out.clip_dist   = clip_distances_rect(pos, inst.clip_bounds);
    return out;
}

// ---- fragment stage ----------------------------------------------------- //

@fragment
fn fs_shadow(v: ShadowVaryings) -> @location(0) vec4<f32> {
    if any(v.clip_dist < vec4<f32>(0.0)) {
        return vec4<f32>(0.0);
    }

    let inst     = instances[v.instance_id];
    let pixel    = v.position.xy;

    // SDF distance to the inner (content) rounded rect, before blur expansion.
    // The blur quad is expanded by blur_radius on each side, so we contract back.
    let content_bounds = Rect(
        inst.bounds.origin + vec2<f32>(inst.blur_radius),
        inst.bounds.size   - vec2<f32>(inst.blur_radius * 2.0),
    );
    let dist = rounded_rect_sdf(pixel, content_bounds, inst.corner_radii);

    // Shadow coverage falls off as a Gaussian from the SDF boundary outward.
    let coverage = gauss_shadow_coverage(dist, inst.blur_radius);
    if coverage <= 0.0 {
        return vec4<f32>(0.0);
    }

    // Apply premultiplied alpha output using the instance color's alpha.
    let alpha    = inst.color.a * coverage;
    let rgb_mult = select(1.0, alpha, globals.premultiplied_alpha != 0u);
    return vec4<f32>(inst.color.rgb * rgb_mult, alpha);
}
