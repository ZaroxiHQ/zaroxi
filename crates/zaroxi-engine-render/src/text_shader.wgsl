@group(0) @binding(0)
var font_tex: texture_2d<f32>;

@group(0) @binding(1)
var font_sampler: sampler;

// Diagnostic toggles:
// Set DIAGNOSTIC_MAGENTA or DIAGNOSTIC_SOLID to true for temporary rendering checks.
// These are compile-time constants; leave them false in production.
const DIAGNOSTIC_MAGENTA: bool = true;
const DIAGNOSTIC_SOLID: bool = false;

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
    // Sample coverage from the atlas (atlas encoded as R8Unorm -> use .r).
    let sampled = textureSample(font_tex, font_sampler, in.uv);
    let coverage = sampled.r;

    // Diagnostic 1: force magenta glyphs unconditionally to validate geometry/pipeline.
    // This temporarily bypasses atlas alpha checks so we can verify whether glyph quads
    // are being rasterized/blended by the GPU pipeline. Remove or set to false once
    // the issue is diagnosed.
    if DIAGNOSTIC_MAGENTA {
        return vec4<f32>(1.0, 0.0, 1.0, 1.0);
    }

    // Diagnostic 2: output vertex RGB with solid alpha to verify color path (bypass atlas).
    if DIAGNOSTIC_SOLID {
        return vec4<f32>(in.color.rgb, 1.0);
    }

    // Normal rendering: treat atlas coverage as the alpha multiplier for the vertex color.
    // This expects the pipeline to use ALPHA_BLENDING and that the atlas stores straight alpha.
    let alpha = in.color.a * coverage;
    return vec4<f32>(in.color.rgb, alpha);
}
