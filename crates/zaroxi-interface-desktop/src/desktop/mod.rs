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

pub use composition::{
    ActiveDocumentSummary, CommandBarState, DesktopComposition, DesktopSummary,
    OpenedBufferItemSummary, OpenedBuffersSummary, ShellContext, ShellSnapshot,
    ViewportAnchoring, ViewportSummary,
};

pub use consistency::DesktopConsistencyReport;
pub use crate::close::PendingClose;
