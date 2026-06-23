// zaroxi-core-engine-font
// Auto-generated crate stub for the Zaroxi migration.
// Responsibility: Font management and shaping for engine text.
//
// This crate now exposes a small, focused loader API used by higher-level
// rendering code. It owns the convention for locating the project font asset
// and returns raw font bytes so renderers (Cosmic Text path) can initialize
// font systems using a single canonical loader.
//
// NOTE: Keep this crate minimal — it does not attempt to perform shaping or
// rasterization, it only discovers/loads font bytes and provides lightweight
// font metric helpers retained from the previous stub.

#![allow(dead_code)]
#![allow(unused_imports)]

use std::fs;
use std::io;
use std::path::Path;

/// Minimal font descriptor used by the engine text layout.
#[derive(Clone, Debug)]
pub struct Font {
    /// Logical family name (informational).
    pub family: String,
    /// Monospace character advance in pixels.
    pub char_width: u32,
    /// Line height in pixels (baseline-to-baseline).
    pub line_height: u32,
}

/// Simple glyph metrics placeholder for future refinement.
#[derive(Clone, Debug)]
pub struct GlyphMetrics {
    /// Advance in pixels for the glyph.
    pub advance: u32,
}

/// Load a bundled, deterministic monospace font descriptor appropriate for
/// Phase 5 rendering. This returns a small Font struct with sensible
/// monospace metrics. Consumers should treat this as a stable, portable
/// metric provider rather than a full shaping/font loader.
pub fn load_bundled_monospace() -> Font {
    // Conservative defaults for legacy code paths. New Cosmic Text renderer
    // should prefer loading the real project font via `load_project_font_bytes`.
    Font { family: "ZaroxiMono".to_string(), char_width: 8, line_height: 16 }
}

/// Measure a UTF-8 string using the monospace advance from `font`.
/// This is intentionally simple: width = N_glyphs * char_width.
pub fn measure_text_width(text: &str, font: &Font) -> u32 {
    text.chars().count() as u32 * font.char_width
}

/// Per-line height accessor.
pub fn line_height(font: &Font) -> u32 {
    font.line_height
}

/// Primary loader API: load the project's canonical font asset bytes.
///
/// The function attempts to read the TTF file from the workspace asset path:
/// `zaroxi/assets/fonts/JetBrainsMonoNerdFont-Regular.ttf`
///
/// Returns:
/// - Ok(Vec<u8>) with file bytes on success
/// - Err(String) with a clear error message on failure
///
/// Consumers (renderers) should pass these bytes into their font system
/// (for example, Cosmic Text's FontSystem) and handle shaping/rasterization
/// there. Keeping raw bytes here avoids coupling renderers to file layout.
pub fn load_project_font_bytes() -> Result<Vec<u8>, String> {
    // Try a small deterministic set of candidate locations so tests/CI running
    // from various crate working directories can still find the repository asset.
    //
    // The "Mono" Nerd Font variant (`...NerdFontMono-Regular.ttf`) renders all
    // icon glyphs at a single cell width, which keeps the explorer icon column
    // perfectly aligned. We PREFER it when present and fall back to the standard
    // variant otherwise — so the swap takes effect the moment the Mono asset is
    // dropped into `assets/fonts/`, with no code change.
    let rels = [
        "assets/fonts/JetBrainsMonoNerdFontMono-Regular.ttf",
        "assets/fonts/JetBrainsMonoNerdFont-Regular.ttf",
    ];
    let mut tried: Vec<std::path::PathBuf> = Vec::new();

    let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").ok();
    for rel in rels.iter() {
        // Basic candidates relative to current cwd.
        tried.push(Path::new(rel).to_path_buf());
        tried.push(Path::new("../").join(rel));
        tried.push(Path::new("../../").join(rel));

        // If CARGO_MANIFEST_DIR is available (common in tests/builds), try it
        // and its parents.
        if let Some(ref manifest_dir) = manifest_dir {
            let manifest = Path::new(manifest_dir);
            tried.push(manifest.join(rel));
            tried.push(manifest.join("..").join(rel));
            tried.push(manifest.join("..").join("..").join(rel));
        }
    }

    // Try each candidate in order (all Mono-variant locations first, then the
    // standard variant) and return the first successful read.
    for p in tried.iter() {
        if let Ok(bytes) = fs::read(p) {
            return Ok(bytes);
        }
    }

    // Nothing found — produce an informative error listing attempted locations.
    let tried_list: Vec<String> = tried.iter().map(|p| format!("{:?}", p)).collect();
    Err(format!(
        "project font not found. Attempted paths: {}. Ensure the repository contains assets/fonts/JetBrainsMonoNerdFont-Regular.ttf and you are running from the workspace root or set CARGO_MANIFEST_DIR.",
        tried_list.join(", ")
    ))
}

/// New small, explicit public API alias for clarity: return the project font bytes.
///
/// Rationale:
/// - This makes the canonical font-loading seam explicit for consumers in other
///   crates (for example `zaroxi-interface-desktop`) while keeping the
///   implementation and discovery policy centralized here.
/// - The function intentionally returns raw bytes so higher-level crates that
///   own shaping/layout (for example Cosmic Text integration) can register the
///   font into their runtime-owned FontSystem without pulling font-discovery
///   logic into those crates.
pub fn project_font_bytes() -> Result<Vec<u8>, String> {
    load_project_font_bytes()
}

/// Marker retained for packaging/compatibility.
pub fn _crate_marker() {
    // kept to avoid changing crate layout semantics in this phase.
}
