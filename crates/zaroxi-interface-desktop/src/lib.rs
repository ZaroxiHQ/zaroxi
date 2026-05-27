#![doc = "Editor core state and light command types.\n\nThis crate contains in-memory editor state (open documents, active document)\nand small helper APIs. Business logic lives here; rendering and I/O are left to\nother crates."]

pub mod actions;
pub mod clipboard;
pub mod close;
pub mod commands;
pub mod compose;
pub mod desktop;
pub mod events;
pub mod gpu_shell_adapter;
pub mod gpu_shell_runtime;
pub mod input;
pub mod presenter;
pub mod selection_view;
pub mod state;
pub mod text;
pub mod text_view;
pub mod view_adapter;

// Re-export application ports so tests and internal modules can refer to `crate::ports`.
// This keeps the interface crate surface small while enabling test modules to implement
// application traits without repetitively importing the application crate paths.
pub use zaroxi_application_workspace::ports;
// Re-export the BoxFuture alias from the application ports at the crate root so
// test modules and internal helpers can refer to `crate::BoxFuture` and avoid
// verbose `crate::ports::BoxFuture` occurrences in inline test stubs.
pub use crate::ports::BoxFuture;

pub use actions::{
    ActionResult, ShellActionResult, move_cursor_to_start_and_refresh,
    refresh_and_get_shell_context, refresh_desktop, set_active_buffer_and_get_shell_context,
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

pub mod render_debug_text;
pub use render_debug_text::render_debug_text;

pub mod presenters;
pub mod diagnostics;
pub use clipboard::InMemoryClipboard;
pub use presenters::GpuShellPresenter;
pub use presenters::ShellRenderPresenter;

// Small adapter-local projections collected under `projections`.
// Keep all shaping here; do not leak UI/shell concerns into application/domain.
pub mod projections;
pub use projections::active_buffer_line::ActiveBufferLine;
pub use projections::last_event_line::LastEventLine;
pub use projections::last_event_line::summarize_last_event;
pub use projections::location_line::LocationLine;
