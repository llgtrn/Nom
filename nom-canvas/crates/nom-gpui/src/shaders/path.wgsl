// path.wgsl — two-pass bezier path renderer for nom-gpui
//
// Pass 1 (path_rasterization):
//   Rasterizes filled quadratic bezier paths into an intermediate MSAA texture.
//   Each triangle in the mesh carries (xy_position, st, color, clip_bounds).
//   The fragment stage computes the signed distance to the implicit quadratic
//   curve  f(s,t) = s² - t  and converts it to coverage via smoothstep.
//   Output is premultiplied RGBA into a Bgra8Unorm MSAA texture (sample_count=4).
//   The MSAA resolve to a single-sample Bgra8Unorm intermediate happens via the
//   render-pass resolve_target — renderer.rs integration handles that externally.
//
// Pass 2 (paths):
//   Composites the resolved intermediate texture back onto the surface.
//   One quad per path sprite; UV coordinates are derived from the sprite's
//   uv_min/uv_max range into the intermediate texture.
//   Uses One / OneMinusSrcAlpha blending (premultiplied coverage over frame).
//
// NOTE: renderer.rs does NOT yet dispatch these pipelines.  Integration is a
//       follow-up task.  The pipelines are compiled and available in the
//       Pipelines registry; the dispatch sequence is documented below.
//
// ── Renderer integration notes (informational — do NOT implement here) ──────
//
//   The drop-pass / begin-pass sequence for wgpu 22 requires:
//
//   1. While inside the main surface RenderPass, call `drop(pass)` when a
//      Paths batch is encountered.  wgpu 22 requires the pass to be DROPPED
//      (not just paused) before encoder commands that begin new passes can be
//      recorded — the Rust borrow checker enforces this.
//
//   2. Call `encoder.begin_render_pass(path_rasterization_pass_descriptor)`.
//      The descriptor targets the MSAA view (`path_msaa_view`) as the primary
//      color attachment and the resolved single-sample view
//      (`path_intermediate_view`) as `resolve_target`.
//      LoadOp::Clear(TRANSPARENT) is required each frame to reset coverage.
//
//   3. Set pipeline = `pipelines.path_rasterization`.
//      Bind group 0 = globals (IMPORTANT: premultiplied_alpha MUST be 0 for
//      the intermediate pass — Zed sets a separate path_globals_buffer with
//      premultiplied_alpha=0 for this reason).
//      Bind group 1 = path vertex storage buffer.
//      Draw 0..vertex_count, instance_count=1.
//
//   4. Drop the path rasterization pass.  wgpu automatically resolves the MSAA
//      texture into `path_intermediate_view` when the pass ends.
//
//   5. Resume the main surface pass with `begin_render_pass(…LoadOp::Load…)`
//      so previously drawn quads/shadows/etc. are preserved.
//
//   6. Set pipeline = `pipelines.paths`.
//      Bind group 1 = sprite buffer + intermediate texture view + sampler.
//      Draw 4 vertices (TriangleStrip unit quad), instance_count = sprite_count.
//
//   Gotcha (wgpu 22): TextureView bindings in a BindGroup must match the
//   multisampled=false flag in the BindGroupLayout; the intermediate view
//   is single-sample (the MSAA was resolved), so the layout declared in
//   pipelines.rs (path_sprite_bgl, binding=1, multisampled=false) is correct.

// ── common header ─────────────────────────────────────────────────────────── //

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

fn clip_distances_rect(pos: vec2<f32>, clip: Rect) -> vec4<f32> {
    let tl = pos - clip.origin;
    let br = clip.origin + clip.size - pos;
    return vec4<f32>(tl.x, br.x, tl.y, br.y);
}

// ── group(0): globals ─────────────────────────────────────────────────────── //

@group(0) @binding(0) var<uniform> globals: RenderParams;

// ── path rasterization types ──────────────────────────────────────────────── //

// Per-vertex data for the path rasterization triangle mesh.
//
// xy_position: screen-space pixel position of this mesh vertex.
// st:          implicit quadratic bezier barycentric coordinates.
//              Curve boundary:  f(s,t) = s*s - t = 0.
//              Inside-fill triangles use (0,0), (0.5,0), (1,1) encoding
//              (Loop-Blinn 2005).  The signed distance drives alpha.
// color:       premultiplied RGBA fill color for this path.
// clip_bounds: axis-aligned clip rectangle in screen pixels (inclusive).
struct PathVertex {
    xy_position: vec2<f32>,
    st: vec2<f32>,
    color: vec4<f32>,
    clip_bounds: Rect,
}

struct PathRasterVaryings {
    @builtin(position)              position:  vec4<f32>,
    @location(0)                    st:        vec2<f32>,
    @location(1)                    clip_dist: vec4<f32>,
    @location(2) @interpolate(flat) vertex_id: u32,
}

// ── path sprite types ─────────────────────────────────────────────────────── //

// Per-instance data for the compositing quad (pass 2).
//
// bounds:      screen-pixel destination rectangle on the surface.
// clip_bounds: scissor rectangle in screen pixels.
// uv_min:      normalised UV of the top-left corner in the intermediate texture.
//              Typically  bounds.origin / viewport_size.
// uv_max:      normalised UV of the bottom-right corner.
//              Typically  (bounds.origin + bounds.size) / viewport_size.
struct PathSprite {
    bounds:      Rect,
    clip_bounds: Rect,
    uv_min:      vec2<f32>,
    uv_max:      vec2<f32>,
}

struct PathVaryings {
    @builtin(position)              position:    vec4<f32>,
    @location(0)                    atlas_uv:    vec2<f32>,
    @location(1)                    clip_dist:   vec4<f32>,
    @location(2) @interpolate(flat) instance_id: u32,
}

// ── group(1) bindings ─────────────────────────────────────────────────────── //
//
// WGSL allows two global variables at the same (group, binding) slot when no
// single entry point accesses both — naga validates reachability per entry
// point, not globally.  vs_path_raster / fs_path_raster use only path_vertices;
// vs_path / fs_path use only path_sprites / path_tex / path_smp.

// group(1) for path_rasterization pipeline:
@group(1) @binding(0) var<storage, read> path_vertices: array<PathVertex>;

// group(1) for paths pipeline:
@group(1) @binding(0) var<storage, read> path_sprites: array<PathSprite>;
@group(1) @binding(1) var path_tex: texture_2d<f32>;
@group(1) @binding(2) var path_smp: sampler;

// ────────────────────────────────────────────────────────────────────────────
// Pass 1 — path_rasterization entry points
// ────────────────────────────────────────────────────────────────────────────

// Vertex: lift each mesh position to NDC.
// Positions are screen-space pixels tessellated by the CPU; no bounds-scaling
// is needed (contrast with sprite pipelines that scale a unit quad).

@vertex
fn vs_path_raster(
    @builtin(vertex_index) vid: u32,
) -> PathRasterVaryings {
    let v = path_vertices[vid];

    var out: PathRasterVaryings;
    out.position  = to_ndc(v.xy_position, globals.viewport_size);
    out.st        = v.st;
    out.clip_dist = clip_distances_rect(v.xy_position, v.clip_bounds);
    out.vertex_id = vid;
    return out;
}

// Fragment: Loop-Blinn implicit quadratic SDF coverage.
//
// The signed distance to the quadratic curve  f(s,t) = s² - t  is:
//
//   f    = s*s - t
//   grad = (2s·∂s/∂x - ∂t/∂x,  2s·∂s/∂y - ∂t/∂y)
//   dist = f / |grad|          (pixels, positive = outside curve)
//
// Alpha = smoothstep(0.5, -0.5, dist)  →  1 inside, 0 outside, AA at ±0.5 px.
// Output is premultiplied RGBA (color.rgb * a, a) for PREMULTIPLIED_ALPHA_BLENDING.

@fragment
fn fs_path_raster(v: PathRasterVaryings) -> @location(0) vec4<f32> {
    if any(v.clip_dist < vec4<f32>(0.0)) {
        return vec4<f32>(0.0);
    }

    let s = v.st.x;
    let t = v.st.y;

    // Screen-space partial derivatives of the interpolated (s,t) coordinates.
    let ds = vec2<f32>(dpdx(s), dpdy(s));
    let dt = vec2<f32>(dpdx(t), dpdy(t));

    // Implicit function value and screen-space gradient magnitude.
    let f         = s * s - t;
    let grad      = 2.0 * s * ds - dt;
    let grad_len  = length(grad);

    var alpha: f32;
    if grad_len < 0.001 {
        // Degenerate / flat interior fill — fully covered.
        alpha = 1.0;
    } else {
        // Signed pixel distance to the curve boundary.
        let dist = f / grad_len;
        // smoothstep: dist < -0.5 → alpha=1 (inside); dist > 0.5 → alpha=0 (outside).
        alpha = smoothstep(0.5, -0.5, dist);
    }

    let color = path_vertices[v.vertex_id].color;
    let a = color.a * alpha;
    // Premultiplied output: RGB pre-multiplied by alpha.
    return vec4<f32>(color.rgb * a, a);
}

// ────────────────────────────────────────────────────────────────────────────
// Pass 2 — paths (composite intermediate → surface)
// ────────────────────────────────────────────────────────────────────────────

// Vertex: standard unit-quad scaled to the sprite bounds, UV from uv_min/uv_max.

@vertex
fn vs_path(
    @builtin(vertex_index)   vid: u32,
    @builtin(instance_index) iid: u32,
) -> PathVaryings {
    let sprite = path_sprites[iid];
    let uv     = unit_vertex(vid);
    let pos    = uv * sprite.bounds.size + sprite.bounds.origin;
    let tex_uv = sprite.uv_min + uv * (sprite.uv_max - sprite.uv_min);

    var out: PathVaryings;
    out.position    = to_ndc(pos, globals.viewport_size);
    out.atlas_uv    = tex_uv;
    out.clip_dist   = clip_distances_rect(pos, sprite.clip_bounds);
    out.instance_id = iid;
    return out;
}

// Fragment: sample the resolved intermediate path coverage texture and output.
// The intermediate is already premultiplied; pipeline blend = One/OneMinusSrcAlpha.
// Sample before clip test so screen-space derivatives remain well-defined
// (matching the pattern in poly_sprite.wgsl).

@fragment
fn fs_path(v: PathVaryings) -> @location(0) vec4<f32> {
    let sample = textureSample(path_tex, path_smp, v.atlas_uv);

    if any(v.clip_dist < vec4<f32>(0.0)) {
        return vec4<f32>(0.0);
    }

    // Intermediate is premultiplied; pass through for One/OneMinusSrcAlpha compositing.
    return sample;
}
