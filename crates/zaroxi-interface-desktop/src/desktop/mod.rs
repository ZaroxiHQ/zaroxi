/*
Desktop root module.

This file exposes the desktop submodules and re-exports the small, stable
public surface consumed by other crates (tests, harness, app). The large
composition implementation has been split under `desktop/composition/`.
Keep this façade minimal: declare submodules and re-export the small set of
types that callers import from `crate::desktop`.
*/

pub mod composition;
mod consistency;
pub mod projections;
mod snapshot;
mod state;
mod status_bar;
mod summary;

mod command_bar;
mod pending_close;
mod status;

// Re-export the composition-facing public surface.
//
// Many internal modules (actions, status_bar, gpu runtime, etc.) import a
// small set of types from `crate::desktop`. Preserve that surface here so
// existing callers remain unchanged while the implementation is split.
pub use composition::{
    ActiveBufferDetails, ActiveDocumentSummary, AiKind, AiProjection, AiProjectionSummary, AiState,
    Command, CommandBarState, DesktopComposition, DesktopMetadata, DesktopSummary, OpenedBufferItem,
    OpenedBufferItemSummary, OpenedBuffersSummary, RefreshReason, ShellContext, ShellSnapshot,
    StatusBarLine, ViewportAnchoring, ViewportSummary,
};

// Preserve other re-exports used across the crate.
pub use consistency::DesktopConsistencyReport;
pub use crate::close::PendingClose;
