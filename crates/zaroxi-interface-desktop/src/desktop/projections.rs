///
/// The functions are intentionally small and pure-ish: they read from the parent
/// DesktopComposition and return a shallow ViewportSummary. They purposely avoid
/// mutating composition state.
// The shared workspace-view DTOs live in zaroxi-application-workspace.
// Re-export them here so crate::desktop::projections::VisibleWindowBasic
// and similar paths continue to resolve.
pub use zaroxi_application_workspace::workspace_view::{
    ViewportAnchoring, ViewportSummary, VisibleWindowBasic,
};

/// Compute a small, read-only ViewportSummary from the DesktopComposition.
///
/// Preference order:
/// - If composition.metadata.visible_window (VisibleWindowBasic) is present, build summary from that.
/// - Otherwise inspect the presenter's InterfaceRenderableWindow snapshot and compute a deterministic summary.
///
/// This function preserves the exact heuristics previously present in desktop.rs:
/// - cursor_visible flag when a cursor-like span exists in the visible lines.
/// - anchoring heuristic: Top when cursor == top, Centered when cursor strictly inside, Unknown otherwise.
pub fn latest_viewport_summary(comp: &super::DesktopComposition) -> Option<ViewportSummary> {
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
                ViewportAnchoring::Top
            } else if cursor_line > top && cursor_line < bottom {
                ViewportAnchoring::Centered
            } else {
                ViewportAnchoring::Unknown
            }
        } else {
            ViewportAnchoring::Unknown
        };

        return Some(ViewportSummary {
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
            ViewportAnchoring::Top
        } else if cursor_line > top && cursor_line < bottom {
            ViewportAnchoring::Centered
        } else {
            ViewportAnchoring::Unknown
        }
    } else {
        ViewportAnchoring::Unknown
    };

    Some(ViewportSummary {
        top_visible_line: top,
        visible_line_count: visible_count,
        total_lines: total,
        cursor_visible,
        anchoring,
    })
}
