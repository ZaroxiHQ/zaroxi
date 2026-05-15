#![doc = include_str!("../README.md")]
//! Kernel-level protocol definitions for Zaroxi.
//!
//! This crate lives in the kernel-* layer and exposes stable, minimal wire
//! types and errors used across the workspace, application, and infrastructure
//! layers. It must contain no IO or higher-level dependencies and should be
//! strictly versioned for wire-compatibility between components.

pub mod commands;
pub mod events;
pub mod workspace;

/// Prelude for convenient imports.
pub mod prelude {
    pub use super::commands::*;
    pub use super::events::*;
    pub use super::workspace::*;
}
