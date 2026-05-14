@group(0) @binding(0)
var font_tex: texture_2d<f32>;

@group(0) @binding(1)
var font_sampler: sampler;

struct VertexInput {
    @location(0) pos: vec2<f32>,
    @location(1) uv: vec2<f32>,
    @location(2) color: vec4<f32>,
};

struct VSOut {
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec2<f32>,
    @location(1) color: vec4<f32>,
};

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
    // Diagnostic: sample the single-channel font atlas into coverage and
    // render bright red glyphs where coverage > 0. This helps verify atlas
    // upload and sampling channel correctness. Uses the red channel (r)
    // because the atlas is uploaded as R8Unorm.
    let coverage = textureSample(font_tex, font_sampler, in.uv).r;
    return vec4<f32>(1.0, 0.0, 0.0, coverage);
}
