#![allow(dead_code)]
// Minimal application-command crate: owns command DTOs and small command-owned helpers.

// Re-export the ports module so consumers can import:
//   zaroxi_application_command::ports::{...}
// The ports.rs file contains the DTOs and serde derives.
pub mod ports;

pub use ports::*;
