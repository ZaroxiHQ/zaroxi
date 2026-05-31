/*!
GuiApp implementation and winit ApplicationHandler lifecycle methods.
This file contains the GuiApp struct and its ApplicationHandler impl
(moved out of the large `window.rs` to make the module tree clearer).
*/

use pollster;
use winit::{
    dpi::PhysicalPosition,
    event::{StartCause, WindowEvent},
    event_loop::ControlFlow,
    window::WindowAttributes,
};

use crate::gui::ShellFrame;

/// Small application handler that owns the engine window handle and the ShellFrame
/// snapshot. Lifecycle methods handle window creation and redraw requests.
/// The window stays hidden until the first full renderer frame completes,
/// avoiding any visible bootstrap/fallback composition flicker.
pub struct GuiApp {
    pub window_attributes: WindowAttributes,
    pub title: String,
    pub maybe_window: Option<zaroxi_core_engine_window::ZaroxiWindow>,
    /// Keep a clone of the ShellFrame so we can resolve stable region rects and theme tokens
    /// at window creation time and pass low-level draw inputs into the backend.
    pub shell: ShellFrame,
    /// Request the initial frame once after window creation to avoid a busy loop.
    pub requested_initial_frame: bool,
    /// Prevent repeated "already created" logs from flooding the terminal.
    pub already_logged_existing: bool,
    /// Track whether the first full-renderer frame has been presented and the
    /// window made visible. The window stays hidden until this flag flips so
    /// the user never sees a bootstrap/fallback composition.
    pub first_render_shown: bool,
}

impl winit::application::ApplicationHandler for GuiApp {
    fn new_events(&mut self, active_loop: &winit::event_loop::ActiveEventLoop, cause: StartCause) {
        // Create the window once on Init (or when resumed on some platforms).
        if self.maybe_window.is_none() && matches!(cause, StartCause::Init) {
            eprintln!("GuiApp: attempting to create window (StartCause::Init)");
            match active_loop.create_window(self.window_attributes.clone()) {
                Ok(w) => {
                    // Convert the raw winit Window into the engine wrapper.
                    // Keep the window hidden until the first full renderer frame completes
                    // so the user never sees a bootstrap/fallback composition.
                    let zaroxi_w = zaroxi_core_engine_window::ZaroxiWindow::from_window(w);
                    let wid = zaroxi_w.window().id();
                    eprintln!("GuiApp: created engine window id={:?}", wid);
                    zaroxi_w.window().set_title(&self.title);
                    let _ = zaroxi_w.window().set_outer_position(PhysicalPosition::new(100, 100));
                    self.maybe_window = Some(zaroxi_w);

                    // Request a single initial frame. The window will be made visible
                    // inside RedrawRequested after the full renderer produces its first frame.
                    if let Some(z) = self.maybe_window.as_ref() {
                        let _ = z.window().request_redraw();
                    }
                    active_loop.set_control_flow(ControlFlow::Wait);
                    eprintln!("GuiApp: window created (hidden); initial redraw requested");
                }
                Err(e) => {
                    eprintln!("GuiApp: failed to create window: {}", e);
                    active_loop.exit();
                }
            }
        } else if self.maybe_window.is_some() {
            // Already created; noop but only log once for diagnostics to avoid terminal bloat.
            if !self.already_logged_existing {
                eprintln!("GuiApp: new_events called but window already created");
                self.already_logged_existing = true;
            }
        } else {
            eprintln!("GuiApp: new_events called with cause={:?} (no creation)", cause);
        }
    }

    fn resumed(&mut self, active_loop: &winit::event_loop::ActiveEventLoop) {
        // Some Wayland compositors deliver readiness after `resumed`, not Init.
        // Attempt window creation here as a fallback when new_events didn't create it.
        if self.maybe_window.is_none() {
            eprintln!("GuiApp: resumed -> attempting to create window");
            match active_loop.create_window(self.window_attributes.clone()) {
                Ok(w) => {
                    // Keep the window hidden until the first full renderer frame completes.
                    let zaroxi_w = zaroxi_core_engine_window::ZaroxiWindow::from_window(w);
                    let wid = zaroxi_w.window().id();
                    eprintln!("GuiApp: created engine window on resumed id={:?}", wid);
                    self.maybe_window = Some(zaroxi_w);

                    // Request a single initial frame. The window will be made visible
                    // inside RedrawRequested after the full renderer produces its first frame.
                    if let Some(z) = self.maybe_window.as_ref() {
                        let _ = z.window().request_redraw();
                    }
                    eprintln!(
                        "GuiApp: window created on resumed (hidden); initial redraw requested"
                    );
                }
                Err(e) => {
                    eprintln!("GuiApp: resumed failed to create window: {}", e);
                    active_loop.exit();
                }
            }
        } else {
            eprintln!("GuiApp: resumed called but window already created");
        }
    }

    fn about_to_wait(&mut self, active_loop: &winit::event_loop::ActiveEventLoop) {
        // Request the initial frame once to avoid a continuous busy redraw loop.
        if self.requested_initial_frame {
            if let Some(z) = self.maybe_window.as_ref() {
                eprintln!("GuiApp: about_to_wait -> requesting initial redraw (engine window)");
                let _ = z.window().request_redraw();
            }
            self.requested_initial_frame = false;
            // After requesting the single initial frame, stop polling to avoid busy-looping.
            active_loop.set_control_flow(ControlFlow::Wait);
            eprintln!("GuiApp: about_to_wait -> switched control flow back to Wait");
        }
        // Otherwise remain idle (Wait) and let the platform wake us for real events.
    }

    fn window_event(
        &mut self,
        active_loop: &winit::event_loop::ActiveEventLoop,
        _window_id: winit::window::WindowId,
        event: WindowEvent,
    ) {
        match event {
            WindowEvent::CloseRequested => {
                active_loop.exit();
            }
            WindowEvent::Resized(_size) => {
                if let Some(z) = self.maybe_window.as_ref() {
                    eprintln!("GuiApp: Resized -> requesting redraw (engine window)");
                    let _ = z.window().request_redraw();
                }
            }
            WindowEvent::ScaleFactorChanged { .. } => {
                if let Some(z) = self.maybe_window.as_ref() {
                    eprintln!("GuiApp: ScaleFactorChanged -> requesting redraw (engine window)");
                    let _ = z.window().request_redraw();
                }
            }
            WindowEvent::RedrawRequested => {
                eprintln!("GuiApp: RedrawRequested received");
                if let Some(z) = self.maybe_window.as_ref() {
                    eprintln!("GuiApp: performing present-related nudges (engine window)");
                    let _ = z.window().pre_present_notify();

                    // Diagnostic: produce a compact per-redraw summary so we can compare
                    // the clear_present_once bootstrap path with the normal redraw path.
                    let rects = super::frame::build_overlay_rects(&self.shell);
                    let tmp_layout = std::env::temp_dir().join("zaroxi_gui_trace_layout");
                    let tmp_cosmic = std::env::temp_dir().join("zaroxi_gui_trace_cosmic_prepare");
                    let layout_present = tmp_layout.exists();
                    let cosmic_present = tmp_cosmic.exists();
                    let mut adapter_text_ops: usize = 0;
                    if layout_present {
                        if let Ok(s) = std::fs::read_to_string(&tmp_layout) {
                            if let Some(rest) = s.strip_prefix("lines=") {
                                adapter_text_ops =
                                    rest.split(" | ").filter(|p| !p.is_empty()).count();
                            }
                        }
                    }
                    let backend_text_ops = rects.len();

                    // Attempt to create and drive the full renderer path so text prepare/render
                    // are exercised. This replaces the previous "simplified path" that only
                    // did an overlay rect present and helps diagnose where text ops are lost.
                    //
                    // Build a RenderLayout and UiBlock list derived from the ShellFrame so
                    // the renderer has the same resolved regions it expects.
                    let find_rect = |id: &str| -> zaroxi_core_engine_render::Rect {
                        if let Some(r) = self.shell.regions.iter().find(|rr| rr.id == id) {
                            zaroxi_core_engine_render::Rect {
                                x: r.rect.x as f32,
                                y: r.rect.y as f32,
                                w: r.rect.width as f32,
                                h: r.rect.height as f32,
                            }
                        } else {
                            zaroxi_core_engine_render::Rect { x: 0.0, y: 0.0, w: 0.0, h: 0.0 }
                        }
                    };

                    let layout = zaroxi_core_engine_render::RenderLayout {
                        title_bar: find_rect("toolbar"),
                        sidebar: find_rect("sidebar"),
                        editor: find_rect("center_editor"),
                        right_panel: find_rect("ai_panel_content"),
                        bottom_panel: find_rect("bottom_dock"),
                        status_bar: find_rect("status_bar"),
                        colors: zaroxi_interface_theme::SemanticColors::dark(),
                    };

                    let mut render_blocks: Vec<zaroxi_core_engine_render::UiBlock> = Vec::new();
                    for r in &self.shell.regions {
                        render_blocks.push(zaroxi_core_engine_render::UiBlock {
                            id: r.id.to_string(),
                            title: r.name.to_string(),
                            content: String::new(),
                            visible: true,
                            rect: zaroxi_core_engine_render::Rect {
                                x: r.rect.x as f32,
                                y: r.rect.y as f32,
                                w: r.rect.width as f32,
                                h: r.rect.height as f32,
                            },
                            header_color: None,
                            content_color: None,
                        });
                    }

                    // Create renderer (blocking for now) and invoke the full render_with_layout path.
                    match pollster::block_on(zaroxi_core_engine_render::Renderer::new(
                        z.window(),
                        [0.051, 0.054, 0.062, 1.0],
                    )) {
                        Ok(mut renderer) => {
                            // AppState is a zero-sized stub exposed by the renderer crate for compatibility.
                            // Use the core module path where AppState is declared.
                            let app_state = zaroxi_core_engine_render::renderer::core::AppState;
                            match renderer.render_with_layout(&app_state, &layout, &render_blocks) {
                                Ok(()) => {
                                    eprintln!(
                                        "GuiApp: full renderer path executed (render_with_layout succeeded)"
                                    );
                                    // Show the window on the very first successful full-renderer
                                    // frame so the user never sees a bootstrap/fallback composition.
                                    if !self.first_render_shown {
                                        let _ = z.window().set_visible(true);
                                        let _ = z.window().pre_present_notify();
                                        self.first_render_shown = true;
                                        eprintln!(
                                            "GuiApp: first full-renderer frame presented; window now visible"
                                        );
                                    }
                                }
                                Err(e) => {
                                    eprintln!(
                                        "GuiApp: renderer.render_with_layout failed: {:?}",
                                        e
                                    );
                                }
                            }
                        }
                        Err(e) => {
                            eprintln!("GuiApp: failed to create full renderer: {:?}", e);
                        }
                    }

                    // Recompute core_text_ops and pipeline/fallback markers for the summary logs.
                    let core_text_ops: usize = {
                        // Try to probe queued_len via a temporary renderer instance is non-trivial here;
                        // conservatively assume renderer attempted to queue text if adapter/layout present.
                        if layout_present { 1 } else { 0 }
                    };
                    let fallback_used = layout_present && !cosmic_present;

                    eprintln!(
                        "GUI_TEXT_FRAME_SUMMARY: path=redraw_requested adapter_text_ops={} backend_text_ops={} core_text_ops={} cosmic_prepare_called={} glyphs=0 atlas_entries=0 pipeline_render_called=unknown overlay_rects={} fallback_used={}",
                        adapter_text_ops,
                        backend_text_ops,
                        core_text_ops,
                        if cosmic_present { "true" } else { "false" },
                        backend_text_ops,
                        if fallback_used { "true" } else { "false" }
                    );

                    // Hard-checks for broken links (diagnostic only).
                    if adapter_text_ops > 0 && backend_text_ops == 0 {
                        eprintln!("GUI_TEXT_BROKEN_LINK: adapter->backend");
                    }
                    if backend_text_ops > 0 && core_text_ops == 0 {
                        eprintln!("GUI_TEXT_BROKEN_LINK: backend->core");
                    }

                    // If we later add a wgpu clear/present path we will call it here.
                }
            }
            _ => {}
        }
    }
}
