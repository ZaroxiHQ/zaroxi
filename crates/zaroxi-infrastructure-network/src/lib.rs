#![doc = "zaroxi-infrastructure-network: concrete networking adapters for the infrastructure layer.\n\nThis crate MUST only implement adapters for traits defined by kernel/application layers. It may perform IO and use async runtimes."]
#![deny(missing_docs)]

/// Networking adapters module (placeholder).
pub mod adapters {
    /// Placeholder for a networking client adapter trait implementation.
    pub struct NetClient;

    impl NetClient {
        /// Create a new client (placeholder).
        pub async fn new() -> Self {
            NetClient
        }
    }
}
