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
// Env-gated debug helpers (piped in by pipeline build) to aid triage at runtime.
const ZAROXI_TEXT_SHOW_ATLAS_MASK: bool = false;
const ZAROXI_TEXT_SHOW_GLYPH_ALPHA: bool = false;

// Instance-driven vertex input: each instance provides quad origin (NDC), size (NDC), UV rect and color.
struct VSOut {
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec2<f32>,
    @location(1) color: vec4<f32>,
}

@vertex
fn vs_main(
    @builtin(vertex_index) vertex_index: u32,
    @builtin(instance_index) instance_index: u32,
    @location(0) i_pos: vec2<f32>,
    @location(1) i_size: vec2<f32>,
    @location(2) i_uv0: vec2<f32>,
    @location(3) i_uv1: vec2<f32>,
    @location(4) i_color: vec4<f32>,
) -> VSOut {
    var out: VSOut;
    // Triangle-list corners for two triangles forming a quad (0..5)
    let corners: array<vec2<f32>, 6> = array<vec2<f32>, 6>(
        vec2<f32>(0.0, 0.0),
        vec2<f32>(1.0, 0.0),
        vec2<f32>(1.0, 1.0),
        vec2<f32>(0.0, 0.0),
        vec2<f32>(1.0, 1.0),
        vec2<f32>(0.0, 1.0),
    );
    let corner = corners[vertex_index];
    let pos_ndc = i_pos + corner * i_size;
    out.position = vec4<f32>(pos_ndc, 0.0, 1.0);
    out.uv = mix(i_uv0, i_uv1, corner);
    out.color = i_color;
    return out;
}

@fragment
fn fs_main(in: VSOut) -> @location(0) vec4<f32> {
    if DIAGNOSTIC_MAGENTA {
        return vec4<f32>(1.0, 0.0, 1.0, 1.0);
    }

    // Robust coverage extraction: take the max of all sampled channels.
    // Atlas textures may be R8 (coverage in .r) or RGBA (coverage in .a or any channel).
    let sampled = textureSample(font_tex, font_sampler, in.uv);
    let coverage = max(max(sampled.r, sampled.g), max(sampled.b, sampled.a));

    // Env-gated debug: show atlas mask as grayscale (useful to verify sampling).
    if ZAROXI_TEXT_SHOW_ATLAS_MASK {
        return vec4<f32>(coverage, coverage, coverage, 1.0);
    }
    // Env-gated debug: show glyph shapes as alpha only (white color, coverage as alpha)
    if ZAROXI_TEXT_SHOW_GLYPH_ALPHA {
        return vec4<f32>(1.0, 1.0, 1.0, coverage);
    }

    if DIAGNOSTIC_SHOW_COVERAGE {
        return vec4<f32>(coverage, coverage, coverage, 1.0);
    }

    if DIAGNOSTIC_SOLID {
        return vec4<f32>(in.color.rgb, 1.0);
    }

    let alpha = in.color.a * coverage;
    return vec4<f32>(in.color.rgb, alpha);
}
