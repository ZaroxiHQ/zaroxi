use anyhow::Result;
use std::sync::{Arc, Mutex};

use crate::window_state::WindowState;

/// Minimal engine application skeleton (window + lifecycle state).
///
/// The winit/renderer integration that previously lived behind the
/// `render_integration` feature referenced a crate (`zaroxi_app`) and a
/// `UiBlock` shape that no longer exist; it has been removed. The maintained
/// application entry point is the desktop shell (`zaroxi-interface-desktop`).
#[allow(dead_code)]
pub struct App {
    title: String,
    width: u32,
    height: u32,
    clear_color: [f64; 4],

    window: Option<Arc<winit::window::Window>>,
    window_state: Option<WindowState>,
    fatal_error: Option<anyhow::Error>,

    continuous: bool,
}

impl App {
    pub fn new(title: String, width: u32, height: u32, clear_color: [f64; 4]) -> Self {
        Self {
            title,
            width,
            height,
            clear_color,
            window: None,
            window_state: None,
            fatal_error: None,
            continuous: false,
        }
    }
}

/// Runtime entry point.
///
/// The renderer integration is intentionally not built here; this returns an
/// error so callers get a clear signal rather than a silent no-op. Use the
/// `zaroxi-interface-desktop` shell for the real application.
pub fn run(
    _title: String,
    _width: u32,
    _height: u32,
    _clear_color: [f64; 4],
    _app_state: Arc<Mutex<()>>,
) -> Result<()> {
    Err(anyhow::anyhow!(
        "runtime render integration removed; use the zaroxi-interface-desktop shell"
    ))
}
