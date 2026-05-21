#![doc = "Foundation primitives shared across Zaroxi crates.

This crate intentionally stays small: strongly-typed IDs and tiny common
helpers that can be used by multiple other crates without dragging in lots
of dependencies.
"]

pub mod ids;

pub use ids::*;
