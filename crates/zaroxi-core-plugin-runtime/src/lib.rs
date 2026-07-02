#![doc = "zaroxi-core-plugin-runtime: plugin host traits and safe boundaries for core systems.\n\nThis crate defines the minimal host interface and safety contracts plugins must satisfy. Actual dynamic loading implementations belong in platform/infrastructure crates."]
#![deny(missing_docs)]

/// Host-facing traits for plugins.
pub mod host {
    /// Minimal plugin lifecycle.
    pub trait Plugin: Send + Sync {
        /// Name of the plugin.
        fn name(&self) -> &'static str;
        /// Initialize plugin; returns opaque boxed extension or unit.
        // Placeholder plugin API: the `()` error is a deliberate stub until a
        // concrete plugin error type lands.
        #[allow(clippy::result_unit_err)]
        fn initialize(&self) -> Result<(), ()>;
        /// Shutdown plugin.
        fn shutdown(&self);
    }
}
