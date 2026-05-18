#![doc = "Shell frame view model (application-facing wrapper around the desktop ShellFrameModel)."]

// The application crate intentionally does not depend on the desktop crate to
// avoid upward dependency cycles. For compile-time stability we define a tiny,
// local, minimal subset of the desktop projection types used by this module.
// These types mirror the fields consumed by the application-facing wrapper
// and keep the application layer independent from the desktop implementation.
//
// NOTE: These are lightweight, compile-time-only shapes used to carry semantic
// data across the app surface; they are NOT the authoritative desktop types.
#[derive(Debug, Clone, Default)]
pub struct ShellFrameModel {
    pub viewport_summary: Option<String>,
    pub status_text: Option<String>,
    pub shell_chrome: Option<String>,
    pub last_command: Option<String>,
    pub active_text_view: Option<TextView>,
    pub selection_view: Option<SelectionView>,
}

#[derive(Debug, Clone)]
pub struct TextView {
    pub top_line: usize,
    pub total_lines: usize,
    pub lines: Vec<String>,
    pub cursor_line: Option<usize>,
    pub cursor_column: Option<usize>,
}

#[derive(Debug, Clone)]
pub struct SelectionView {
    pub start: Position,
    pub end: Position,
}

#[derive(Debug, Clone)]
pub struct Position {
    pub line: usize,
    pub column: usize,
}

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
    pub fn set<F: Into<ShellFrameModel>>(&mut self, frame: F) {
        self.frame = Some(frame.into());
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

impl ShellFrameViewModel {
    /// Convert this application-facing ShellFrameViewModel into the engine-facing,
    /// non-visual EngineShellViewInput.
    ///
    /// The conversion preserves only semantic, non-visual pieces:
    /// - visible text lines and simple document metrics,
    /// - optional cursor line/column,
    /// - optional selection bounds (start/end line+column),
    /// - viewport summary, status text, shell chrome text, last command,
    /// - a small ai_present boolean (false here; presenter may set true when appropriate).
    ///
    /// If no active text view is present the function returns EngineShellViewInput::absent().
    pub fn to_engine_input(&self) -> zaroxi_core_engine_view::EngineShellViewInput {
        // Use active_text_view as the mandatory piece for a meaningful input.
        if let Some(tv) = self.active_text_view() {
            let selection = self.selection_view().map(|s| {
                zaroxi_core_engine_view::EngineSelection {
                    start_line: s.start.line as u32,
                    start_column: s.start.column as u32,
                    end_line: s.end.line as u32,
                    end_column: s.end.column as u32,
                }
            });

            zaroxi_core_engine_view::EngineShellViewInput {
                top_line: tv.top_line as u32,
                total_lines: tv.total_lines as u32,
                lines: tv.lines.clone(),
                cursor_line: tv.cursor_line.map(|c| c as u32),
                cursor_column: tv.cursor_column.map(|c| c as u32),
                selection,
                viewport_summary: self.viewport(),
                status_text: self.status_text(),
                shell_chrome: self.shell_chrome(),
                last_command: self.last_command(),
                // Interface-level code knows whether an AI projection is present; for now keep conservative.
                ai_present: false,
            }
        } else {
            zaroxi_core_engine_view::EngineShellViewInput::absent()
        }
    }
}

#[cfg(test)]
mod desktop_interop {
    use super::*;
    use zaroxi_interface_desktop::projections::shell_frame as desktop;

    impl From<desktop::ShellFrameModel> for ShellFrameModel {
        fn from(d: desktop::ShellFrameModel) -> Self {
            ShellFrameModel {
                viewport_summary: d.viewport_summary,
                status_text: d.status_text,
                shell_chrome: d.shell_chrome,
                last_command: d.last_command,
                active_text_view: d.active_text_view.map(|t| TextView {
                    top_line: t.top_line,
                    total_lines: t.total_lines,
                    lines: t.lines,
                    cursor_line: t.cursor_line,
                    cursor_column: t.cursor_column,
                }),
                selection_view: d.selection_view.map(|s| SelectionView {
                    start: Position { line: s.start.line, column: s.start.column },
                    end: Position { line: s.end.line, column: s.end.column },
                }),
            }
        }
    }
}
