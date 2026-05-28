/*!
Composition submodule split into focused files.

This module now provides a small façade that exposes the composition public
types and functions while delegating implementation to focused submodules:

- state: stored data types, DesktopComposition struct, and basic accessors.
- refresh: refresh/build/update logic and AI apply/request helpers.
- projections: projection assembly helpers (active document, opened buffers, shell context).
- summary: small summary helpers (AI projection summary).

The goal is to keep behavior identical while making future maintenance easier.

Public API (stable): the original symbols remain available from
`crate::desktop` because `desktop/mod.rs` includes this file.
*/

pub mod state;
pub mod refresh;
pub mod projections;
pub mod summary;

pub use state::{
    ActiveBufferDetails, ActiveDocumentSummary, AiKind, AiProjection, AiProjectionSummary, AiState,
    Command, CommandBarState, DesktopComposition, DesktopMetadata, DesktopStatus, OpenedBufferItem,
    OpenedBufferItemSummary, OpenedBuffersSummary, RefreshReason, ShellContext, ShellSnapshot,
    StatusBarLine, ViewportAnchoring, ViewportSummary,
};

pub use refresh::{apply_ai_edit_active, cancel_ai_edit_active, request_ai_edit_active, refresh_with_service};

pub use projections::{latest_active_document_summary, latest_opened_buffers_summary, latest_shell_context};

pub use summary::latest_ai_projection_summary;

/// Keep the small internal helper available to the crate as before.
pub(crate) use state::command_kind_short_name;
