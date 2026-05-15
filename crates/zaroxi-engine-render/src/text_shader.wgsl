@group(0) @binding(0)
var font_tex: texture_2d<f32>;

@group(0) @binding(1)
var font_sampler: sampler;

// Diagnostic toggles:
// Set DIAGNOSTIC_MAGENTA or DIAGNOSTIC_SOLID to true for temporary rendering checks.
// These are compile-time constants; toggle them during investigation and revert to
// false for normal rendering.
const DIAGNOSTIC_MAGENTA: bool = true;
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
    // Sample coverage from the atlas (atlas encoded as R8Unorm -> use .r).
    let sampled = textureSample(font_tex, font_sampler, in.uv);
    let coverage = sampled.r;

    // Diagnostic proof: show coverage as grayscale if enabled.
    if DIAGNOSTIC_SHOW_COVERAGE {
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
