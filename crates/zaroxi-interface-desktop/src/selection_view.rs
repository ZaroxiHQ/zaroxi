/*!
Tiny, read-only SelectionView for the active editor surface.

Purpose (Phase 33 minimal):
- Provide a tiny, shell-facing read-only projection answering:
    - whether a selection exists,
    - the selection start and end positions (1-based line, 0-based column),
    - whether any part of the selection is visible in the current InterfaceRenderableWindow.
- Compose on top of the existing InterfaceRenderableWindow produced by the presenter/adapter seam.
- No mutation, no styling, no rendering — purely a deterministic projection for shells/harnesses.

Design notes:
- We treat any InterfaceRenderSpan with kind Selection or SelectionCursor as part of the selection.
- Start is the earliest (line,column) encountered; end is the latest (line,column) encountered.
- Visibility is true when the selection intersects the window's reported line range.
*/

use crate::view_adapter::{InterfaceRenderableWindow, InterfaceSpanKind};

/// Selection position: 1-based line number, 0-based character column.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SelectionPosition {
    pub line: usize,
    pub column: usize,
}

/// Tiny read-only selection projection.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SelectionView {
    /// Selection start position (inclusive).
    pub start: SelectionPosition,
    /// Selection end position (exclusive).
    pub end: SelectionPosition,
    /// Whether any part of the selection intersects the current visible window.
    pub visible_in_window: bool,
}

impl SelectionView {
    /// Build a SelectionView from a DesktopComposition by reusing the presenter's
    /// latest InterfaceRenderableWindow. Returns None when no selection is present.
    pub fn from_composition(comp: &crate::desktop::DesktopComposition) -> Option<Self> {
        comp.latest_window().and_then(|w| Self::from_window(&w))
    }

    /// Build a SelectionView from an InterfaceRenderableWindow.
    ///
    /// Returns None when no selection spans are present in the window.
    pub fn from_window(win: &InterfaceRenderableWindow) -> Option<Self> {
        let mut found = false;
        let mut start = SelectionPosition { line: 0, column: 0 };
        let mut end = SelectionPosition { line: 0, column: 0 };

        for line in win.lines.iter() {
            for sp in line.spans.iter() {
                match sp.kind {
                    InterfaceSpanKind::Selection | InterfaceSpanKind::SelectionCursor => {
                        let ln = line.line_number;
                        let sc = sp.start_col;
                        let ec = sp.end_col;

                        if !found {
                            start = SelectionPosition { line: ln, column: sc };
                            end = SelectionPosition { line: ln, column: ec };
                            found = true;
                        } else {
                            // Update start when earlier
                            if (ln < start.line) || (ln == start.line && sc < start.column) {
                                start = SelectionPosition { line: ln, column: sc };
                            }
                            // Update end when later
                            if (ln > end.line) || (ln == end.line && ec > end.column) {
                                end = SelectionPosition { line: ln, column: ec };
                            }
                        }
                    }
                    _ => {}
                }
            }
        }

        if !found {
            return None;
        }

        // Determine visible range for the window (1-based lines).
        let window_top = win.top_line;
        let window_bottom = window_top + win.lines.len().saturating_sub(1);

        // Selection is visible if it intersects the visible line range.
        let visible = !(end.line < window_top || start.line > window_bottom);

        Some(SelectionView { start, end, visible_in_window: visible })
    }
}
