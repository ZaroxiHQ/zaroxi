#![doc = "Interface-desktop: native windowing, event-loop integration, shell layout,\nGPU overlay submission, transcript rendering, and desktop-only presenters.\n\nBusiness logic and content assembly policy lives in Core/Application/Domain crates.\nThis crate only places and renders pre-assembled workspace content."]

pub mod actions;
pub mod clipboard;
pub mod close;
pub mod commands;
pub mod desktop;
pub mod engine_adapter;
pub mod events;
pub mod folder_picker;
pub mod gpu_shell_adapter;
pub mod gpu_shell_runtime;
pub mod input;
pub mod presenter;
pub mod selection_view;
pub mod state;
pub mod text;
pub mod text_view;
pub mod view_adapter;

pub use crate::ports::BoxFuture;
pub use zaroxi_application_workspace::ports;

pub use actions::{
    move_cursor_to_start_and_refresh, refresh_and_get_shell_context, refresh_desktop,
    set_active_buffer_and_get_shell_context,
};
pub use commands::EditorCommand;
pub use desktop::{
    ActiveDocumentSummary, CommandBarState, DesktopComposition, DesktopConsistencyReport,
    DesktopSummary, OpenedBufferItemSummary, OpenedBuffersSummary, PendingClose, ShellContext,
    ShellSnapshot, ViewportAnchoring, ViewportSummary,
};
pub use presenter::Presenter;
pub use selection_view::SelectionView;
pub use state::EditorState;
pub use text_view::TextView;
pub use view_adapter::{
    InterfaceRenderSpan, InterfaceRenderableLine, InterfaceRenderableWindow, InterfaceSpanKind,
    fetch_renderable_window,
};
pub use zaroxi_application_workspace::workspace_view::{ActionResult, ShellActionResult};

pub mod render_debug_text;
pub use render_debug_text::render_debug_text;

pub mod diagnostics;
pub mod gui;
pub mod presenters;
pub use clipboard::InMemoryClipboard;
pub use presenters::GpuShellPresenter;
pub use presenters::ShellRenderPresenter;

// Small adapter-local projections collected under `projections`.
// Keep all shaping here; do not leak UI/shell concerns into application/domain.
pub mod ai;
pub mod projections;
pub use projections::active_buffer_line::ActiveBufferLine;
pub use projections::last_event_line::LastEventLine;
pub use projections::last_event_line::summarize_last_event;
pub use projections::location_line::LocationLine;
