use super::*;

/// Consistency helpers extracted from the parent `desktop` module.
///
/// This module contains the DesktopConsistencyReport type and a single helper
/// that computes a small, read-only consistency report from a DesktopComposition.
/// The implementation preserves the exact checks and heuristics previously
/// present in `desktop.rs`.
///
/// Note: the module intentionally reads `&super::DesktopComposition` and returns
/// the local `DesktopConsistencyReport` type which is re-exported by the parent
/// module (`pub use consistency::DesktopConsistencyReport;`) to preserve the
/// previous public API.

/// Small, shell-facing consistency report for a DesktopComposition.
///
/// Purpose:
/// - Provide a tiny read-only report derived from existing composition fields.
/// - Allow harnesses and simple shells to assert basic invariants without
///   introducing a validation/telemetry subsystem.
/// - Keep semantics conservative: when data is absent we consider the check
///   satisfied unless an inconsistency can be observed.
///
/// Checks included:
/// - status_present_matches_summary: whether summary.status.is_some() aligns with `composition.status`.
/// - active_buffer_matches_details: when metadata exposes an active_buffer, does the active-buffer-details projection match it?
/// - active_buffer_in_opened_buffers: when opened_buffers is non-empty and active_buffer present, is the active_buffer one of the opened_buffers?
/// - presenter_window_matches_status: whether the presenter's window presence aligns with the status.has_render_window flag.
#[derive(Clone, Debug)]
pub struct DesktopConsistencyReport {
    /// Whether the status presence recorded in latest_summary() matches `composition.status` presence.
    pub status_present_matches_summary: bool,
    /// When metadata exposes an active buffer, whether that aligns with active-buffer-details buffer id.
    pub active_buffer_matches_details: bool,
    /// When opened_buffers is non-empty and an active buffer exists, whether the active buffer is among opened_buffers.
    pub active_buffer_in_opened_buffers: bool,
    /// Whether the presenter's window presence equals status.has_render_window (when status present).
    pub presenter_window_matches_status: bool,
    /// Overall ok (all checks true).
    pub overall_ok: bool,
}

/// Compute the consistency report for a DesktopComposition.
///
/// This function is a straightforward extraction of the logic previously living
/// inside DesktopComposition::latest_consistency_report. It intentionally does
/// not mutate the composition and performs only shallow, deterministic checks.
pub fn latest_consistency_report(comp: &super::DesktopComposition) -> DesktopConsistencyReport {
    // 1) summary status presence vs actual status presence
    let summary_has_status = comp.latest_summary().and_then(|s| s.status).is_some();
    let status_present = comp.status.is_some();
    let status_present_matches_summary = summary_has_status == status_present;

    // 2) active buffer alignment with active-buffer-details
    let meta_active = comp.metadata.as_ref().and_then(|m| m.active_buffer.clone());
    let abd_opt = comp.latest_active_buffer_details();
    let active_buffer_matches_details = if meta_active.is_some() {
        match abd_opt {
            Some(abd) => abd.buffer_id == meta_active.unwrap(),
            None => false,
        }
    } else {
        // Nothing asserted by metadata -> treat as OK
        true
    };

    // 3) active buffer is among opened buffers when opened list non-empty
    let active_buffer_in_opened_buffers = match &comp.metadata {
        Some(meta) => {
            if meta.opened_buffers.is_empty() {
                true
            } else {
                match meta.active_buffer.clone() {
                    Some(act) => meta.opened_buffers.iter().any(|i| i.buffer_id == act),
                    None => true,
                }
            }
        }
        None => true,
    };

    // 4) presenter window presence aligns with status.has_render_window
    let presenter_has = comp.presenter.latest().is_some();
    let status_has_render = comp.status.as_ref().map(|s| s.has_render_window).unwrap_or(false);
    let presenter_window_matches_status = presenter_has == status_has_render;

    let overall_ok = status_present_matches_summary
        && active_buffer_matches_details
        && active_buffer_in_opened_buffers
        && presenter_window_matches_status;

    DesktopConsistencyReport {
        status_present_matches_summary,
        active_buffer_matches_details,
        active_buffer_in_opened_buffers,
        presenter_window_matches_status,
        overall_ok,
    }
}
