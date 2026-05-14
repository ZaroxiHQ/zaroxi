@group(0) @binding(0)
var font_tex: texture_2d<f32>;

@group(0) @binding(1)
var font_sampler: sampler;

struct VertexInput {
    @location(0) pos: vec2<f32>,
    @location(1) uv: vec2<f32>,
    @location(2) color: vec4<f32>,
}

struct VSOut {
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec2<f32>,
    @location(1) color: vec4<f32>,
}

@vertex
fn vs_main(in: VertexInput) -> VSOut {
    var out: VSOut;
    out.position = vec4<f32>(in.pos, 0.0, 1.0);
    out.uv = in.uv;
    out.color = in.color;
    return out;
}

@fragment
fn fs_main(in: VSOut) -> @location(0) vec4<f32> {
    // Text rendering: sample the single-channel font atlas (R8Unorm). The atlas
    // encodes coverage in the red channel. Use sampled coverage as the fragment
    // alpha and keep the vertex color RGB as the output color.
    let coverage = textureSample(font_tex, font_sampler, in.uv).r;
    let alpha = in.color.a * coverage;
    return vec4<f32>(in.color.rgb, alpha);
}
