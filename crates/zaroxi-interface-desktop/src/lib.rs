#![doc = "Editor core state and light command types.\n\nThis crate contains in-memory editor state (open documents, active document)\nand small helper APIs. Business logic lives here; rendering and I/O are left to\nother crates."]

pub mod state;
pub mod commands;
pub mod compose;
pub mod view_adapter;
pub mod presenter;
pub mod desktop;
pub mod actions;

pub use state::EditorState;
pub use commands::EditorCommand;
pub use view_adapter::{InterfaceRenderableWindow, InterfaceRenderableLine, InterfaceRenderSpan, InterfaceSpanKind, fetch_renderable_window};
pub use presenter::Presenter;
pub use desktop::DesktopComposition;
pub use actions::{refresh_desktop, move_cursor_to_start_and_refresh, ActionResult};
