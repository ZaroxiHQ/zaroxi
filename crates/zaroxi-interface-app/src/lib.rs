#![doc = "Application-facing facade for small, pure view models exposed to the runtime.\n\nThis crate intentionally avoids pulling in unrelated workspace crates while the\ninterface layer is migrated in phases. For Phase 48 we only expose the\nShellFrameViewModel wrapper which owns the desktop ShellFrameModel."]

pub mod shell_frame;
pub use shell_frame::ShellFrameViewModel;

// Tiny UI-facing semantic view model for renderer outputs.
//
// See `crate::shell_render_view` for details. This module intentionally stays
// minimal and non-visual: it carries only section ids, presence markers, order,
// and simple textual lines (no geometry, colors, or layout).
pub mod shell_render_view;
pub use shell_render_view::ShellRenderViewModel;
