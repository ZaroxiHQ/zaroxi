//! Status bar panels.
//!
//! Each panel owns one focused fragment of the status bar and produces a small
//! list of display segments from the shared [`StatusModel`]. The view layer
//! places `workspace` on the left and `editor_position` + `file_format` on the
//! right. New panels (git, diagnostics, tasks, selection) slot in here without
//! touching the existing ones.

pub mod editor_position;
pub mod file_format;
pub mod workspace;
