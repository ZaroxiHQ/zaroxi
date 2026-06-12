/*!
Minimal engine-owned text backend seam.

This module exposes:
- TextLayout: a small, deterministic layout result type consumable by engine-render.
- TextBackend: a trait abstraction for layout operations.
- DummyBackend: a deterministic, always-available stub backend (the default).

Design rationale:
- Keep the text backend as a clean abstraction behind this crate's public seam.
- Do not leak implementation types into interface or render crates.
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
/// build and test without pulling heavy GPU deps into every CI run.
pub(crate) struct DummyBackend {}

impl DummyBackend {
    pub fn new() -> Self {
        DummyBackend {}
    }
}

impl TextBackend for DummyBackend {
    fn layout_label(&self, label: &TextLabel, _max_width: Option<u32>) -> TextLayout {
        let width = (label.text.chars().count() as u32).saturating_mul(8);
        let height = 16;
        TextLayout { width, height, lines: vec![label.text.clone()] }
    }
}

/// Construct the default engine text backend.
///
/// Returns the DummyBackend. Consumers outside this crate do not need to know
/// which implementation is used.
#[doc(hidden)]
pub fn new_backend() -> Box<dyn TextBackend> {
    Box::new(DummyBackend::new())
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
}
