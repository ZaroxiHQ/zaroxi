use crate::events::types::{FrameModel, Key, Mouse, Region, RenderViewModel, Resize, UiEvent};

/// Minimal event router that converts incoming UiEvent values into calls on
/// the UI-facing models (`FrameModel` and `RenderViewModel`).
///
/// The router keeps a notion of the currently active region (chrome/content/status)
/// and routes events to the active region. The router intentionally does not
/// perform engine-level mutations; it only calls into UI-layer models.
#[derive(Debug, Default)]
pub struct EventRouter {
    pub active_region: Region,
}

impl EventRouter {
    /// Create a new router with Content as the default active region.
    pub fn new() -> Self {
        Self { active_region: Region::Content }
    }

    /// Set the active region (e.g. when layout changes focus).
    pub fn set_active_region(&mut self, region: Region) {
        self.active_region = region;
    }

    /// Handle a high-level UiEvent by routing it to the provided UI models.
    ///
    /// - Key events affecting navigation (ArrowDown/ArrowUp) are routed to
    ///   the FrameModel when the Content region is active.
    /// - Enter activates the currently focused buffer (UI-level action).
    /// - Resize and Mouse events are routed to the RenderViewModel to allow
    ///   it to update active sections or other UI-only presentation state.
    pub fn handle_ui_event(
        &mut self,
        event: UiEvent,
        frame: &mut dyn FrameModel,
        view: &mut dyn RenderViewModel,
    ) {
        match event {
            UiEvent::Key(k) => self.handle_key(k, frame, view),
            UiEvent::Mouse(m) => self.handle_mouse(m, frame, view),
            UiEvent::Resize(r) => self.handle_resize(r, view),
        }
    }

    fn handle_key(&mut self, key: Key, frame: &mut dyn FrameModel, view: &mut dyn RenderViewModel) {
        match key {
            Key::ArrowDown => {
                if self.active_region == Region::Content {
                    frame.move_focus_down();
                } else {
                    // Non-content regions may change which section is active in the view.
                    view.set_active_section(self.active_region.clone());
                }
            }
            Key::ArrowUp => {
                // For symmetry; UI models may implement move_focus_up if desired.
                // We'll reuse move_focus_down as a placeholder for UI changes in this phase.
                if self.active_region == Region::Content {
                    // no-op by default; concrete FrameModel can decide behavior.
                    frame.move_focus_down();
                } else {
                    view.set_active_section(self.active_region.clone());
                }
            }
            Key::Enter => {
                // Activate the buffer under the cursor in UI state.
                frame.activate_current_buffer();
            }
            Key::Char(c) => {
                // Example: typing could change the active buffer name in UI.
                frame.set_active_buffer(c.to_string());
            }
            _ => {
                // Other keys are ignored at this minimal stage.
            }
        }
    }

    fn handle_mouse(
        &mut self,
        mouse: Mouse,
        _frame: &mut dyn FrameModel,
        view: &mut dyn RenderViewModel,
    ) {
        if mouse.clicked {
            // Map mouse clicks to the current region becoming active.
            view.set_active_section(self.active_region.clone());
        }
    }

    fn handle_resize(&mut self, _resize: Resize, view: &mut dyn RenderViewModel) {
        // On resize we ensure the view knows the content region is active so it can
        // reflow. This is intentionally minimal.
        view.set_active_section(Region::Content);
    }
}
