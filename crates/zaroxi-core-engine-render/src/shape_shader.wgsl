// Shape shader: renders colored quads with optional rounded corners.
// When corner_radius > 0, the fragment shader applies a signed-distance
// rounded-rect mask using the interpolated local UV coordinates.

struct VsIn {
    @location(0) pos: vec2<f32>,
    @location(1) uv: vec2<f32>,   // local quad coords: (0,0)..(1,1)
    @location(2) color: vec4<f32>,
    @location(3) corner_radius: f32,
}

struct VsOut {
    @builtin(position) position: vec4<f32>,
    @location(0) color: vec4<f32>,
    @location(1) uv: vec2<f32>,
    @location(2) corner_radius: f32,
}

@vertex
fn vs_main(in: VsIn) -> VsOut {
    var out: VsOut;
    out.position = vec4<f32>(in.pos, 0.0, 1.0);
    out.color = in.color;
    out.uv = in.uv;
    out.corner_radius = in.corner_radius;
    return out;
}

// Compute signed distance to a rounded rectangle.
// uv: local coordinate within [0,1]x[0,1] quad
// corner_radius: radius in NDC units (normalized from pixel radius)
// Returns < 0 inside the shape, > 0 outside.
fn rounded_rect_sdf(uv: vec2<f32>, corner_radius: f32) -> f32 {
    // Map uv from [0,1] to [-0.5, 0.5] centered
    let p = uv - vec2<f32>(0.5);
    let half_size = vec2<f32>(0.5);
    // Distance to inner rect corner (accounting for radius)
    let q = abs(p) - (half_size - vec2<f32>(corner_radius));
    return length(max(q, vec2<f32>(0.0))) + min(max(q.x, q.y), 0.0) - corner_radius;
}

@fragment
fn fs_main(in: VsOut) -> @location(0) vec4<f32> {
    if in.corner_radius <= 0.0 {
        return in.color;
    }

    let d = rounded_rect_sdf(in.uv, in.corner_radius);
    // Smooth anti-aliased edge: alpha = 1.0 inside, 0.0 outside, blend in between
    let aa_width = fwidth(d) * 1.5;
    let alpha = 1.0 - smoothstep(-aa_width, aa_width, d);
    return vec4<f32>(in.color.rgb, in.color.a * alpha);
}
