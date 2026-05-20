/*!
Minimal engine-owned text backend seam.

This module exposes:
- TextLayout: a small, deterministic layout result type consumable by engine-render.
- TextBackend: a trait abstraction for layout operations.
- DummyBackend: a deterministic, always-available stub backend (used by default).
- GlyphonBackend (feature-gated "glyphon_backend"): a small adapter that sits behind
  the TextBackend trait and delegates to Glyphon when the feature is enabled.

Design rationale:
- Keep Glyphon as an implementation detail behind this crate's public seam.
- Do not leak Glyphon types into interface or render crates.
- Default to a tiny deterministic DummyBackend so CI/tests remain fast and stable.
*/

use crate::label::TextLabel;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TextLayout {
    /// Total logical width in pixels (approximate for stubs).
    pub width: u32,
    /// Total logical height in pixels.
    pub height: u32,
    /// Logical lines produced by layout (cloned strings).
    pub lines: Vec<String>,
}

/// Minimal trait representing a text/layout backend owned by the engine layer.
///
/// Implementations must not expose implementation-specific types in their API so
/// consumers (render, interface) remain backend-agnostic.
pub trait TextBackend: Send + Sync {
    fn layout_label(&self, label: &TextLabel, max_width: Option<u32>) -> TextLayout;
}

/// Very small deterministic backend used as the default. This lets the workspace
/// build and test without pulling Glyphon into every CI run.
pub struct DummyBackend {}

impl DummyBackend {
    pub fn new() -> Self {
        DummyBackend {}
    }
}

impl TextBackend for DummyBackend {
    fn layout_label(&self, label: &TextLabel, _max_width: Option<u32>) -> TextLayout {
        // Deterministic, conservative approximation:
        // width = char_count * 8, height = 16, single line only.
        let width = (label.text.chars().count() as u32).saturating_mul(8);
        let height = 16;
        TextLayout {
            width,
            height,
            lines: vec![label.text.clone()],
        }
    }
}

#[cfg(feature = "glyphon_backend")]
mod glyphon_impl {
    //! Feature-gated Glyphon-backed implementation.
    //!
    //! This module intentionally keeps Glyphon usage private and exposes only
    //! the engine-owned GlyphonBackend type which implements TextBackend. The
    //! module and its concrete types are NOT publicly re-exported so Glyphon
    //! types never leak out of this crate's abstraction boundary.
    //!
    //! NOTE: imports and usage are minimal so the public API of this crate never
    //! mentions Glyphon types.

    use super::{TextBackend, TextLayout};
    use crate::label::TextLabel;

    // Glyphon imports are feature-gated so workspace-level builds remain clean.
    // The exact glyphon API surface may change; this adapter keeps usage local.
    use glyphon::FontSystem;

    struct GlyphonBackend {
        fs: FontSystem,
    }

    impl GlyphonBackend {
        // Keep constructor visible inside the crate so `new_backend()` can call it,
        // but do NOT export the concrete type from the crate public API.
        pub fn new() -> Self {
            let fs = FontSystem::new();
            // Real font loading/metrics would be performed here in a full implementation.
            GlyphonBackend { fs }
        }
    }

    impl TextBackend for GlyphonBackend {
        fn layout_label(&self, label: &TextLabel, _max_width: Option<u32>) -> TextLayout {
            // Minimal, safe approximation when Glyphon is present.
            // A real implementation would use shaping/layout from Glyphon and return
            // accurate lines/metrics. Keep this simple to prove the adapter seam.
            let width = (label.text.chars().count() as u32).saturating_mul(9);
            let height = 18;
            TextLayout {
                width,
                height,
                lines: vec![label.text.clone()],
            }
        }
    }
}

/// Construct the default engine text backend.
///
/// When the "glyphon_backend" feature is enabled this will return the GlyphonBackend;
/// otherwise it returns the DummyBackend. Consumers outside this crate do not need
/// to know which implementation is used.
pub fn new_backend() -> Box<dyn TextBackend> {
    #[cfg(feature = "glyphon_backend")]
    {
        Box::new(glyphon_impl::GlyphonBackend::new())
    }
    #[cfg(not(feature = "glyphon_backend"))]
    {
        Box::new(DummyBackend::new())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::label::TextLabel;

    #[test]
    fn dummy_backend_layouts_label() {
        let backend = DummyBackend::new();
        let label = TextLabel::from("hello");
        let layout = backend.layout_label(&label, None);
        assert_eq!(layout.lines, vec!["hello".to_string()]);
        assert_eq!(layout.height, 16);
        assert_eq!(layout.width, 5u32.saturating_mul(8));
    }

    // This test only runs when the glyphon_backend feature is explicitly enabled.
    #[cfg(feature = "glyphon_backend")]
    #[test]
    fn glyphon_backend_constructs_and_layouts() {
        let backend = glyphon_impl::GlyphonBackend::new();
        let label = TextLabel::from("hello glyphon");
        let layout = backend.layout_label(&label, None);
        assert!(!layout.lines.is_empty());
    }
}
