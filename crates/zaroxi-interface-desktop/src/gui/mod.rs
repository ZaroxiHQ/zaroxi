#![doc = "Minimal GUI-1 shell module: layout-first scaffold and simple placeholder rendering.\n\nThis module is intentionally small: it exposes a ShellFrame type that computes\nstable region rectangles for a canonical layout and produces deterministic\nplaceholder lines suitable for smoke tests and manual inspection.\n\nNo editor behavior or GPU rendering is implemented here; the binary prints\ntranscript lines for verification."]
pub mod shell;

pub use shell::{ShellFrame, ShellRegion, Size, Rect};
