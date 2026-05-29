#[cfg(feature = "gui_window")]
mod app;
#[cfg(feature = "gui_window")]
mod bootstrap;
#[cfg(feature = "gui_window")]
mod frame;
#[cfg(feature = "gui_window")]
mod theme_adapter;
#[cfg(feature = "gui_window")]
mod redraw;

#[cfg(feature = "gui_window")]
pub use bootstrap::run_shell_window;
