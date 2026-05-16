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
