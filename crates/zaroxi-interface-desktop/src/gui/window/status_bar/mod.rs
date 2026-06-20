//! Status bar feature module.
//!
//! Clean, modular status bar for the desktop editor shell. The pieces are split
//! by responsibility:
//!
//! * [`model`] — typed view-model derived from app/editor/workspace state.
//! * [`style`] — restrained colour/spacing tokens for the bar.
//! * [`panels`] — small, focused fragment builders (workspace, position, format).
//! * [`view`] — assembles panels into the final shell `UiBlock`.
//!
//! Left zone = primary/global state (workspace, transient doc state).
//! Right zone = contextual file/editor state (position, indent, encoding,
//! line endings, language). The design is deliberately quiet: it should not draw
//! attention unless something actually matters.

mod model;
mod panels;
mod style;
mod view;

pub use model::StatusModel;
pub use view::StatusView;
