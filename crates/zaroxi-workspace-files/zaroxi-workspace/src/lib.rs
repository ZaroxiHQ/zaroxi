#![doc = "Workspace model: root path, items and selection state."]

pub mod state;
pub mod item;

pub use state::WorkspaceState;
pub use item::WorkspaceItem;
