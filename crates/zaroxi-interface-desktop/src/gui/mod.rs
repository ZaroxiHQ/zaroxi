#![doc = "Minimal GUI-1 shell module: layout-first scaffold and simple placeholder rendering.\n\nThis module is intentionally small: it exposes a ShellFrame type that computes\nstable region rectangles for a canonical layout and produces deterministic\nplaceholder lines suitable for smoke tests and manual inspection.\n\nNo editor behavior or GPU rendering is implemented here; the binary prints\ntranscript lines for verification."]
pub mod shell;
pub mod widgets;
#[cfg(feature = "gui_window")]
pub mod window;
pub mod work_content;

// Re-export commonly used shell types so downstream window modules can refer to
// crate::gui::Theme (and other types) without importing the internal `shell` path.
pub use shell::{Rect, ShellFrame, ShellRegion, Size, Theme};
pub use widgets::render_chrome;
#[cfg(feature = "gui_window")]
pub use window::run_shell_window;
pub use work_content::ShellWorkContent;
