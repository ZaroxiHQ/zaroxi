#![doc = "zaroxi-security-audit: structured, append-only audit event types and helpers.\n\nThis crate provides canonical event types used across the system for auditing sensitive operations. It should not perform IO itself."]
#![deny(missing_docs)]

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Canonical audit event.
#[derive(Debug, Serialize, Deserialize)]
pub struct AuditEvent {
    /// RFC3339 timestamp.
    pub timestamp: DateTime<Utc>,
    /// The actor performing the action (user id or system).
    pub actor: String,
    /// Action description.
    pub action: String,
    /// Optional details (opaque JSON string).
    pub details: Option<String>,
}
