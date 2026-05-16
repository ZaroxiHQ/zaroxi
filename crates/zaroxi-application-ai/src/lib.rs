 //! AI service orchestration logic for Zaroxi Studio.
 //!
 //! This crate contains small, focused pieces for Phase 0: a service implementation,
 //! task DTOs, and the ports module (the application-owned trait).
 //!
 //! Keep the public surface minimal. The composition root (apps/zaroxi-desktop-harness)
 //! wires infra adapters to application services.

 pub mod service;
 pub mod tasks;
 pub mod ports;

 /// Prelude for convenient imports used by outer composition and tests.
 pub mod prelude {
     pub use super::service::*;
     pub use super::tasks::*;
     pub use super::ports::*;
 }
