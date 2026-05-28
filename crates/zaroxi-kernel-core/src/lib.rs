#![doc = "Foundation primitives shared across Zaroxi crates.

This crate intentionally stays small: strongly-typed IDs and tiny common
helpers that can be used by multiple other crates without dragging in lots
of dependencies.
"]

pub mod ids;

pub use ids::*;

/// Return a canonical release version string for the built crate/workspace.
///
/// This tiny accessor is provided so release tooling and scripts can query
/// a stable symbol instead of parsing many Cargo.toml files. It is intentionally
/// minimal and resolves at compile-time using Cargo-provided environment variables.
pub fn release_version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}
