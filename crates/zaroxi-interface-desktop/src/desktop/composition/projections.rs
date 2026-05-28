/*!
Projection assembly helpers for DesktopComposition.

This module implements small, pure helpers that derive shell-facing projections
from the stored DesktopComposition. These were migrated from the original
monolithic composition module and are intended to be pure/read-only.
*/

/// Construct an ActiveDocumentSummary projection from composition state.
///
/// This mirrors the previous logic: prefer WorkspaceView-provided visible-window
/// projection (stored in metadata.visible_window) and otherwise inspect the presenter's
/// latest InterfaceRenderableWindow snapshot.
pub fn latest_active_document_summary(
    comp: &super::DesktopComposition,
) -> Option<super::ActiveDocumentSummary> {
    let meta = comp.metadata.as_ref()?;
    let abd = meta.active_buffer_details.clone()?;

    // Prefer a direct visible-window projection from WorkspaceView when available;
    // otherwise fall back to the presenter's latest renderable window.
    let vw_opt = comp.metadata.as_ref().and_then(|m| m.visible_window.clone());
    let mut cursor_line: Option<usize> = None;
    let mut cursor_column: Option<usize> = None;
    let mut selection_present = false;
    let mut current_line_snippet: Option<String> = None;

    if let Some(vw) = vw_opt {
        // Use the basic visible-window projection to fill cursor/selection/snippet.
        cursor_line = vw.cursor_line;
        cursor_column = vw.cursor_column;
        selection_present = vw.selection_present;

        // Determine a reasonable current-line snippet: prefer cursor line, else top_line.
        let snippet_line_no = cursor_line.unwrap_or(vw.top_line);
        // Convert snippet_line_no into an index in vw.lines (lines stored from top_line).
        if snippet_line_no >= vw.top_line {
            let idx = snippet_line_no.saturating_sub(vw.top_line);
            if let Some(line_text) = vw.lines.get(idx) {
                let snippet: String = line_text.chars().take(120).collect();
                current_line_snippet = Some(snippet);
            }
        }
    } else {
        // Fallback: inspect the presenter's InterfaceRenderableWindow spans as before.
        let win_opt = comp.presenter.latest();
        if let Some(win) = win_opt {
            // Scan spans to find a cursor or selection.
            for line in win.lines.iter() {
                for sp in line.spans.iter() {
                    match sp.kind {
                        crate::view_adapter::InterfaceSpanKind::SelectionCursor
                        | crate::view_adapter::InterfaceSpanKind::Cursor => {
                            cursor_line = Some(line.line_number);
                            cursor_column = Some(sp.start_col);
                        }
                        crate::view_adapter::InterfaceSpanKind::Selection => {
                            selection_present = true;
                        }
                        _ => {}
                    }
                    // stop early if we found both
                    if cursor_line.is_some() && selection_present {
                        break;
                    }
                }
                if cursor_line.is_some() && selection_present {
                    break;
                }
            }

            // If we didn't detect selection while scanning for cursor, do a secondary lightweight check.
            if !selection_present {
                'outer2: for line in win.lines.iter() {
                    for sp in line.spans.iter() {
                        if let crate::view_adapter::InterfaceSpanKind::Selection = sp.kind {
                            selection_present = true;
                            break 'outer2;
                        }
                    }
                }
            }

            // Determine a reasonable current-line snippet: prefer cursor line, else top_line.
            // NOTE: Only include user-facing text in the snippet. Exclude presenter marker spans
            // (cursor/selection/debug) so the produced snippet is clean and free of inline
            // debug markers like "|^|" or "|/|/" that some renderers may inject.
            let snippet_line_no = cursor_line.unwrap_or(win.top_line);
            if let Some(l) = win.lines.iter().find(|l| l.line_number == snippet_line_no) {
                let mut s = String::new();
                // Only include "text" spans in the plain snippet. Exclude cursor/selection/debug spans
                // so that the resulting snippet remains clean and user-facing.
                for sp in l.spans.iter() {
                    match sp.kind {
                        crate::view_adapter::InterfaceSpanKind::SelectionCursor
                        | crate::view_adapter::InterfaceSpanKind::Cursor
                        | crate::view_adapter::InterfaceSpanKind::Selection => {
                            // skip marker spans from presenter (cursor/selection); these are surfaced
                            // separately via cursor_line/cursor_column/selection_present.
                        }
                        _ => {
                            s.push_str(&sp.text);
                        }
                    }
                }
                // Truncate to 120 Unicode scalars for compactness.
                let snippet: String = s.chars().take(120).collect();
                current_line_snippet = Some(snippet);
            }
        }
    }

    Some(super::ActiveDocumentSummary {
        buffer_id: meta.active_buffer.clone(),
        display: abd.display,
        line_count: abd.line_count,
        cursor_line,
        cursor_column,
        selection_present,
        current_line_snippet,
    })
}

/// Build the opened-buffers summary projection.
///
/// This function is pure and reads only composition metadata.
pub fn latest_opened_buffers_summary(
    comp: &super::DesktopComposition,
) -> super::OpenedBuffersSummary {
    if let Some(meta) = &comp.metadata {
        // Build per-item summaries. Prefer line_count from active_buffer_details when it matches.
        let mut items: Vec<super::OpenedBufferItemSummary> =
            Vec::with_capacity(meta.opened_buffers.len());
        for it in meta.opened_buffers.iter() {
            // Try to obtain line_count from active_buffer_details when it matches the buffer id.
            let mut line_count: usize = 0;
            if let Some(abd) = &meta.active_buffer_details {
                if abd.buffer_id == it.buffer_id {
                    line_count = abd.line_count;
                }
            }
            items.push(super::OpenedBufferItemSummary {
                buffer_id: it.buffer_id.clone(),
                display: it.display.clone(),
                line_count,
                active: it.active,
            });
        }
        super::OpenedBuffersSummary {
            count: meta.opened_buffer_count,
            items,
            active: meta.active_buffer.clone(),
        }
    } else {
        super::OpenedBuffersSummary { count: 0, items: Vec::new(), active: None }
    }
}

/// Build a small ShellContext projection (read-only).
pub fn latest_shell_context(comp: &super::DesktopComposition) -> Option<super::ShellContext> {
    // Mirror latest_summary presence semantics: require at least one refresh to return a context.
    if comp.revision == 0 && comp.metadata.is_none() && comp.status.is_none() {
        return None;
    }

    // Determine active_display: prefer active_buffer_details.display, fall back to opened_buffers item display.
    let active_display = comp.metadata.as_ref().and_then(|m| {
        m.active_buffer_details
            .as_ref()
            .and_then(|d| d.display.clone())
            .or_else(|| m.opened_buffers.iter().find(|i| i.active).and_then(|i| i.display.clone()))
    });

    let has_ai = comp.metadata.as_ref().and_then(|m| m.ai_projection.as_ref()).is_some();

    Some(super::ShellContext {
        active_buffer: comp.metadata.as_ref().and_then(|m| m.active_buffer.clone()),
        active_display,
        latest_revision: comp.revision,
        latest_refresh_reason: comp.metadata.as_ref().and_then(|m| m.refresh_reason.clone()),
        has_ai_projection: has_ai,
        last_command_line: comp.metadata.as_ref().and_then(|m| m.last_command_line.clone()),
    })
}
