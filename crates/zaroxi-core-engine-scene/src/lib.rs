#![allow(dead_code)]
// Tiny semantic scene-description model for Phase 50.
// See ARCHITECTURE.md for rationale and details.

pub const CRATE_NAME: &str = "zaroxi-core-engine-scene";

pub mod scene;
pub use scene::ShellSceneModel;

pub fn info() -> &'static str {
    CRATE_NAME
}
