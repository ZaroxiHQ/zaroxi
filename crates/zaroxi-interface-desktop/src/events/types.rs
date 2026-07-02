/// UI-only types for event handling inside the `zaroxi-interface-desktop` crate.
///
/// This file defines a minimal set of UI events and thin UI-facing traits that
/// the event router will call. These are intentionally UI-only: they do not
/// perform engine mutations directly, they only express UI-state transitions
/// (focused line, active buffer, active section).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Key {
    ArrowUp,
    ArrowDown,
    ArrowLeft,
    ArrowRight,
    Enter,
    Escape,
    Char(char),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MouseButton {
    Left,
    Right,
    Middle,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Mouse {
    pub x: u32,
    pub y: u32,
    pub button: Option<MouseButton>,
    pub clicked: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Resize {
    pub width: u32,
    pub height: u32,
}

/// High-level UI events exposed within the interface layer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UiEvent {
    Key(Key),
    Mouse(Mouse),
    Resize(Resize),
}

/// Logical layout regions produced by the layout layer. The router will route
/// events to the "active" region.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum Region {
    Chrome,
    #[default]
    Content,
    Status,
}

/// Thin UI-facing trait representing the frame-level UI model.
///
/// Implementations live in `zaroxi-interface-desktop` (UI only) and are used by
/// the EventRouter to update view-facing state such as focused line or active
/// buffer. These methods intentionally do not execute engine actions; they
/// only update UI state that may later be turned into engine actions.
pub trait FrameModel {
    /// Return the 0-based focused line index for the active buffer.
    fn focused_line(&self) -> u32;

    /// Move focus down by one line in the active buffer (UI-only).
    fn move_focus_down(&mut self);

    /// Activate the current buffer (e.g. when Enter is pressed). UI-only.
    fn activate_current_buffer(&mut self);

    /// Set the currently active buffer by name (UI-only).
    fn set_active_buffer(&mut self, name: String);
}

/// Thin UI-facing trait representing the render view model (shell/frame
/// presentation).
pub trait RenderViewModel {
    /// Set which region/section is considered active for routing (chrome/content/status).
    fn set_active_section(&mut self, region: Region);

    /// Inspect the currently active region in the view model.
    fn active_section(&self) -> Region;
}
