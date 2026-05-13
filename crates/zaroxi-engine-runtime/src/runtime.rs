use anyhow::Result;
use log::{error, info};
use std::sync::Arc;
use std::time::Instant;
use winit::application::{Application, ApplicationExt, ActiveEventLoop, ApplicationId};
use winit::window::{Window, WindowAttributes};
use winit::event::{Event, WindowEvent};
use winit::platform::run_return::EventLoopExtRunReturn;

use crate::window_state::WindowState;
use zaroxi_engine_input::event::Event as InputEvent;
use zaroxi_engine_render::renderer::Renderer;

/// Runtime application struct used with winit 0.30 Application API.
///
/// This struct implements the Application lifecycle: create window in `resumed`,
/// handle window events in `window_event`, and keep continuous redraw behavior.
struct EngineApp {
    title: String,
    width: u32,
    height: u32,
    clear_color: [f64; 4],
    window: Option<Arc<Window>>,
    renderer: Option<Renderer<'static>>,
    window_state: Option<WindowState>,
    fatal_error: Option<anyhow::Error>,
    continuous: bool,
    last_frame: Instant,
}

impl EngineApp {
    fn new(title: String, width: u32, height: u32, clear_color: [f64; 4]) -> Self {
        Self {
            title,
            width,
            height,
            clear_color,
            window: None,
            renderer: None,
            window_state: None,
            fatal_error: None,
            continuous: true,
            last_frame: Instant::now(),
        }
    }

    /// Initialize window and renderer when application resumes.
    fn resumed(&mut self, active_loop: &ActiveEventLoop) {
        // Create window attributes and window via ActiveEventLoop.
        let mut attrs = Window::default_attributes();
        attrs.inner_size = Some(winit::dpi::PhysicalSize::new(self.width, self.height));
        attrs.title = self.title.clone();

        match active_loop.create_window(attrs) {
            Ok(win) => {
                let win = Arc::new(win);
                // Initialize renderer (tie lifetime via 'static by leaking window reference
                // into a static reference for the renderer lifetime required here).
                // SAFETY: the Arc<Window> is stored in self.window so the window lives long enough.
                self.window = Some(win.clone());
                let window_ref: &'static Window = unsafe { &*(Arc::as_ptr(&win) as *const Window) };

                match pollster::block_on(Renderer::new(window_ref, self.clear_color)) {
                    Ok(r) => {
                        self.renderer = Some(r);
                        self.window_state = Some(WindowState::new(win.inner_size()));
                        // Request initial redraw.
                        win.request_redraw();
                    }
                    Err(e) => {
                        self.fatal_error = Some(anyhow::anyhow!("renderer init failed: {:?}", e));
                        // Exit the active event loop.
                        active_loop.exit();
                    }
                }
            }
            Err(e) => {
                self.fatal_error = Some(anyhow::anyhow!("window create failed: {:?}", e));
                active_loop.exit();
            }
        }
    }

    fn window_event(&mut self, event: &WindowEvent, active_loop: &ActiveEventLoop) {
        match event {
            WindowEvent::CloseRequested => {
                active_loop.exit();
            }
            WindowEvent::Resized(size) => {
                if let (Some(renderer), Some(ws)) = (self.renderer.as_mut(), self.window_state.as_mut()) {
                    if size.width > 0 && size.height > 0 {
                        ws.size = *size;
                        if let Err(e) = renderer.resize(*size) {
                            self.fatal_error = Some(anyhow::anyhow!("resize failed: {:?}", e));
                            active_loop.exit();
                        }
                    }
                }
            }
            WindowEvent::RedrawRequested => {
                if let Some(renderer) = self.renderer.as_mut() {
                    match renderer.render() {
                        Ok(_) => {
                            if let Some(win) = self.window.as_ref() {
                                // continuous redraw
                                if self.continuous {
                                    win.request_redraw();
                                }
                            }
                        }
                        Err(err) => {
                            // Map renderer error into fatal or recoverable states.
                            self.fatal_error = Some(anyhow::anyhow!("render error: {:?}", err));
                            active_loop.exit();
                        }
                    }
                }
            }
            other => {
                // For future: translate input events
                let _ = InputEvent::from_winit(other);
            }
        }
    }
}

/// Start the engine runtime using winit 0.30 Application API.
pub fn run(title: String, width: u32, height: u32, clear_color: [f64; 4]) -> Result<()> {
    // Initialize logging
    let _ = env_logger::try_init();
    info!("Starting runtime (application API) with title '{}'", title);

    // Create a new EventLoop and activate it.
    let event_loop = winit::event_loop::EventLoop::new()?;
    let mut active = ActiveEventLoop::new(event_loop)?;

    // Build the application instance.
    let mut app = EngineApp::new(title, width, height, clear_color);

    // Run the application. `run_app` will call back into lifecycle methods;
    // we implement resume/stop via the ActiveEventLoop API above.
    // Note: `run_app` may return an error type from winit; map to anyhow::Error.
    active.run_app(&mut app).map_err(|e| anyhow::anyhow!("run_app failed: {:?}", e))?;

    // If a fatal error was recorded, return it.
    if let Some(err) = app.fatal_error {
        return Err(err);
    }

    Ok(())
}
