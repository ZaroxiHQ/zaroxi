#![doc = "zaroxi-core-state: deterministic, allocation-aware state primitives for core engines.\n\nThis crate is part of the core-* layer and must avoid IO and heavy allocations. It provides small, efficient containers and helpers consumed by core systems."]
#![deny(missing_docs)]

/// Minimal prelude for consumers.
pub mod prelude {
    pub use crate::state::{SmallState, Version};
}

/// Lightweight state primitives used by core systems.
pub mod state {
    /// Simple versioned state token.
    #[derive(Clone, Copy, Debug, Eq, PartialEq)]
    pub struct Version(pub u64);

    /// Small state holder (placeholder for more specialized types).
    #[derive(Clone, Debug)]
    pub struct SmallState {
        /// Current version token for this state.
        pub version: Version,
    }

    impl SmallState {
        /// Create a new SmallState with the given version.
        pub const fn new(v: Version) -> Self {
            Self { version: v }
        }
    }
}
