/*!
Small internal seam: let the render crate consume engine-owned text backends
through the Zaroxi-owned abstraction without exposing Glyphon.

Design:
- Calls `zaroxi_core_engine_text::new_backend()` as the single public seam.
- Returns `TextLayout` produced by the backend.
- Keeps Glyphon and any concrete backend types private to `zaroxi-core-engine-text`.
*/

use zaroxi_core_engine_text::{new_backend, TextLabel, TextLayout};

/// Layout a plain string via the engine text backend seam.
///
/// Constructs the backend via `new_backend()` (the only public seam) and
/// returns the resulting `TextLayout`. This function is deliberately minimal:
/// it preserves the abstraction boundary and provides a tiny adapter the render
/// crate can call directly.
pub fn layout_label_for_render(label: &str, max_width: Option<u32>) -> TextLayout {
    let backend = new_backend();
    let tl = TextLabel::from(label);
    backend.layout_label(&tl, max_width)
}
