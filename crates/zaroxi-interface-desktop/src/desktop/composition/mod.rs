/*!
Composition submodule split into focused files.

- state: stored data types, DesktopComposition struct, and basic accessors.
- refresh: refresh/build/update logic and AI apply/request helpers.
- projections: projection assembly helpers (active document, opened buffers, shell context).
- summary: small summary helpers (AI projection summary).
- work_content: DesktopComposition::build_work_content() adapter bridging
  desktop DTOs into Core's ShellWorkContent.
*/

pub mod projections;
pub mod refresh;
pub mod state;
pub mod summary;
pub mod work_content;

pub use state::{
    AiKind, AiProjection, AiProjectionSummary, AiState, Command, CommandBarState,
    DesktopComposition, DesktopMetadata, DesktopStatus, DesktopSummary, OpenedBufferItem,
    ShellSnapshot, StatusBarLine, canonical_buffer_path, dedupe_opened_buffers,
    same_opened_document,
};

// Re-export workspace-view DTOs through composition so downstream crate::desktop::*
// imports find them at the same paths.
pub use zaroxi_application_workspace::workspace_view::{
    ActiveBufferDetails, ActiveDocumentSummary, OpenedBufferItemSummary, OpenedBuffersSummary,
    RefreshReason, ShellContext, ViewportAnchoring, ViewportSummary,
};

pub use refresh::{
    apply_ai_edit_active, cancel_ai_edit_active, refresh_with_service, request_ai_edit_active,
};

pub use projections::{
    latest_active_document_summary, latest_opened_buffers_summary, latest_shell_context,
};

pub use summary::latest_ai_projection_summary;
