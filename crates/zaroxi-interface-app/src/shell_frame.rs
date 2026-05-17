#![doc = "Shell frame view model (application-facing wrapper around the desktop ShellFrameModel)."]

use zaroxi_interface_desktop::projections::shell_frame::ShellFrameModel;
use zaroxi_interface_desktop::{TextView, SelectionView};

/// Tiny, read-only wrapper exposed to the application layer that owns a
/// ShellFrameModel received from the desktop composition and exposes a
/// minimal semantic API for downstream consumers.
///
/// Lifecycle rule: absent when no ShellFrameModel has been provided; present
/// when the wrapper holds an owned ShellFrameModel.
#[derive(Debug, Clone, Default)]
pub struct ShellFrameViewModel {
    frame: Option<ShellFrameModel>,
}

impl ShellFrameViewModel {
    /// Create an empty view model (no frame present).
    pub fn new() -> Self {
        Self { frame: None }
    }

    /// Set (replace) the owned ShellFrameModel received from the desktop layer.
    pub fn set(&mut self, frame: ShellFrameModel) {
        self.frame = Some(frame);
    }

    /// Clear the held ShellFrameModel (make the view model absent).
    pub fn clear(&mut self) {
        self.frame = None;
    }

    /// Whether a frame is currently present.
    pub fn is_present(&self) -> bool {
        self.frame.is_some()
    }

    /// Read-only accessor for the compact viewport summary (semantic string).
    pub fn viewport(&self) -> Option<String> {
        self.frame.as_ref().and_then(|f| f.viewport_summary.clone())
    }

    /// Read-only accessor for the status text when present.
    pub fn status_text(&self) -> Option<String> {
        self.frame.as_ref().and_then(|f| f.status_text.clone())
    }

    /// Read-only accessor for the rendered shell chrome if available.
    pub fn shell_chrome(&self) -> Option<String> {
        self.frame.as_ref().and_then(|f| f.shell_chrome.clone())
    }

    /// Read-only accessor for the last command line rendered string, if any.
    pub fn last_command(&self) -> Option<String> {
        self.frame.as_ref().and_then(|f| f.last_command.clone())
    }

    /// Borrow the active text view when present (semantic, read-only).
    pub fn active_text_view(&self) -> Option<&TextView> {
        self.frame.as_ref().and_then(|f| f.active_text_view.as_ref())
    }

    /// Borrow the selection view when present (semantic, read-only).
    pub fn selection_view(&self) -> Option<&SelectionView> {
        self.frame.as_ref().and_then(|f| f.selection_view.as_ref())
    }
}
