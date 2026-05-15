//! AI service orchestration logic for Zaroxi Studio.

pub mod service;
pub mod tasks;

/// Prelude for convenient imports.
pub mod prelude {
    pub use super::service::*;
    pub use super::tasks::*;
}
