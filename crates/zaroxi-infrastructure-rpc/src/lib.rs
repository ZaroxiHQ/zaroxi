//! Remote Procedure Call framework for Neote.
//!
//! Provides client and server implementations for RPC communication,
//! including message framing, serialization, and connection management.

pub mod client;
pub mod framing;
pub mod messages;
pub mod server;
