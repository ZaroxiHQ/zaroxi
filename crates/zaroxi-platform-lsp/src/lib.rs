//! Language Server Protocol client implementation.
//!
//! Provides a client for communicating with Language Servers, including
//! session management, transport handling, capabilities negotiation,
//! and diagnostics processing.

pub mod capabilities;
pub mod diagnostics;
pub mod manager;
pub mod session;
pub mod transport;
