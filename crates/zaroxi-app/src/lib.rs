#![doc = "Application orchestration: AppState, command dispatch and layout decisions.\n\nThis crate connects the domain/editor/workspace crates into a single app model\nthat the runtime/renderer will later consume. It purposefully avoids UI code\nand side effects."]
pub mod app;
pub mod commands;
pub mod layout;
pub mod status;

/// UI-related app modules that own small pieces of app state. Kept in the
/// `zaroxi-app` crate because they are purely application-level state (no UI).
pub mod panels;
pub mod assistant;
pub mod tabs;

pub use app::AppState;
pub use commands::AppCommand;
pub use status::StatusState;
pub use panels::BottomPanelState;
pub use assistant::AssistantState;
pub use tabs::TabState;
