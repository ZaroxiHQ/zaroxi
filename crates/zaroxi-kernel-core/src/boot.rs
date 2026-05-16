// Minimal kernel helpers for Phase 0 / Phase 1 boot and async types.
//
// This module defines small boxed future aliases and a Boot trait used by the
// composition root. Keep this tiny — no runtime selection, no feature flags.

use std::pin::Pin;
use std::future::Future;
use std::path::PathBuf;

/// Boxed future alias used across the first slice.
pub type BoxFuture<'a, T> = Pin<Box<dyn Future<Output = T> + Send + 'a>>;

/// Simple result alias for bootstrap phase.
pub type KernelResult<T> = Result<T, String>;

/// Boot trait: a small lifecycle hook for starting application composed services.
pub trait Boot: Send + Sync {
    /// Start the composed application. Implementations should run until the minimal scenario completes.
    fn start(&self) -> BoxFuture<'static, KernelResult<()>>;
}

/// Lightweight boot config DTO for future extension.
#[derive(Clone, Debug)]
pub struct BootConfig {
    pub workspace_path: Option<PathBuf>,
}

impl Default for BootConfig {
    fn default() -> Self {
        Self { workspace_path: None }
    }
}
