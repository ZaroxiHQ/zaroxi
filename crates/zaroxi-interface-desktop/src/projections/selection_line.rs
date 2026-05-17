#![allow(clippy::missing_const_for_fn)]

// Tiny, explicit shell-facing line that reports whether a selection exists
// and, when present, its start/end bounds in a compact shell-friendly string.
//
// Lifecycle rule (Phase 46): SelectionLine is absent before first refresh and
// absent after refresh when no selection exists; present only after refresh
// when a selection exists.
//
// Implementation notes:
// - This adapter-local projection does not introduce any new framework.
// - It purposefully accepts primitive bounds (u32,u32) so callers can adapt
//   from existing SelectionView or other adapter-local read-only accessors.
// - Minimal surface: from_bounds, compose_from_optional_bounds, render.

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SelectionLine {
    pub text: String,
}

impl SelectionLine {
    /// Create a SelectionLine from explicit integer bounds.
    /// start_line/start_column and end_line/end_column are expressed as-is (no conversion).
    /// visible_in_window is optional presentation info; when true we append " (visible)".
    pub fn from_bounds(
        start_line: u32,
        start_column: u32,
        end_line: u32,
        end_column: u32,
        visible_in_window: bool,
    ) -> Self {
        let mut text = format!(
            "Selection: {}:{} -> {}:{}",
            start_line, start_column, end_line, end_column
        );
        if visible_in_window {
            text.push_str(" (visible)");
        }
        Self { text }
    }

    /// Compose a SelectionLine from an optional tuple of bounds.
    /// Returns None when the optional is None (i.e. no selection).
    /// This helper makes it easy for callers to map existing Option<SelectionView>
    /// into the small, adapter-local projection without introducing a trait.
    pub fn compose_from_optional_bounds(
        bounds: Option<(u32, u32, u32, u32, bool)>,
    ) -> Option<Self> {
        match bounds {
            Some((sl, sc, el, ec, vis)) => Some(Self::from_bounds(sl, sc, el, ec, vis)),
            None => None,
        }
    }

    /// Render the compact single-line representation for shell output.
    pub fn render(&self) -> String {
        self.text.clone()
    }
}
