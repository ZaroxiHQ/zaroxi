#![doc = "AI service orchestration logic for Zaroxi Studio.\n\nThis crate contains small, focused pieces for Phase 0: a service implementation,\ntask DTOs, and the ports module (the application-owned trait).\n\nKeep the public surface minimal. The composition root (apps/zaroxi-desktop-harness)\nwires infra adapters to application services.\n"]

pub mod mock;
pub mod ports;
pub mod service;
pub mod tasks;

/// Prelude for convenient imports used by outer composition and tests.
/// Note: do not re-export `mock` here to avoid exposing infra test adapters
/// unnecessarily and to silence unused-import warnings when the infra mock
/// is not linked into a particular composition.
pub mod prelude {
    pub use super::ports::*;
    pub use super::service::*;
    pub use super::tasks::*;
}

// Small view model re-exports for UI consumption
pub mod view_model;
