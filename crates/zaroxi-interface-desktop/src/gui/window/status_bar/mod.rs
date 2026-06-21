//! Status bar feature module.
//!
//! Clean, modular status bar for the desktop editor shell. The pieces are split
//! by responsibility:
//!
//! * [`model`] — typed view-model + raw [`StatusInputs`] derived from live state.
//! * [`style`] — restrained colour/spacing tokens for the bar.
//! * [`panels`] — small, focused fragment builders (workspace, document state,
//!   diagnostics, editor position, file format).
//! * [`view`] — assembles panels into the final shell `UiBlock`.
//!
//! Left zone = primary/global state (workspace, document state, diagnostics).
//! Right zone = contextual file/editor state (position + selection, indent,
//! encoding, line endings, language). The design is deliberately quiet: it should
//! not draw attention unless something actually matters.

mod model;
mod panels;
mod style;
mod view;

pub use model::{DiagnosticCounts, SelectionInfo, StatusInputs, StatusModel};
pub use view::StatusView;
