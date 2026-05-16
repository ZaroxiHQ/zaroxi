 // Thin view module: Phase 2 editor-view seam.
 //
 // Purpose: export tiny, UI-oriented helpers and markers. The primary API surface
 // is the `WorkspaceView` trait in `ports.rs` which is implemented by the
 // application orchestrator (usecases::WorkspaceOrchestrator).
 //
 // Keep this file minimal to avoid scope creep.

 /// Marker to make the view module non-empty and available for re-exports.
 pub fn _crate_marker_view() {}
