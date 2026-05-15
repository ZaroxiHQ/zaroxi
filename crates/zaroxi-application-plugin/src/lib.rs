#![doc = "zaroxi-application-plugin: high-level plugin registration and lifecycle management for application layer.\n\nThis crate defines the application-facing plugin host API. Implementations (dynamic loaders, sandboxing) belong to platform/infrastructure crates."]
#![deny(missing_docs)]

/// Application plugin registry API (placeholder).
pub mod registry {
    /// Plugin descriptor used by the application host.
    pub struct PluginDescriptor {
        /// Stable plugin id.
        pub id: &'static str,
        /// Human-friendly name.
        pub name: &'static str,
        /// Semantic version string.
        pub version: &'static str,
    }
}
