use anyhow::Result;
use log::info;
use std::sync::{Arc, Mutex};
use winit::application::ApplicationHandler;
use winit::event::WindowEvent;
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop};
use winit::window::{Window, WindowId};

use crate::window_state::WindowState;
use zaroxi_engine_input::event::Event as InputEvent;

#[cfg(feature = "render_integration")]
use zaroxi_engine_render::{Renderer, RenderLayout, Rect, UiBlock};

#[cfg(feature = "render_integration")]
use zaroxi_app::AppState;

/// Minimal engine application that implements the winit 0.30 ApplicationHandler
/// lifecycle. This keeps the runtime small and focused on window + renderer.
pub struct App {
    title: String,
    width: u32,
    height: u32,
    clear_color: [f64; 4],

    window: Option<Arc<Window>>,
    #[cfg(feature = "render_integration")]
    renderer: Option<Renderer<'static>>,
    window_state: Option<WindowState>,
    fatal_error: Option<anyhow::Error>,

    continuous: bool,
    /// Shared app state (read-only rendering & command dispatch).
    #[cfg(feature = "render_integration")]
    app_state: Option<Arc<Mutex<AppState>>>,
}

impl App {
    #[cfg(feature = "render_integration")]
    pub fn new(title: String, width: u32, height: u32, clear_color: [f64; 4], app_state: Arc<Mutex<AppState>>) -> Self {
        Self {
            title,
            width,
            height,
            clear_color,
            window: None,
            renderer: None,
            window_state: None,
            fatal_error: None,
            continuous: false,
            app_state: Some(app_state),
        }
    }

    #[cfg(not(feature = "render_integration"))]
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

#[cfg(feature = "render_integration")]
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

                let _app_state = self.app_state.as_ref().expect("app_state missing").clone();

                match pollster::block_on(Renderer::new(window_ref, self.clear_color)) {
                    Ok(renderer) => {
                        self.renderer = Some(renderer);
                        // Request an initial redraw.
                        info!("requesting initial redraw");
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
                if let (Some(renderer), Some(app_state)) = (self.renderer.as_mut(), self.app_state.as_ref()) {
                    // Lock app state for reading.
                    let state = app_state.lock().unwrap();
                    info!("received RedrawRequested");
                    info!("entering render_with_layout");
                    // Resolve a simple layout model from app_state and window size.
                    // The app/layout layer owns these rules; runtime here performs a
                    // minimal mapping to pixel rects for the renderer. Crucially,
                    // visibility/size of panels comes from app state (panels/assistant).
                    let ws = self.window_state.as_ref().unwrap();
                    let sz = ws.size;
                    let width = sz.width as f32;
                    let height = sz.height as f32;

                    // Log app state summary before layout resolution for traceability.
                    // Panels visibility is derived from the app-owned panel entries.
                    let panels_visible = state.app_panels.iter().any(|p| p.id == "bottom_panel" && p.visible);
                    info!(
                        "[runtime] app_state summary: title='{}', assistant_visible={}, panels_visible={}, open_docs={}",
                        state.config.title,
                        state.assistant.visible,
                        panels_visible,
                        state.editor.open_documents.len()
                    );

                    // Layout metrics (tunable by app/layout later)
                    let title_h = 48.0f32;
                    let status_h = 24.0f32;
                    let sidebar_w = 260.0f32;
                    // right panel width depends on assistant visibility
                    let right_w = if state.assistant.visible { 320.0f32 } else { 0.0f32 };
                    // bottom panel height derived from the app-owned panel entries
                    let bottom_h = if state.app_panels.iter().any(|p| p.id == "bottom_panel" && p.visible) { 200.0f32 } else { 0.0f32 };

                    // Compute rects while honoring visibility (zero-size when hidden)
                    let title_bar = Rect { x: 0.0, y: 0.0, w: width, h: title_h };
                    let sidebar = Rect { x: 0.0, y: title_h, w: sidebar_w, h: height - title_h - status_h.max(0.0) };
                    let right_panel = Rect { x: width - right_w, y: title_h, w: right_w, h: height - title_h - status_h.max(0.0) };
                    let bottom_panel = Rect { x: sidebar_w, y: height - status_h - bottom_h, w: width - sidebar_w - right_w, h: bottom_h };
                    let editor = Rect { x: sidebar_w, y: title_h, w: (width - sidebar_w - right_w).max(0.0), h: (height - title_h - status_h - bottom_h).max(0.0) };
                    let status_bar = Rect { x: 0.0, y: height - status_h, w: width, h: status_h };

                    // Resolve semantic colors from app state (system dark assumed false for now).
                    let sem = state.theme_mode.colors(false);

                    // Build the resolved RenderLayout that will be consumed by the renderer.
                    let mut layout = RenderLayout {
                        title_bar,
                        sidebar,
                        editor,
                        right_panel,
                        bottom_panel,
                        status_bar,
                        colors: sem,
                    };

                    // Convert app-owned panels into renderer-facing UiBlock descriptors.
                    // Runtime/app are responsible for mapping stable panel ids to rects
                    // and for selecting visual tokens (colors). This keeps the renderer
                    // generic and free of application-specific logic.
                    let render_panels = zaroxi_app::view_model::to_render_panels(&*state);
                    let sem = state.theme_mode.colors(false);
                    let mut render_blocks: Vec<zaroxi_engine_render::UiBlock> = Vec::new();
                    for p in &render_panels {
                        let target = match p.id.as_str() {
                            "titlebar" => layout.title_bar,
                            "sidebar" => layout.sidebar,
                            "editor" => layout.editor,
                            "right_panel" => layout.right_panel,
                            "bottom_panel" => layout.bottom_panel,
                            "status_bar" => layout.status_bar,
                            other => {
                                info!("unknown panel id '{}', skipping", other);
                                continue;
                            }
                        };

                        let header_color = match p.id.as_str() {
                            "titlebar" => sem.title_bar_background,
                            _ => sem.panel_header_background,
                        };

                        let content_color = match p.id.as_str() {
                            "titlebar" => sem.app_chrome_background,
                            "sidebar" => sem.sidebar_background,
                            "editor" => sem.editor_background,
                            "right_panel" => sem.assistant_panel_background,
                            "bottom_panel" => sem.panel_background,
                            "status_bar" => sem.status_bar_background,
                            _ => sem.panel_background,
                        };

                        render_blocks.push(zaroxi_engine_render::UiBlock {
                            id: p.id.clone(),
                            title: p.title.clone(),
                            content: p.content.clone(),
                            visible: p.visible,
                            rect: target,
                            header_color: Some(header_color),
                            content_color: Some(content_color),
                        });
                    }
                    log::debug!("[runtime] render_blocks count = {}", render_blocks.len());

                    // Log resolved layout for debugging first frame rendering.
                    log::debug!("[runtime] resolved layout: {:?}", layout);

                    match renderer.render_with_layout(&*state, &layout, &render_blocks) {
                        Ok(_) => {
                            info!("render_with_layout completed OK");
                            // Only request redraw when continuous mode is explicitly enabled.
                            if self.continuous {
                                if let Some(win) = &self.window {
                                    info!("continuous mode active; requesting redraw");
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

#[cfg(feature = "render_integration")]
/// Run the application using winit 0.30 Application API.
pub fn run(title: String, width: u32, height: u32, clear_color: [f64; 4], app_state: Arc<Mutex<AppState>>) -> Result<()> {
    // Initialize logging
    let _ = env_logger::try_init();
    info!("Starting runtime (application API) with title '{}'", title);

    // Create EventLoop and run the application. The ActiveEventLoop is provided
    // by winit to ApplicationHandler callbacks; do not construct it manually.
    let event_loop = EventLoop::new()?;
    event_loop.set_control_flow(ControlFlow::Poll);

    // Create the app and run it using the event loop's run_app method.
    let mut app = App::new(title, width, height, clear_color, app_state);

    event_loop
        .run_app(&mut app)
        .map_err(|e| anyhow::anyhow!("run_app failed: {:?}", e))?;

    // Return fatal error if recorded.
    if let Some(err) = app.fatal_error {
        return Err(err);
    }

    Ok(())
}

#[cfg(not(feature = "render_integration"))]
/// Stubbed runtime entry when render integration is disabled.
///
/// This returns an error signaling that the runtime's rendering integration
/// is intentionally disabled in the current build. Enable the
/// `render_integration` feature to enable full runtime behavior.
pub fn run(_title: String, _width: u32, _height: u32, _clear_color: [f64; 4], _app_state: Arc<Mutex<zaroxi_app::AppState>>) -> Result<()> {
    Err(anyhow::anyhow!("runtime render integration disabled; build with feature=\"render_integration\" to enable"))
}
