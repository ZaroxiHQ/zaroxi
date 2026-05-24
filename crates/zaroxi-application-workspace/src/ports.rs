/*!
Ports module (refactored).

This file is a thin facade that re-exports the focused ports submodules
implemented under `src/ports/`. The original monolithic `ports.rs` was
split into smaller files to reduce review surface and make future edits safer.

Behavior and public API are preserved by re-exporting the original symbols
from the `types` module (which contains the original DTOs/traits) and
by keeping small placeholder modules for further focused splitting.
*/

pub mod ai;
pub mod buffer;
pub mod close_flow;
pub mod durability;
pub mod editor;
pub mod history;
pub mod types;
pub mod workspace;

// Preserve the original crate::ports public surface by re-exporting the
// primary definitions from the `types` module (this keeps callers working
// without changing their import paths).
pub use types::*;
