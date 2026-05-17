#![doc = "Application-facing facade for small, pure view models exposed to the runtime.\n\nThis crate intentionally avoids pulling in unrelated workspace crates while the\ninterface layer is migrated in phases. For Phase 48 we only expose the\nShellFrameViewModel wrapper which owns the desktop ShellFrameModel."]

pub mod shell_frame;
pub use shell_frame::ShellFrameViewModel;
