struct VsIn {
    @location(0) pos: vec2<f32>,
    @location(1) uv: vec2<f32>, // preserved for layout compatibility, unused
    @location(2) color: vec4<f32>,
    @location(3) corner_radius: f32, // preserved for layout compatibility, unused
}

struct VsOut {
    @builtin(position) position: vec4<f32>,
    @location(0) color: vec4<f32>,
}

@vertex
fn vs_main(in: VsIn) -> VsOut {
    var out: VsOut;
    out.position = vec4<f32>(in.pos, 0.0, 1.0);
    out.color = in.color;
    return out;
}

@fragment
fn fs_main(in: VsOut) -> @location(0) vec4<f32> {
    // Return the vertex color directly (no texture sampling, no alpha modulation).
    return in.color;
}
