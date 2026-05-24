/*!
Split actions implementation hub.

This module aggregates smaller action submodules and re-exports their
symbols so the public API at `crate::actions` remains stable while the
implementation is organized into focused files.
*/

// The action submodules live in the same directory as this file (src/actions).
// Reference them by relative path inside the actions/ directory so the compiler
// can locate them when compiling this module file.
#[path = "actions_refresh.rs"]
pub mod actions_refresh;
pub use actions_refresh::*;

#[path = "actions_cursor.rs"]
pub mod actions_cursor;
pub use actions_cursor::*;

#[path = "actions_buffer.rs"]
pub mod actions_buffer;
pub use actions_buffer::*;

#[path = "actions_close_flow.rs"]
pub mod actions_close_flow;
pub use actions_close_flow::*;

#[path = "actions_command_bar.rs"]
pub mod actions_command_bar;
pub use actions_command_bar::*;
