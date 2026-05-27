///
/// The functions are intentionally small and pure-ish: they read from the parent
/// DesktopComposition and return a shallow ViewportSummary. They purposely avoid
/// mutating composition state.

/// Small basic visible-window projection derived from WorkspaceView VisibleLinesWindow.
/// This representation is intentionally tiny and decoupled from presenter view_adapter types.
/// It is a best-effort, read-only projection populated during `refresh_with_service` when
/// the caller's WorkspaceView can provide VisibleLinesWindow data. Consumers prefer this
/// projection over presenter snapshots when present.
#[derive(Clone, Debug)]
pub struct VisibleWindowBasic {
    /// 1-based top visible line number.
    pub top_line: usize,
    /// Total number of lines in the buffer/document.
    pub total_lines: usize,
    /// Visible lines' textual content, in order from `top_line`.
    pub lines: Vec<String>,
    /// Optional 1-based cursor line if present in the visible window.
    pub cursor_line: Option<usize>,
    /// Optional 0-based cursor column within the cursor line.
    pub cursor_column: Option<usize>,
    /// Whether any selection intersects the visible window.
    pub selection_present: bool,
}

/// Compute a small, read-only ViewportSummary from the DesktopComposition.
///
/// Preference order:
/// - If composition.metadata.visible_window (VisibleWindowBasic) is present, build summary from that.
/// - Otherwise inspect the presenter's InterfaceRenderableWindow snapshot and compute a deterministic summary.
///
/// This function preserves the exact heuristics previously present in desktop.rs:
/// - cursor_visible flag when a cursor-like span exists in the visible lines.
/// - anchoring heuristic: Top when cursor == top, Centered when cursor strictly inside, Unknown otherwise.
pub fn latest_viewport_summary(comp: &super::DesktopComposition) -> Option<super::ViewportSummary> {
    // Prefer WorkspaceView-provided visible-window when available.
    if let Some(vw) = comp.metadata.as_ref().and_then(|m| m.visible_window.clone()) {
        let top = vw.top_line;
        let visible_count = vw.lines.len();
        let total = vw.total_lines;

        let cursor_visible = vw.cursor_line.is_some();
        let cursor_line_opt = vw.cursor_line;

        let anchoring = if let Some(cursor_line) = cursor_line_opt {
            let bottom = top.saturating_add(visible_count.saturating_sub(1));
            if cursor_line == top {
                super::ViewportAnchoring::Top
            } else if cursor_line > top && cursor_line < bottom {
                super::ViewportAnchoring::Centered
            } else {
                super::ViewportAnchoring::Unknown
            }
        } else {
            super::ViewportAnchoring::Unknown
        };

        return Some(super::ViewportSummary {
            top_visible_line: top,
            visible_line_count: visible_count,
            total_lines: total,
            cursor_visible,
            anchoring,
        });
    }

    // Fallback: use presenter's snapshot.
    let win = comp.presenter.latest()?;
    let top = win.top_line;
    let visible_count = win.lines.len();
    let total = win.total_lines;

    let mut cursor_visible = false;
    let mut cursor_line_opt: Option<usize> = None;
    for line in win.lines.iter() {
        for sp in line.spans.iter() {
            match sp.kind {
                crate::view_adapter::InterfaceSpanKind::Cursor
                | crate::view_adapter::InterfaceSpanKind::SelectionCursor => {
                    cursor_visible = true;
                    cursor_line_opt = Some(line.line_number);
                    break;
                }
                _ => {}
            }
        }
        if cursor_visible {
            break;
        }
    }

    let anchoring = if let Some(cursor_line) = cursor_line_opt {
        let bottom = top.saturating_add(visible_count.saturating_sub(1));
        if cursor_line == top {
            super::ViewportAnchoring::Top
        } else if cursor_line > top && cursor_line < bottom {
            super::ViewportAnchoring::Centered
        } else {
            super::ViewportAnchoring::Unknown
        }
    } else {
        super::ViewportAnchoring::Unknown
    };

    Some(super::ViewportSummary {
        top_visible_line: top,
        visible_line_count: visible_count,
        total_lines: total,
        cursor_visible,
        anchoring,
    })
}
