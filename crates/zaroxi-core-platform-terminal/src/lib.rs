//! Integrated terminal backend for Zaroxi Studio.
//!
//! This crate owns the non-UI half of the integrated terminal:
//! - [`session::TerminalSession`] — a real PTY-backed shell process with an
//!   asynchronous output pump, resize, scrollback and clean lifecycle.
//! - [`render_model`] — a renderer-agnostic cell-grid projection of the live
//!   emulator screen.
//! - [`input`] — pure key → VT byte encoding.
//! - [`palette`] — `vt100::Color` → RGBA resolution wired from theme tokens.
//!
//! The interface layer (`zaroxi-interface-desktop`) drives a session, converts
//! the grid into draw commands, and routes keyboard/mouse/clipboard events.
//!
//! ## Crate stack rationale
//! - **portable-pty** provides a safe, cross-platform PTY abstraction (Unix
//!   openpty + Windows ConPTY) so this crate needs no `unsafe`.
//! - **vt100** is a pure, well-tested VT emulator (parser + screen grid with
//!   scrollback, ANSI/256/truecolor and alternate-screen support) that is
//!   trivial to unit-test by feeding bytes and inspecting the screen.

#![doc = "Integrated terminal backend (PTY + VT emulation)."]

pub mod config;
pub mod input;
pub mod palette;
pub mod render_model;
pub mod session;
pub mod view_model;

pub use config::{TerminalConfig, resolve_shell_program};
pub use input::{KeyModifiers, TerminalKey, encode_key};
pub use palette::{Rgba, TerminalPalette};
pub use render_model::{TerminalCell, TerminalCursor, TerminalGrid, build_grid};
pub use session::{PumpOutcome, TerminalError, TerminalExit, TerminalSession};

/// The crate package name.
pub const CRATE_NAME: &str = env!("CARGO_PKG_NAME");

/// A small sanity helper (retained for compatibility with earlier stubs).
pub fn info() -> &'static str {
    CRATE_NAME
}
