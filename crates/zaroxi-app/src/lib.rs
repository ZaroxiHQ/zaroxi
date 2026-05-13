#![doc = "Application orchestration: AppState, command dispatch and layout decisions.\n\nThis crate connects the domain/editor/workspace crates into a single app model\nthat the runtime/renderer will later consume. It purposefully avoids UI code\nand side effects."]
pub mod app;
pub mod commands;
pub mod layout;
pub mod status;

pub use app::AppState;
pub use commands::AppCommand;
pub use status::StatusState;
