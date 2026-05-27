#![doc = "Docs placeholder crate. Purpose: allow `cargo metadata` to include `docs/` as a workspace member. This file is intentionally minimal and MUST NOT contain runtime logic."]
#![allow(dead_code)]

/// Crate identity helper for the docs placeholder.
pub const CRATE_NAME: &str = "docs";

/// Minimal info function.
pub fn info() -> &'static str {
    CRATE_NAME
}
