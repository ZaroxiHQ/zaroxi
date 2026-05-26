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

 use std::path::Path;
 use std::fs;
 use std::io;

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
     Font {
         family: "ZaroxiMono".to_string(),
         char_width: 8,
         line_height: 16,
     }
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
     // Canonical asset path (workspace-root relative). This keeps discovery simple
     // and consistent across binaries that run from the repository root.
     // If your runtime requires a different discovery mechanism (embedded assets,
     // packaging, etc.) adapt this loader in the future to return embedded bytes.
     let asset_path = Path::new("zaroxi/assets/fonts/JetBrainsMonoNerdFont-Regular.ttf");

     // Try to read the asset file
     match fs::read(&asset_path) {
         Ok(bytes) => Ok(bytes),
         Err(e) => {
             // Produce a helpful error with suggestions.
             let msg = match e.kind() {
                 io::ErrorKind::NotFound => format!(
                     "project font not found at {:?}. Ensure the repository root contains the file and you're running from workspace root.",
                     asset_path
                 ),
                 _ => format!("failed to read project font at {:?}: {}", asset_path, e),
             };
             Err(msg)
         }
     }
 }

 /// Marker retained for packaging/compatibility.
 pub fn _crate_marker() {
     // kept to avoid changing crate layout semantics in this phase.
 }
