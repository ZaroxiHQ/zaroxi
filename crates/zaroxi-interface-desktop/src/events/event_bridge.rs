use crate::events::{UiEvent, Key};

/// Small enum describing high-level editor actions that originate from UI events.
///
/// These are UI->action bridge-level actions; actual execution is performed by
/// an ActionExecutor provided by the caller (which can invoke application/engine
/// actions). Keeping this indirection keeps the bridge testable and thin.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Action {
    MoveCursorDown,
    MoveCursorUp,
    InsertNewLine,
    SetActiveBuffer(String),
}

/// Executor trait the bridge calls to perform mapped actions. Implementations
/// should translate these into application/engine calls (outside this module).
pub trait ActionExecutor {
    fn execute(&mut self, action: Action);
}

/// The EventBridge maps UiEvent -> Action and forwards the Action to an
/// ActionExecutor. This keeps mapping logic isolated and testable while
/// avoiding duplication of engine logic in the UI layer.
pub struct EventBridge;

impl EventBridge {
    /// Map a UiEvent to a single high-level Action (if applicable).
    pub fn map_event_to_action(event: UiEvent) -> Option<Action> {
        match event {
            UiEvent::Key(k) => match k {
                Key::ArrowDown => Some(Action::MoveCursorDown),
                Key::ArrowUp => Some(Action::MoveCursorUp),
                Key::Enter => Some(Action::InsertNewLine),
                Key::Char(c) => Some(Action::SetActiveBuffer(c.to_string())),
                _ => None,
            },
            UiEvent::Mouse(m) => {
                if m.clicked {
                    // Example mapping: clicking could focus/select a buffer by coords.
                    Some(Action::SetActiveBuffer(format!("click_{}_{}", m.x, m.y)))
                } else {
                    None
                }
            }
            UiEvent::Resize(_) => None,
        }
    }

    /// Convenience handler: map the event and immediately dispatch to the executor.
    pub fn handle_event(event: UiEvent, executor: &mut dyn ActionExecutor) {
        if let Some(action) = Self::map_event_to_action(event) {
            executor.execute(action);
        }
    }
}
