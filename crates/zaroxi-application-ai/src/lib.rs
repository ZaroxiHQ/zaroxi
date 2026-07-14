#![doc = "AI service orchestration logic for Zaroxi Studio.\n\nThis crate contains small, focused pieces for Phase 0: a service implementation,\ntask DTOs, and the ports module (the application-owned trait).\n\nKeep the public surface minimal. The composition root (apps/zaroxi-desktop-harness)\nwires infra adapters to application services.\n"]

pub mod action_service;
pub mod context_collector;
pub mod diff_applier;
pub mod mock;
pub mod panel;
pub mod ports;
pub mod provider_registry;
pub mod service;
pub mod session_manager;
pub mod tasks;
pub mod trace;

/// Prelude for convenient imports used by outer composition and tests.
pub mod prelude {
    pub use super::action_service::*;
    pub use super::context_collector::*;
    pub use super::diff_applier::*;
    pub use super::ports::*;
    pub use super::provider_registry::*;
    pub use super::service::*;
    pub use super::session_manager::*;
    pub use super::tasks::*;
}

pub mod view_model;
