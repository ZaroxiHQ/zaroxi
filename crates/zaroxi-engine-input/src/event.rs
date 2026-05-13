use winit::event::{ElementState, MouseButton, MouseScrollDelta, WindowEvent};

/// A very small normalization layer for input events.
/// This is intentionally minimal for v1 to provide a stable seam
/// for future dispatcher integration.
#[derive(Debug, Clone)]
pub enum Event {
    Key {
        scancode: u32,
        state: ElementState,
    },
    MouseButton {
        button: MouseButton,
        state: ElementState,
    },
    CursorMoved {
        x: f64,
        y: f64,
    },
    Wheel {
        delta: MouseScrollDelta,
    },
    Resized {
        width: u32,
        height: u32,
    },
}

impl Event {
    /// Convert from a winit WindowEvent to our normalized Event.
    /// Returns None for events we don't normalize yet.
    pub fn from_winit(ev: &WindowEvent) -> Option<Event> {
        match ev {
            // WindowEvent::KeyboardInput is a struct variant in current winit.
            // Match the `event` field to obtain the KeyboardInput.
            WindowEvent::KeyboardInput { event, .. } => {
                let sc = event.scancode;
                Some(Event::Key {
                    scancode: sc,
                    state: event.state,
                })
            }
            WindowEvent::MouseInput { state, button, .. } => Some(Event::MouseButton {
                button: *button,
                state: *state,
            }),
            WindowEvent::CursorMoved { position, .. } => Some(Event::CursorMoved {
                x: position.x,
                y: position.y,
            }),
            WindowEvent::MouseWheel { delta, .. } => Some(Event::Wheel { delta: *delta }),
            WindowEvent::Resized(size) => Some(Event::Resized {
                width: size.width,
                height: size.height,
            }),
            _ => None,
        }
    }
}
