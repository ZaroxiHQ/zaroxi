/*!
Thin view module: Phase 2 editor-view seam.

Audit summary:
- Purpose: provide a strictly read-only, UI-oriented seam that exposes tiny helpers
  built on top of the application use-case read APIs (WorkspaceView). This module
  is intentionally small and documents the allowed scope of the "view" layer.
- Rationale: keep orchestration, session lifecycle, snapshot/checkpoint semantics,
  buffer identity and mutation inside application-workspace or core-editor-buffer.
  The view layer must only transform already-read state into harness-friendly DTOs
  or small presentation helpers (snippets, safe truncation, simple formatting).
- Validation: current harness behavior (querying buffer content via WorkspaceView)
  is preserved. No runtime or trait changes are required.

Allowed responsibilities for the view layer:
- Thin, deterministic helpers that transform read-only buffer content into UI/harness
  DTOs (e.g., snippet generation, safe truncation, small sanitizers).
- Small utilities that accept Option<String> (buffer content) and produce
  harness-friendly strings or compact previews.
- Non-stateful presentation helpers that do not perform IO, locking, or alter
  domain/application state.
- Documentation and markers clarifying the seam and public contracts for re-exports.

Forbidden responsibilities (must remain in core-editor-buffer or application-workspace):
- Any buffer identity, parsing, path↔id conversions, or mutation logic (these belong
  to core-editor-buffer).
- Session management, orchestration, snapshot composition, history/event recording,
  checkpoint serialization/deserialization, and durability (these belong to
  application-workspace).
- Introducing new traits for buffers, sessions, or history access (avoid expanding
  the port surface here).
- Any I/O, filesystem, or infra adapter wiring (belongs to infra/harness).
- Cursor movement, layout, selection transforms that imply editor engine behavior.

Tiny refactoring included to clarify seam:
- Add a single, pure helper `content_snippet` that is deterministic, Unicode-safe,
  and useful to the harness/UI when they need a compact preview of buffer content.
  This helper is intentionally minimal, pure (no IO), and kept in the view module
  to make presentation intent explicit. No trait or API surface changes are made.

Freeze note (Phase‑1 editor‑view seam):
- The view layer is frozen as a presentation-only seam. It may own only pure,
  deterministic helpers that transform read-only buffer content into compact,
  harness-friendly values (snippets, small sanitizers). The view must not be
  extended to include orchestration, mutation, lifecycle, or engine-like behaviors.
- Any future need for additional helpers must be reviewed against this freeze:
  does the helper operate solely on already-read state and produce a presentation
  artifact? If yes, keep in view. If it requires session, history, or buffer
  mutation knowledge, place it back in application-workspace or core-editor-buffer.

*/

/// Marker to make the view module non-empty and available for re-exports.
pub fn _crate_marker_view() {}

use crate::ports::EditorDocument;

/// Create a compact, harness-friendly snippet from optional buffer content.
///
/// - `content`: optional full buffer text as returned by a `WorkspaceView` read API.
/// - `max_chars`: maximum number of Unicode scalar values to include in the snippet.
/// Returns `None` when `content` is `None`, otherwise returns the possibly-truncated
/// string. Truncation is Unicode-safe and appends "..." when the content is longer
/// than `max_chars`.
///
/// This helper is intentionally pure, small, and presentation-focused. It does not
/// access sessions, perform IO, or touch application state.
pub fn content_snippet(content: Option<String>, max_chars: usize) -> Option<String> {
    content.map(|s| {
        // Count and take by chars to remain Unicode-safe.
        let char_count = s.chars().count();
        if char_count > max_chars {
            let snippet: String = s.chars().take(max_chars).collect();
            format!("{}...", snippet)
        } else {
            s
        }
    })
}

/// A single visible/projected line DTO.
///
/// - `line_number` is 1-based (matches common editor UI conventions).
/// - `text` is the line contents (no trimming performed).
/// - `is_cursor_line` marks whether the editor cursor is on this line.
/// - `cursor_column` when Some(_) is the 0-based character column of the cursor within this line.
/// - `selection_intersects` indicates whether the current selection overlaps this line.
/// - `selection_start_column`/`selection_end_column` give the (0-based, inclusive/exclusive)
///   character column range of the selection restricted to this line when `selection_intersects` is true.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct VisibleLine {
    pub line_number: usize,
    pub text: String,
    pub is_cursor_line: bool,
    /// 0-based cursor column within this line when the cursor is on this line.
    pub cursor_column: Option<usize>,
    /// Whether the session selection intersects this line.
    pub selection_intersects: bool,
    /// If selection_intersects, the selection start column for this line (0-based, inclusive).
    pub selection_start_column: Option<usize>,
    /// If selection_intersects, the selection end column for this line (0-based, exclusive).
    pub selection_end_column: Option<usize>,
}

/// A small window of visible lines projected from an EditorDocument.
///
/// - `top_line` is the 1-based line number of the first returned line.
/// - `total_lines` is the total number of lines in the document (0 when content absent).
/// - `lines` contains up to `window_size` VisibleLine entries.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct VisibleLinesWindow {
    pub top_line: usize,
    pub total_lines: usize,
    pub lines: Vec<VisibleLine>,
}

/// Project a deterministic, presentation-only window of visible lines from an EditorDocument.
///
/// - `doc`: reference to an EditorDocument read-model (pure input).
/// - `window_size`: number of lines to include in the window (preferred).
/// - `center_on_cursor`: when true attempt to center the window on the cursor line;
///    otherwise return the top-of-document window (starting at line 1).
///
/// Semantics (simple, deterministic):
/// - If no content is present `total_lines == 0` and `lines` is empty.
/// - Lines are computed using standard Rust `str::lines()` which is stable and
///   deterministic for presentation (no layout or wrapping is performed).
/// - Cursor line is derived from `doc.cursor.line` (0-based) and compared to the
///   zero-based index of the chosen lines to set `is_cursor_line`.
/// - `top_line` is 1-based and adjusted to ensure the returned window fits inside
///   the document (when document shorter than `window_size` start at 1).
pub fn project_visible_lines(doc: &EditorDocument, window_size: usize, center_on_cursor: bool) -> VisibleLinesWindow {
    // Defensive: empty window_size treated as zero -> return empty projection.
    if window_size == 0 {
        return VisibleLinesWindow { top_line: 1, total_lines: 0, lines: Vec::new() };
    }

    // Extract lines deterministically using `lines()` iterator.
    let lines_vec: Vec<String> = doc.content
        .as_ref()
        .map(|s| s.lines().map(|l| l.to_string()).collect::<Vec<_>>())
        .unwrap_or_default();

    let total = lines_vec.len();

    if total == 0 {
        return VisibleLinesWindow { top_line: 1, total_lines: 0, lines: Vec::new() };
    }

    let cursor_line = doc.cursor.line as usize;
    let cursor_col = doc.cursor.column as usize;

    // Normalize selection (compute ordered start/end positions) if present.
    let sel_opt = doc.selection.as_ref().map(|s| {
        let a_line = s.anchor.line as usize;
        let a_col = s.anchor.column as usize;
        let b_line = s.active.line as usize;
        let b_col = s.active.column as usize;
        if (a_line < b_line) || (a_line == b_line && a_col <= b_col) {
            (a_line, a_col, b_line, b_col)
        } else {
            (b_line, b_col, a_line, a_col)
        }
    });

    // Determine start index (0-based) for the window.
    let mut start = if center_on_cursor {
        let half = window_size / 2;
        if cursor_line > half {
            cursor_line.saturating_sub(half).saturating_add(1)
        } else {
            0
        }
    } else {
        0
    };

    // Clamp start so that window fits within document.
    if start + window_size > total {
        if total >= window_size {
            start = total - window_size;
        } else {
            start = 0;
        }
    }

    let end = std::cmp::min(start + window_size, total);

    let mut out: Vec<VisibleLine> = Vec::with_capacity(end - start);
    for (idx, line) in lines_vec.iter().enumerate().take(end).skip(start) {
        // Compute selection intersection for this line if any.
        let mut selection_intersects = false;
        let mut sel_start_col: Option<usize> = None;
        let mut sel_end_col: Option<usize> = None;

        if let Some((start_line, start_col, end_line, end_col)) = sel_opt {
            if idx >= start_line && idx <= end_line {
                selection_intersects = true;
                let line_char_count = line.chars().count();
                let s = if idx == start_line { start_col } else { 0 };
                let e = if idx == end_line { end_col } else { line_char_count };
                // clamp to line bounds
                let s = std::cmp::min(s, line_char_count);
                let e = std::cmp::min(e, line_char_count);
                sel_start_col = Some(s);
                sel_end_col = Some(e);
            }
        }

        let cursor_on_line = idx == cursor_line;
        let cursor_column = if cursor_on_line { Some(cursor_col) } else { None };

        out.push(VisibleLine {
            line_number: idx + 1,
            text: line.clone(),
            is_cursor_line: cursor_on_line,
            cursor_column,
            selection_intersects,
            selection_start_column: sel_start_col,
            selection_end_column: sel_end_col,
        });
    }

    VisibleLinesWindow { top_line: start + 1, total_lines: total, lines: out }
}
 
/// Project visible lines given an explicit stored viewport state.
///
/// Semantics:
/// - If `viewport.center_cursor == true` then prefer centering the cursor similar to
///   the previous centering policy; otherwise use `viewport.top_line` as authoritative
///   (clamped to the document size).
pub fn project_visible_lines_for_viewport(doc: &EditorDocument, viewport: &crate::ports::ViewportState) -> VisibleLinesWindow {
    // Defensive: empty window_height treated as zero -> return empty projection.
    if viewport.window_height == 0 {
        return VisibleLinesWindow { top_line: 1, total_lines: 0, lines: Vec::new() };
    }
 
    let lines_vec: Vec<String> = doc.content
        .as_ref()
        .map(|s| s.lines().map(|l| l.to_string()).collect::<Vec<_>>())
        .unwrap_or_default();
 
    let total = lines_vec.len();
 
    if total == 0 {
        return VisibleLinesWindow { top_line: 1, total_lines: 0, lines: Vec::new() };
    }
 
    let cursor_line = doc.cursor.line as usize;
    let cursor_col = doc.cursor.column as usize;

    // Normalize selection (compute ordered start/end positions) if present.
    let sel_opt = doc.selection.as_ref().map(|s| {
        let a_line = s.anchor.line as usize;
        let a_col = s.anchor.column as usize;
        let b_line = s.active.line as usize;
        let b_col = s.active.column as usize;
        if (a_line < b_line) || (a_line == b_line && a_col <= b_col) {
            (a_line, a_col, b_line, b_col)
        } else {
            (b_line, b_col, a_line, a_col)
        }
    });
 
    // Compute start using viewport state.
    let mut start = if viewport.center_cursor {
        // Centering policy similar to earlier function.
        let half = viewport.window_height / 2;
        if cursor_line > half {
            cursor_line.saturating_sub(half).saturating_add(1)
        } else {
            0
        }
    } else {
        // Convert 1-based top_line to 0-based start, clamp to valid range.
        if viewport.top_line == 0 {
            0
        } else {
            viewport.top_line.saturating_sub(1)
        }
    };
 
    // Clamp start so that window fits within document.
    if start + viewport.window_height > total {
        if total >= viewport.window_height {
            start = total - viewport.window_height;
        } else {
            start = 0;
        }
    }
 
    let end = std::cmp::min(start + viewport.window_height, total);
 
    let mut out: Vec<VisibleLine> = Vec::with_capacity(end - start);
    for (idx, line) in lines_vec.iter().enumerate().take(end).skip(start) {
        // Compute selection intersection for this line if any.
        let mut selection_intersects = false;
        let mut sel_start_col: Option<usize> = None;
        let mut sel_end_col: Option<usize> = None;

        if let Some((start_line, start_col, end_line, end_col)) = sel_opt {
            if idx >= start_line && idx <= end_line {
                selection_intersects = true;
                let line_char_count = line.chars().count();
                let s = if idx == start_line { start_col } else { 0 };
                let e = if idx == end_line { end_col } else { line_char_count };
                // clamp to line bounds
                let s = std::cmp::min(s, line_char_count);
                let e = std::cmp::min(e, line_char_count);
                sel_start_col = Some(s);
                sel_end_col = Some(e);
            }
        }

        let cursor_on_line = idx == cursor_line;
        let cursor_column = if cursor_on_line { Some(cursor_col) } else { None };

        out.push(VisibleLine {
            line_number: idx + 1,
            text: line.clone(),
            is_cursor_line: cursor_on_line,
            cursor_column,
            selection_intersects,
            selection_start_column: sel_start_col,
            selection_end_column: sel_end_col,
        });
    }
 
    VisibleLinesWindow { top_line: start + 1, total_lines: total, lines: out }
}
