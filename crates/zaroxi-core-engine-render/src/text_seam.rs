/*!
Small internal seam: let the render crate consume engine-owned text backends
through the Zaroxi-owned abstraction without exposing Glyphon.

Design:
- Calls `zaroxi_core_engine_text::new_backend()` as the single public seam.
- Adapts the backend `TextLayout` into a render-local `TextLayout` that
  exposes simple width/height metrics alongside the lines vector. This avoids
  leaking engine-backend types into the render crate while providing the
  minimal fields `plan.rs` expects.
- Keeps Glyphon and any concrete backend types private to `zaroxi-core-engine-text`.
*/

use zaroxi_core_engine_text::{new_backend, TextLabel};

/// Minimal render-facing layout struct.
///
/// This local type intentionally mirrors only what the render path needs:
/// - lines: the raw lines returned by the backend
/// - width: deterministic approximate width in pixels
/// - height: deterministic approximate height in pixels
#[derive(Clone, Debug)]
pub struct TextLayout {
    pub lines: Vec<String>,
    pub width: u32,
    pub height: u32,
}

/// Layout a plain string via the engine text backend seam.
///
/// Constructs the backend via `new_backend()` (the only public seam) and
/// adapts the backend `TextLayout` into the render-local `TextLayout`.
/// Width/height are computed with simple, deterministic monospace heuristics
/// so the render crate can make basic layout decisions without depending on
/// backend-specific types.
pub fn layout_label_for_render(label: &str, max_width: Option<u32>) -> TextLayout {
    let backend = new_backend();
    let tl = TextLabel::from(label);
    let backend_layout = backend.layout_label(&tl, max_width);

    // Deterministic, conservative metrics (matches engine-font defaults used in Phase 5):
    // char_width = 8, line_height = 16
    let mut max_chars: usize = 0;
    for l in &backend_layout.lines {
        let c = l.chars().count();
        if c > max_chars {
            max_chars = c;
        }
    }
    let width = (max_chars as u32).saturating_mul(8);
    let height = (backend_layout.lines.len() as u32).saturating_mul(16);

    TextLayout {
        lines: backend_layout.lines,
        width,
        height,
    }
}
