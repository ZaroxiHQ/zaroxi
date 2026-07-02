#![allow(dead_code)]
// Auto-generated stub for `zaroxi-core-engine-text`.
//
// Phase 5 additions: a minimal, stable layout helper that converts plain UTF-8
// lines into scene text primitives using monospace metrics from
// zaroxi-core-engine-font. This intentionally keeps shaping/layout trivial and
// non-brittle.

pub const CRATE_NAME: &str = "zaroxi-core-engine-text";

pub fn info() -> &'static str {
    CRATE_NAME
}

mod label;
pub use label::TextLabel;

mod backend;
pub use backend::{TextBackend, new_backend};
// NOTE: We intentionally DO NOT re-export the GlyphonBackend concrete type here.
// Keeping Glyphon types fully private to this crate preserves the architecture
// invariant that implementation details (Glyphon) never leak into higher layers.
// DummyBackend is intentionally NOT publicly re-exported to avoid outer-layer
// crates instantiating concrete engine backends directly. Consumers should use
// the render-facing seam:
//   zaroxi_core_engine_render::text_seam::layout_label_for_render
// or the public `new_backend()` seam when working inside engine-layer crates.

/// Phase 5: very small text-layout helper API.
///
/// This helper produces simple scene-level text primitives (monospace, single
/// run per input line) which are stable across platforms and adequate for the
/// first editor content rendering. It intentionally avoids shaping and complex
/// attributes.
pub mod plain {
    use zaroxi_core_engine_font::Font;
    use zaroxi_core_engine_scene::TextPrimitive;

    /// A tiny layout result wrapper used by the presenter to consume shaped
    /// runs and inject them into the render paint plan.
    #[derive(Clone, Debug)]
    pub struct LineLayout {
        pub primitives: Vec<TextPrimitive>,
    }

    impl Default for LineLayout {
        fn default() -> Self {
            Self::new()
        }
    }

    impl LineLayout {
        pub fn new() -> Self {
            LineLayout { primitives: Vec::new() }
        }
    }

    /// Layout a slice of UTF-8 lines into simple TextPrimitive instances.
    ///
    /// - lines: source text lines (no trailing newline semantics required).
    /// - font: monospace font metrics (see zaroxi-core-engine-font).
    /// - editor_x/editor_y: absolute origin (top-left) inside the window where the
    ///   first line should be placed (these coordinates should come from the shell layout).
    /// - max_width: optional clamp for primitive width (left for presenter/renderer to honor).
    pub fn layout_plain_lines<S: AsRef<str>>(
        lines: &[S],
        font: &Font,
        editor_x: u32,
        editor_y: u32,
        max_width: Option<u32>,
    ) -> LineLayout {
        let mut out = LineLayout::new();
        let lh = font.line_height;
        for (i, s) in lines.iter().enumerate() {
            let y = editor_y + (i as u32) * lh;
            let tp = TextPrimitive {
                x: editor_x,
                y,
                text: s.as_ref().to_string(),
                font_name: font.family.clone(),
                max_width,
            };
            out.primitives.push(tp);
        }
        out
    }
}

// A small shim type kept for compatibility with other internal APIs that may
// reference `TextLayout`. This is intentionally lightweight.
#[derive(Clone, Debug)]
pub struct TextLayout {
    pub lines: Vec<String>,
}

impl TextLayout {
    pub fn from_lines(lines: Vec<String>) -> Self {
        TextLayout { lines }
    }
}
