//! Status bar panels.
//!
//! Each panel owns one focused fragment of the status bar and produces a small
//! list of display segments from the shared [`StatusModel`]. Layout:
//!
//! * Left zone — `workspace`, `document_state`, `diagnostics`.
//! * Right zone — `editor_position`, `file_format`.
//!
//! New panels (git/VCS, background tasks, AI state) slot in here without
//! touching the existing ones. A `vcs` panel is intentionally absent until a
//! real git source of truth exists.

pub mod diagnostics;
pub mod document_state;
pub mod editor_position;
pub mod file_format;
pub mod workspace;
