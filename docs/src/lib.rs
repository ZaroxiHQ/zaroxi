#![doc = "Placeholder library so `cargo metadata` succeeds for the docs workspace member."]
#![allow(dead_code)]

/// Crate identity helper for the docs placeholder.
pub const CRATE_NAME: &str = "docs";

/// Minimal info function.
pub fn info() -> &'static str {
    CRATE_NAME
}
