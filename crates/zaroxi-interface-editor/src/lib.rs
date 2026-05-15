#![doc = "Editor core state and light command types.\n\nThis crate contains in-memory editor state (open documents, active document)\nand small helper APIs. Business logic lives here; rendering and I/O are left to\nother crates."]

pub mod state;
pub mod commands;

pub use state::EditorState;
pub use commands::EditorCommand;
