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

/// Application state submodules (split for maintainability).
pub mod state;
pub mod view_model;

pub use commands::AppCommand;
pub use status::StatusState;
pub use assistant::AssistantState;
pub use tabs::TabState;

/// Re-export the AppState from the state module as the canonical app state type.
pub use state::AppState;
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

 /// Application state submodules (split for maintainability).
 pub mod state;
 pub mod view_model;

 pub use commands::AppCommand;
 pub use status::StatusState;
 pub use assistant::AssistantState;
 pub use tabs::TabState;

 /// Re-export the AppState from the state module as the canonical app state type.
 pub use state::AppState;
