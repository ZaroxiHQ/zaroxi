//! Minimal editor core engine for Zaroxi Studio.
//!
//! This crate provides the domain model and editing operations for the Zaroxi editor,
//! separate from UI concerns. It is intentionally minimal to guarantee correctness.

pub mod cursor;
pub mod document;
pub mod document_cache;
pub mod thresholds;
pub mod viewport;

// Re-export main types for convenience
pub use cursor::{Cursor, CursorMovement};
pub use document::Document;
pub use document_cache::BufferManager;
pub use document_cache::CachedDocument;
pub use thresholds::FileClass;
pub use viewport::Viewport;
 //! Minimal editor core engine for Zaroxi Studio.
 //!
 //! This crate provides the domain model and editing operations for the Zaroxi editor,
 //! separate from UI concerns. It is intentionally minimal to guarantee correctness.

 pub mod cursor;
 pub mod document;
 pub mod document_cache;
 pub mod thresholds;
 pub mod viewport;

 // Re-export main types for convenience
 pub use cursor::{Cursor, CursorMovement};
 pub use document::Document;
 pub use document_cache::BufferManager;
 pub use document_cache::CachedDocument;
 pub use thresholds::FileClass;
 pub use viewport::Viewport;
