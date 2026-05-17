/*!
Tiny presenter state for the active editor renderable window.

Responsibilities (Phase 12 minimal):
- Hold the latest InterfaceRenderableWindow fetched from the application via the
  adapter seam (zaroxi-interface-desktop::view_adapter::fetch_renderable_window).
- Expose a tiny, read-only accessor returning a cloned snapshot suitable for
  simple presentation in the harness or other interface glue.
- Provide an async refresh method that requests the latest renderable window
  from the application and updates the internal snapshot.

Constraints:
- Read-only oriented: presenter does not compute spans or visibility; it only
  stores and exposes the already-projected model produced by application code.
- No UI framework, no rendering, no styling, no layout. Minimal glue only.
*/

use std::sync::Arc;

use crate::view_adapter::{InterfaceRenderableWindow, fetch_renderable_window};
use zaroxi_application_workspace::ports::{WorkspaceView, SessionId};

/// Very small presenter that stores the last fetched renderable window.
///
/// The presenter owns an optional snapshot of the renderable window and provides
/// a minimal API:
/// - Presenter::new()
/// - Presenter::refresh(view, session_id) -> async Result
/// - Presenter::latest() -> Option<InterfaceRenderableWindow> (clone snapshot)
#[derive(Clone, Debug, Default)]
pub struct Presenter {
    window: Option<InterfaceRenderableWindow>,
}

impl Presenter {
    /// Create a new empty presenter.
    pub fn new() -> Self {
        Self { window: None }
    }

    /// Return the last fetched renderable window, cloned for caller convenience.
    /// This accessor is intentionally read-only and returns an owned snapshot so
    /// callers need not borrow the presenter while rendering/printing.
    pub fn latest(&self) -> Option<InterfaceRenderableWindow> {
        self.window.clone()
    }

    /// Refresh the presenter's snapshot by requesting the current renderable window
    /// from the application via the thin adapter seam.
    ///
    /// This function performs no local projection or mutation logic — it merely
    /// delegates to fetch_renderable_window and stores the returned snapshot.
    pub async fn refresh(&mut self, view: Arc<dyn WorkspaceView>, session_id: SessionId) -> Result<(), String> {
        let win = fetch_renderable_window(view, session_id).await?;
        self.window = Some(win);
        Ok(())
    }
}
