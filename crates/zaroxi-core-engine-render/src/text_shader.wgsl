@group(0) @binding(0)
var font_tex: texture_2d<f32>;

@group(0) @binding(1)
var font_sampler: sampler;

// Diagnostic toggles:
// Set DIAGNOSTIC_MAGENTA or DIAGNOSTIC_SOLID to true for temporary rendering checks.
// These are compile-time constants; toggle them during investigation and revert to
// false for normal rendering.
const DIAGNOSTIC_MAGENTA: bool = false;
const DIAGNOSTIC_SOLID: bool = false;
// Proof mode: when set to true render sampled glyph coverage as grayscale.
// Useful to verify atlas content/sampling without applying vertex colors.
const DIAGNOSTIC_SHOW_COVERAGE: bool = false;

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
    // Quick diagnostic: force magenta output to validate that glyph quads are
    // being emitted and reach the fragment stage. Toggle DIAGNOSTIC_MAGENTA at
    // compile-time to enable/disable this fast check.
    if DIAGNOSTIC_MAGENTA {
        return vec4<f32>(1.0, 0.0, 1.0, 1.0);
    }

    // Sample coverage from the atlas. Some atlas implementations pack coverage
    // in the red channel (R8) while others use the alpha channel (RGBA).
    // To make the shader robust across atlas formats, sample both:
    // - Prefer red (sampled.r) when present; fall back to alpha (sampled.a) when red is near zero.
    let sampled = textureSample(font_tex, font_sampler, in.uv);
    var coverage: f32 = sampled.r;
    if coverage < 0.001 {
        // fall back to alpha channel when red is effectively empty
        coverage = sampled.a;
    }

    // Diagnostic proof: show coverage as grayscale if enabled.
    if DIAGNOSTIC_SHOW_COVERAGE {
        // Render sampled coverage as a visible grayscale map (alpha=1.0) so we can
        // verify atlas uploads and UVs independently of blending.
        return vec4<f32>(coverage, coverage, coverage, 1.0);
    }

    // Diagnostic solid color bypass (keeps the normal path selectable).
    if DIAGNOSTIC_SOLID {
        return vec4<f32>(in.color.rgb, 1.0);
    }

    // Normal rendering: treat atlas coverage as the alpha multiplier for the vertex color.
    // We output straight alpha (not premultiplied). The pipeline uses ALPHA_BLENDING.
    let alpha = in.color.a * coverage;
    return vec4<f32>(in.color.rgb, alpha);
}
