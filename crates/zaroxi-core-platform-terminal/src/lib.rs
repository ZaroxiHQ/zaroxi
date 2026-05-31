// Auto-generated stub.
// Replace this file with the crate implementation.

#![allow(dead_code)]
#![doc = "Auto-generated crate stub. Replace with real implementation."]

/// The crate package name.
pub const CRATE_NAME: &str = env!("CARGO_PKG_NAME");

/// A small sanity helper.
pub fn info() -> &'static str {
    CRATE_NAME
}

// Expose small view model types for the UI composition layer
pub mod view_model;
