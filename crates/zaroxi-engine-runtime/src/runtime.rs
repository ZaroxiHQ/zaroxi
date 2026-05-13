use anyhow::Result;
use log::{error, info};
use std::sync::Arc;
use winit::application::ApplicationHandler;
use winit::event::WindowEvent;
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop};
use winit::window::{Window, WindowId};

use crate::window_state::WindowState;
use zaroxi_engine_input::event::Event as InputEvent;
use zaroxi_engine_render::renderer::Renderer;

/// Minimal engine application that implements the winit 0.30 ApplicationHandler
/// lifecycle. This keeps the runtime small and focused on window + renderer.
pub struct App {
    title: String,
    width: u32,
    height: u32,
    clear_color: [f64; 4],

    window: Option<Arc<Window>>,
    renderer: Option<Renderer<'static>>,
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
            renderer: None,
            window_state: None,
            fatal_error: None,
            continuous: true,
        }
    }
}

impl ApplicationHandler for App {
    /// Called when the application is resumed; create window and renderer here.
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        // Build window attributes using the builder style helpers.
        let attrs = Window::default_attributes()
            .with_title(self.title.clone())
            .with_inner_size(winit::dpi::PhysicalSize::new(self.width, self.height));

        match event_loop.create_window(attrs) {
            Ok(win) => {
                let win: Arc<Window> = Arc::new(win);

                // Store Arc so the window lives long enough.
                self.window_state = Some(WindowState::new(win.inner_size()));
                self.window = Some(win.clone());

                // Create a 'static reference for the renderer by leveraging the Arc.
                // SAFETY: the Arc is kept in self.window so the pointer remains valid.
                let window_ref: &'static Window = unsafe { &*(Arc::as_ptr(&win) as *const Window) };

                match pollster::block_on(Renderer::new(window_ref, self.clear_color)) {
                    Ok(renderer) => {
                        self.renderer = Some(renderer);
                        // Request an initial redraw.
                        win.request_redraw();
                    }
                    Err(e) => {
                        self.fatal_error = Some(anyhow::anyhow!("renderer init failed: {:?}", e));
                        event_loop.exit();
                    }
                }
            }
            Err(e) => {
                self.fatal_error = Some(anyhow::anyhow!("window create failed: {:?}", e));
                event_loop.exit();
            }
        }
    }

    /// Handle window-level events.
    fn window_event(&mut self, event_loop: &ActiveEventLoop, window_id: WindowId, event: WindowEvent) {
        // Only handle events for our window.
        let is_our = match (&self.window, &window_id) {
            (Some(w), id) => *id == w.id(),
            (None, _) => false,
        };

        if !is_our {
            return;
        }

        match event {
            WindowEvent::CloseRequested => {
                event_loop.exit();
            }
            WindowEvent::Resized(new_size) => {
                if let (Some(renderer), Some(ws)) = (self.renderer.as_mut(), self.window_state.as_mut()) {
                    if new_size.width > 0 && new_size.height > 0 {
                        ws.size = new_size;
                        if let Err(e) = renderer.resize(new_size) {
                            self.fatal_error = Some(anyhow::anyhow!("resize failed: {:?}", e));
                            event_loop.exit();
                        }
                    }
                }
            }
            WindowEvent::RedrawRequested => {
                if let Some(renderer) = self.renderer.as_mut() {
                    match renderer.render() {
                        Ok(_) => {
                            if self.continuous {
                                if let Some(win) = &self.window {
                                    win.request_redraw();
                                }
                            }
                        }
                        Err(e) => {
                            self.fatal_error = Some(anyhow::anyhow!("render failed: {:?}", e));
                            event_loop.exit();
                        }
                    }
                }
            }
            other => {
                // Translate to normalized input event for future use.
                let _ = InputEvent::from_winit(&other);
            }
        }
    }
}

/// Run the application using winit 0.30 Application API.
pub fn run(title: String, width: u32, height: u32, clear_color: [f64; 4]) -> Result<()> {
    // Initialize logging
    let _ = env_logger::try_init();
    info!("Starting runtime (application API) with title '{}'", title);

    // Create EventLoop and run the application. The ActiveEventLoop is provided
    // by winit to ApplicationHandler callbacks; do not construct it manually.
    let event_loop = EventLoop::new()?;
    event_loop.set_control_flow(ControlFlow::Poll);

    // Create the app and run it using the event loop's run_app method.
    let mut app = App::new(title, width, height, clear_color);

    event_loop
        .run_app(&mut app)
        .map_err(|e| anyhow::anyhow!("run_app failed: {:?}", e))?;

    // Return fatal error if recorded.
    if let Some(err) = app.fatal_error {
        return Err(err);
    }

    Ok(())
}
