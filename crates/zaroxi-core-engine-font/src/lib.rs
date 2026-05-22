 // zaroxi-core-engine-font
 // Auto-generated crate stub for the Zaroxi migration.
 // Responsibility: Font management and shaping for engine text.
 //
 // Minimal, non-brittle font/metrics API used by Phase 5 (editor text foundation).
 // This crate intentionally provides a tiny, deterministic monospace metric
 // abstraction used by the engine text layout layer. It does not embed or
 // vendor a full shaping/font stack; that will be layered later behind a
 // backend implementation.

 #![allow(dead_code)]
 #![allow(unused_imports)]

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
     // Chosen conservative defaults: 8x16 (typical terminal monospace).
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

 /// Marker retained for packaging/compatibility.
 pub fn _crate_marker() {
     // kept to avoid changing crate layout semantics in this phase.
 }
