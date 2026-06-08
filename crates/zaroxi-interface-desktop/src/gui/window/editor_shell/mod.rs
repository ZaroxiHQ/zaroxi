/*!
Editor Phase 1 — Editor shell extraction module.

Separates editor-specific UI/layout/render orchestration from `app.rs`.
Provides:

- `ShellLayoutController` — Cached Taffy-based layout engine with resize detection.
- `EditorViewport` — Single source of truth for editor content region + clip boundary.
- `clip` — Clipping helpers for viewport-bounded rendering.
- `compute_layout` — Direct Taffy layout computation (for testing/diagnostics).
*/

pub mod clip;
pub mod constants;
pub mod controller;
pub mod layout;
pub mod view;

pub use constants::*;
pub use controller::ShellLayoutController;
pub use layout::{EditorShellLayout, compute_layout};
pub use view::EditorViewport;
