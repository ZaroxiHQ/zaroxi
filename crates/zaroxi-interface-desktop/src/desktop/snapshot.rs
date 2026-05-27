/*!
Tiny helper that builds a ShellSnapshot from a DesktopComposition.

This module contains a single pure/derivational function that mirrors the
previous inline implementation in `desktop.rs`. It is intentionally very small
and read-only: it only calls existing accessors on `DesktopComposition` and
packages their results into the `ShellSnapshot` DTO.
*/

/// Build a ShellSnapshot from the given DesktopComposition.
///
/// Returns None when no shell context is available (mirrors the previous
/// semantics that required a context to produce a snapshot).
pub fn latest_shell_snapshot(comp: &super::DesktopComposition) -> Option<super::ShellSnapshot> {
    // Require at least the shell context to produce a snapshot.
    let ctx = comp.latest_shell_context()?;
    let active_document = comp.latest_active_document_summary();
    let viewport = comp.latest_viewport_summary();
    let ai_summary = comp.latest_ai_projection_summary();
    let opened_buffers = comp.latest_opened_buffers_summary();

    Some(super::ShellSnapshot {
        context: ctx,
        active_document,
        viewport,
        ai_summary,
        opened_buffers,
    })
}
