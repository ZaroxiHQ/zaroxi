#[cfg(feature = "gui_window")]
mod ai_pane;
#[cfg(feature = "gui_window")]
mod app;
#[cfg(feature = "gui_window")]
mod bootstrap;
#[cfg(feature = "gui_window")]
mod bottom_panel;
#[cfg(feature = "gui_window")]
mod editor;
#[cfg(feature = "gui_window")]
mod frame;
#[cfg(feature = "gui_window")]
mod rail;
#[cfg(feature = "gui_window")]
mod redraw;
#[cfg(feature = "gui_window")]
mod status_bar;
#[cfg(feature = "gui_window")]
pub mod style_tokens_adapter;
#[cfg(feature = "gui_window")]
mod syntax_color;
#[cfg(feature = "gui_window")]
mod text_adapter;
#[cfg(feature = "gui_window")]
mod theme_adapter;
#[cfg(feature = "gui_window")]
mod toolbar;

#[cfg(feature = "gui_window")]
pub use bootstrap::run_shell_window;
