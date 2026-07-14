//! AI domain layer: context, conversation, providers, MCP, prompts, and panel content.
//!
//! This crate provides pure domain data structures and logic for the AI side
//! of Zaroxi Studio. No transport, rendering, or persistence concerns live here.
pub mod context;
pub mod context_ide;
pub mod conversation;
pub mod mcp;
pub mod packing;
pub mod panel;
pub mod prompt;
pub mod provider;
pub mod ranking;
pub mod types;

/// Prelude for convenient imports.
pub mod prelude {
    pub use super::context::*;
    pub use super::context_ide::*;
    pub use super::conversation::*;
    pub use super::mcp::*;
    pub use super::packing::*;
    pub use super::panel::*;
    pub use super::prompt::*;
    pub use super::provider::*;
    pub use super::ranking::*;
    pub use super::types::*;
}
