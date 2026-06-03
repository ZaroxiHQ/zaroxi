/*!
GuiApp implementation and winit ApplicationHandler lifecycle methods.
This file contains the GuiApp struct and its ApplicationHandler impl
(moved out of the large `window.rs` to make the module tree clearer).

Phase 28: added cursor hover tracking and widget-tree hit-testing.
*/

use pollster;
use winit::{
    dpi::PhysicalPosition,
    event::{ElementState, MouseButton, StartCause, WindowEvent},
    event_loop::ControlFlow,
    window::WindowAttributes,
};

use crate::gui::{ShellFrame, ShellWorkContent};

/// Small application handler that owns the engine window handle and the ShellFrame
/// snapshot. Lifecycle methods handle window creation and redraw requests.
/// The window stays hidden until the first full renderer frame completes,
/// avoiding any visible bootstrap/fallback composition flicker.
pub struct GuiApp {
    pub window_attributes: WindowAttributes,
    pub title: String,
    pub maybe_window: Option<zaroxi_core_engine_window::ZaroxiWindow>,
    pub shell: ShellFrame,
    /// Live workspace content snapshot built from DesktopComposition.
    /// Applied to `shell.work_content` before each redraw so the GPU
    /// window renders live session data (editor body, tabs, explorer, etc.).
    pub work_content: Option<ShellWorkContent>,
    pub requested_initial_frame: bool,
    pub already_logged_existing: bool,
    pub first_render_shown: bool,
    /// Cached widget tree built on each redraw, used for hit-testing.
    pub widget_tree: Option<zaroxi_core_engine_ui::ShellWidgetTree>,
    /// Index of the currently hovered widget in the tree, if any.
    pub hovered_widget_idx: Option<usize>,
    /// Most recent cursor position for hit-testing after resize/redraw.
    pub cursor_pos: Option<PhysicalPosition<f64>>,
    /// Whether a scrollbar drag is active and its tracked widget index.
    pub scrollbar_drag: Option<(usize, f32)>,
    /// Widget index currently pressed (for button activation).
    pub pressed_widget_idx: Option<usize>,
    /// Scroll offset for editor scrollbar (0.0..1.0).
    pub editor_scroll_offset: f32,
    /// Scroll offset for terminal scrollbar (0.0..1.0).
    pub terminal_scroll_offset: f32,
}

impl winit::application::ApplicationHandler for GuiApp {
    fn new_events(&mut self, active_loop: &winit::event_loop::ActiveEventLoop, cause: StartCause) {
        // Create the window once on Init (or when resumed on some platforms).
        if self.maybe_window.is_none() && matches!(cause, StartCause::Init) {
            eprintln!("GuiApp: attempting to create window (StartCause::Init)");
            match active_loop.create_window(self.window_attributes.clone()) {
                Ok(w) => {
                    let zaroxi_w = zaroxi_core_engine_window::ZaroxiWindow::from_window(w);
                    let wid = zaroxi_w.window().id();
                    eprintln!("GuiApp: created engine window id={:?}", wid);
                    zaroxi_w.window().set_title(&self.title);
                    let _ = zaroxi_w.window().set_outer_position(PhysicalPosition::new(100, 100));
                    self.maybe_window = Some(zaroxi_w);

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
            if !self.already_logged_existing {
                eprintln!("GuiApp: new_events called but window already created");
                self.already_logged_existing = true;
            }
        } else {
            eprintln!("GuiApp: new_events called with cause={:?} (no creation)", cause);
        }
    }

    fn resumed(&mut self, active_loop: &winit::event_loop::ActiveEventLoop) {
        if self.maybe_window.is_none() {
            eprintln!("GuiApp: resumed -> attempting to create window");
            match active_loop.create_window(self.window_attributes.clone()) {
                Ok(w) => {
                    let zaroxi_w = zaroxi_core_engine_window::ZaroxiWindow::from_window(w);
                    let wid = zaroxi_w.window().id();
                    eprintln!("GuiApp: created engine window on resumed id={:?}", wid);
                    self.maybe_window = Some(zaroxi_w);

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
        if self.requested_initial_frame {
            if let Some(z) = self.maybe_window.as_ref() {
                eprintln!("GuiApp: about_to_wait -> requesting initial redraw (engine window)");
                let _ = z.window().request_redraw();
            }
            self.requested_initial_frame = false;
            active_loop.set_control_flow(ControlFlow::Wait);
            eprintln!("GuiApp: about_to_wait -> switched control flow back to Wait");
        }
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
            WindowEvent::Resized(size) => {
                if let Some(z) = self.maybe_window.as_mut() {
                    z.update_size(size.width, size.height);
                    eprintln!("GuiApp: Resized -> {size:?}, requesting redraw (engine window)");
                    let _ = z.window().request_redraw();
                }
            }
            WindowEvent::ScaleFactorChanged { .. } => {
                if let Some(z) = self.maybe_window.as_ref() {
                    eprintln!("GuiApp: ScaleFactorChanged -> requesting redraw (engine window)");
                    let _ = z.window().request_redraw();
                }
            }
            WindowEvent::CursorMoved { position, .. } => {
                self.cursor_pos = Some(position);
                // Scrollbar drag: update thumb position from cursor
                if let Some((drag_idx, start_y)) = self.scrollbar_drag {
                    let delta = position.y as f32 - start_y;
                    if let Some(ref tree) = self.widget_tree {
                        if let Some(w) = tree.widgets.get(drag_idx) {
                            if let zaroxi_core_engine_ui::ShellWidget::ScrollbarTrack {
                                track_rect,
                                ..
                            } = w
                            {
                                let track_h = track_rect.height;
                                let thumb_h = track_h * 0.25;
                                let travel = (track_h - thumb_h).max(1.0);
                                let raw_offset = delta / travel;
                                let clamped = raw_offset.clamp(0.0, 1.0);
                                // Determine which scrollbar: editor (index 1) or terminal (index 0)
                                let is_editor = matches!(
                                    w,
                                    zaroxi_core_engine_ui::ShellWidget::ScrollbarTrack {
                                        id: zaroxi_core_engine_ui::WidgetId::Scrollbar { index: 1 },
                                        ..
                                    }
                                );
                                if is_editor {
                                    self.editor_scroll_offset = clamped;
                                } else {
                                    self.terminal_scroll_offset = clamped;
                                }
                                if let Some(z) = self.maybe_window.as_ref() {
                                    let _ = z.window().request_redraw();
                                }
                                return;
                            }
                        }
                    }
                }
                // Normal hover tracking
                if let Some(ref tree) = self.widget_tree {
                    let new_hover = tree.hit_test(position.x as f32, position.y as f32);
                    if new_hover != self.hovered_widget_idx {
                        if let Some(t) = self.widget_tree.as_mut() {
                            t.clear_all_hover();
                            if let Some(idx) = new_hover {
                                t.set_state_at(idx, zaroxi_core_engine_ui::InteractionState::Hover);
                            }
                        }
                        self.hovered_widget_idx = new_hover;
                        if let Some(z) = self.maybe_window.as_ref() {
                            let _ = z.window().request_redraw();
                        }
                    }
                }
            }
            WindowEvent::CursorLeft { .. } => {
                self.hovered_widget_idx = None;
                self.cursor_pos = None;
                if let Some(t) = self.widget_tree.as_mut() {
                    t.clear_all_hover();
                }
                if let Some(z) = self.maybe_window.as_ref() {
                    let _ = z.window().request_redraw();
                }
            }
            WindowEvent::MouseInput { state, button, .. } => {
                if button == MouseButton::Left {
                    let hit = self.cursor_pos.and_then(|pos| {
                        self.widget_tree
                            .as_ref()
                            .and_then(|t| t.hit_test(pos.x as f32, pos.y as f32))
                    });
                    match state {
                        ElementState::Pressed => {
                            self.pressed_widget_idx = hit;
                            if let Some(idx) = hit {
                                if let Some(t) = self.widget_tree.as_mut() {
                                    // Check if scrollbar thumb was pressed
                                    if let Some(w) = t.widgets.get(idx) {
                                        if matches!(
                                            w,
                                            zaroxi_core_engine_ui::ShellWidget::ScrollbarTrack { .. }
                                        ) {
                                            if let Some(pos) = self.cursor_pos {
                                                self.scrollbar_drag = Some((idx, pos.y as f32));
                                                t.set_state_at(
                                                    idx,
                                                    zaroxi_core_engine_ui::InteractionState::Active,
                                                );
                                            }
                                        } else {
                                            t.set_state_at(
                                                idx,
                                                zaroxi_core_engine_ui::InteractionState::Active,
                                            );
                                        }
                                    }
                                    if let Some(z) = self.maybe_window.as_ref() {
                                        let _ = z.window().request_redraw();
                                    }
                                }
                            }
                        }
                        ElementState::Released => {
                            // Button activation: only fire if released on the same widget
                            if let Some(pressed) = self.pressed_widget_idx.take() {
                                if let Some(t) = self.widget_tree.as_mut() {
                                    t.set_state_at(
                                        pressed,
                                        zaroxi_core_engine_ui::InteractionState::Normal,
                                    );
                                    t.clear_all_hover();
                                }
                            }
                            // End scrollbar drag
                            if self.scrollbar_drag.take().is_some() {
                                if let Some(t) = self.widget_tree.as_mut() {
                                    t.clear_all_hover();
                                }
                            }
                            if let Some(z) = self.maybe_window.as_ref() {
                                let _ = z.window().request_redraw();
                            }
                        }
                    }
                }
            }
            WindowEvent::RedrawRequested => {
                eprintln!("GuiApp: RedrawRequested received");
                if let Some(z) = self.maybe_window.as_mut() {
                    let _ = z.window().pre_present_notify();

                    let (sw, sh) = z.size();
                    if sw > 0 && sh > 0 {
                        let actual = crate::gui::Size { width: sw, height: sh };
                        self.shell = crate::gui::ShellFrame::new(actual);
                    }

                    self.shell.work_content = self.work_content.clone();

                    let rects = super::frame::build_overlay_rects(&self.shell);
                    let backend_text_ops = rects.len();

                    let theme = zaroxi_core_engine_ui::EngineTheme::dark();

                    // Build the engine-side widget tree for hover tracking.
                    let layout = zaroxi_core_engine_ui::ShellLayout::from_window_size(sw, sh);
                    let mut widget_tree =
                        zaroxi_core_engine_ui::build_shell_widget_tree(&layout, &theme);
                    // Re-apply hover state if cursor is over a widget.
                    if let Some(pos) = self.cursor_pos {
                        let hit = widget_tree.hit_test(pos.x as f32, pos.y as f32);
                        if let Some(idx) = hit {
                            widget_tree
                                .set_state_at(idx, zaroxi_core_engine_ui::InteractionState::Hover);
                        }
                        self.hovered_widget_idx = hit;
                    }
                    self.widget_tree = Some(widget_tree.clone());

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

                    let render_layout = zaroxi_core_engine_render::RenderLayout {
                        title_bar: find_rect("toolbar"),
                        sidebar: find_rect("sidebar"),
                        editor: find_rect("center_editor"),
                        right_panel: find_rect("ai_panel_content"),
                        bottom_panel: find_rect("bottom_dock"),
                        status_bar: find_rect("status_bar"),
                        colors: zaroxi_core_engine_render::PanelColors {
                            panel_header_background: theme.panel_header_bg().to_array(),
                            panel_background: theme.surface_default.to_array(),
                        },
                    };

                    // Extract live work content for dynamic shell text.
                    let editor_body_text = self
                        .shell
                        .work_content
                        .as_ref()
                        .and_then(|w| w.editor_body.as_ref())
                        .map(|cv| {
                            let mut lines_with_numbers = String::new();
                            for (i, line) in cv.lines.iter().enumerate() {
                                let num = i + 1;
                                lines_with_numbers.push_str(&format!("{:>3} │ {}\n", num, line));
                            }
                            lines_with_numbers
                        })
                        .unwrap_or_else(|| "fn main() {\n    println!(\"hello\");\n}".to_string());

                    // Extract cursor position from editor content.
                    let editor_cursor_line = self
                        .shell
                        .work_content
                        .as_ref()
                        .and_then(|w| w.editor_body.as_ref())
                        .map(|cv| cv.cursor_line)
                        .unwrap_or(0);
                    let editor_cursor_col = self
                        .shell
                        .work_content
                        .as_ref()
                        .and_then(|w| w.editor_body.as_ref())
                        .map(|cv| cv.cursor_col)
                        .unwrap_or(0);

                    // Produce syntax-colored spans from editor content using tree-sitter.
                    let editor_spans: Option<Vec<(String, [f32; 4])>> = self
                        .shell
                        .work_content
                        .as_ref()
                        .and_then(|w| w.editor_body.as_ref())
                        .map(|cv| super::syntax_color::colorize_source(&cv.lines));

                    let tab_labels = self
                        .shell
                        .work_content
                        .as_ref()
                        .and_then(|w| w.editor_tabs.clone())
                        .unwrap_or_else(|| {
                            vec!["main.rs".into(), "lib.rs".into(), "mod.rs".into()]
                        });
                    let tab_title = tab_labels.first().cloned().unwrap_or_else(|| "main.rs".into());
                    let tab_content: String =
                        tab_labels.iter().skip(1).cloned().collect::<Vec<_>>().join("  ");
                    let breadcrumb_label = self
                        .shell
                        .work_content
                        .as_ref()
                        .and_then(|w| w.editor_breadcrumb.clone())
                        .unwrap_or_else(|| "src > app > main.rs".into());
                    let sidebar_items = self
                        .shell
                        .work_content
                        .as_ref()
                        .and_then(|w| w.explorer_items.clone())
                        .map(|items| {
                            let mut text = String::from("PROJECT\n");
                            for item in &items {
                                text.push_str(&format!("  {}\n", item));
                            }
                            text.push_str("GIT\n  clean\nOUTLINE\n  fn main\n  struct App");
                            text
                        })
                        .unwrap_or_else(|| {
                            "PROJECT\n  src/main.rs\n  src/lib.rs\n  Cargo.toml\nGIT\n  clean\nOUTLINE\n  fn main\n  struct App".to_string()
                        });

                    let region_to_block =
                        |r: &crate::gui::ShellRegion| -> zaroxi_core_engine_render::UiBlock {
                            let rect = zaroxi_core_engine_render::Rect {
                                x: r.rect.x as f32,
                                y: r.rect.y as f32,
                                w: r.rect.width as f32,
                                h: r.rect.height as f32,
                            };

                            match r.id {
                                "toolbar" => zaroxi_core_engine_render::UiBlock {
                                    id: r.id.to_string(),
                                    title: "Zaroxi Studio".to_string(),
                                    content: String::new(),
                                    visible: true,
                                    rect,
                                    header_color: Some(theme.status_bar_background.to_array()),
                                    content_color: None,
                                    corner_radius: 0.0,
                                    border_color: Some(theme.divider_default.to_array()),
                                    border_width: 1.0,
                                    header_only: true,
                                    content_spans: None,
                                    cursor_line: None,
                                    cursor_col: None,
                                    highlight_active_line: false,
                                    text_color: Some(theme.text_primary.to_array()),
                                },
                                "app_rail" => zaroxi_core_engine_render::UiBlock {
                                    id: r.id.to_string(),
                                    title: String::new(),
                                    content: String::new(),
                                    visible: true,
                                    rect,
                                    header_color: Some(theme.activity_rail_background.to_array()),
                                    content_color: Some(theme.activity_rail_background.to_array()),
                                    corner_radius: 0.0,
                                    border_color: Some(
                                        theme.divider_default.adjust_brightness(0.85).to_array(),
                                    ),
                                    border_width: 1.0,
                                    header_only: false,
                                    content_spans: None,
                                    cursor_line: None,
                                    cursor_col: None,
                                    highlight_active_line: false,
                                    text_color: None,

                                },
                                "sidebar" => zaroxi_core_engine_render::UiBlock {
                                    id: r.id.to_string(),
                                    title: "Explorer".to_string(),
                                    content: sidebar_items.clone(),
                                    visible: true,
                                    rect,
                                    header_color: Some(theme.sidebar_background.to_array()),
                                    content_color: Some(theme.sidebar_background.to_array()),
                                    corner_radius: 0.0,
                                    border_color: None,
                                    border_width: 0.0,
                                    header_only: false,
                                    content_spans: None,
                                    cursor_line: None,
                                    cursor_col: None,
                                    highlight_active_line: false,
                                    text_color: None,

                                },
                                "editor_tabs" => zaroxi_core_engine_render::UiBlock {
                                    id: r.id.to_string(),
                                    title: tab_title.clone(),
                                    content: tab_content.clone(),
                                    visible: true,
                                    rect,
                                    header_color: Some(theme.tab_strip_background.to_array()),
                                    content_color: None,
                                    corner_radius: 4.0,
                                    border_color: Some(theme.divider_default.to_array()),
                                    border_width: 1.0,
                                    header_only: true,
                                    content_spans: None,
                                    cursor_line: None,
                                    cursor_col: None,
                                    highlight_active_line: false,
                                    text_color: Some(theme.text_primary.to_array()),
                                },
                                "breadcrumb" => zaroxi_core_engine_render::UiBlock {
                                    id: r.id.to_string(),
                                    title: breadcrumb_label.clone(),
                                    content: String::new(),
                                    visible: true,
                                    rect,
                                    header_color: Some(
                                        theme.editor_background.adjust_brightness(0.97).to_array(),
                                    ),
                                    content_color: None,
                                    corner_radius: 0.0,
                                    border_color: Some(theme.divider_subtle.to_array()),
                                    border_width: 1.0,
                                    header_only: true,
                                    content_spans: None,
                                    cursor_line: None,
                                    cursor_col: None,
                                    highlight_active_line: false,
                                    text_color: Some(theme.text_muted.to_array()),
                                },
                                "center_editor" | "editor_content" => {
                                    zaroxi_core_engine_render::UiBlock {
                                        id: r.id.to_string(),
                                        title: String::new(),
                                        content: editor_body_text.clone(),
                                        visible: true,
                                        rect,
                                        header_color: Some(theme.editor_background.to_array()),
                                        content_color: Some(theme.editor_background.to_array()),
                                        corner_radius: 0.0,
                                        border_color: None,
                                        border_width: 0.0,
                                        header_only: false,
                                        content_spans: editor_spans.clone(),
                                        cursor_line: Some(editor_cursor_line),
                                        cursor_col: Some(editor_cursor_col),
                                        highlight_active_line: true,
                                        text_color: None,

                                    }
                                }
                                "minimap_lane" => zaroxi_core_engine_render::UiBlock {
                                    id: r.id.to_string(),
                                    title: String::new(),
                                    content: String::new(),
                                    visible: true,
                                    rect,
                                    header_color: Some(
                                        theme.editor_background.adjust_brightness(0.95).to_array(),
                                    ),
                                    content_color: None,
                                    corner_radius: 0.0,
                                    border_color: None,
                                    border_width: 0.0,
                                    header_only: true,
                                    content_spans: None,
                                    cursor_line: None,
                                    cursor_col: None,
                                    highlight_active_line: false,
                                    text_color: None,

                                },
                                "center_bottom_panel" => zaroxi_core_engine_render::UiBlock {
                                    id: r.id.to_string(),
                                    title: "Terminal • Problems • Output".to_string(),
                                    content: "$ cargo build\n   Compiling zaroxi v0.1.0\n    Finished dev [unoptimized]".to_string(),
                                    visible: true,
                                    rect,
                                    header_color: Some(theme.panel_header_bg().to_array()),
                                    content_color: Some(theme.bottom_panel_background.to_array()),
                                    corner_radius: 4.0,
                                    border_color: Some(theme.divider_default.to_array()),
                                    border_width: 1.0,
                                    header_only: false,
                                    content_spans: None,
                                    cursor_line: None,
                                    cursor_col: None,
                                    highlight_active_line: false,
                                    text_color: None,

                                },
                                "bottom_dock" => zaroxi_core_engine_render::UiBlock {
                                    id: r.id.to_string(),
                                    title: String::new(),
                                    content: String::new(),
                                    visible: r.rect.height > 0,
                                    rect,
                                    header_color: Some(theme.surface_default.to_array()),
                                    content_color: None,
                                    corner_radius: 0.0,
                                    border_color: None,
                                    border_width: 0.0,
                                    header_only: true,
                                    content_spans: None,
                                    cursor_line: None,
                                    cursor_col: None,
                                    highlight_active_line: false,
                                    text_color: None,

                                },
                                "ai_panel_header" => zaroxi_core_engine_render::UiBlock {
                                    id: r.id.to_string(),
                                    title: "AI Assistant".to_string(),
                                    content: String::new(),
                                    visible: true,
                                    rect,
                                    header_color: Some(theme.panel_header_bg().to_array()),
                                    content_color: None,
                                    corner_radius: 0.0,
                                    border_color: Some(theme.divider_default.to_array()),
                                    border_width: 1.0,
                                    header_only: true,
                                    content_spans: None,
                                    cursor_line: None,
                                    cursor_col: None,
                                    highlight_active_line: false,
                                    text_color: Some(theme.text_primary.to_array()),
                                },
                                "ai_panel_content" => zaroxi_core_engine_render::UiBlock {
                                    id: r.id.to_string(),
                                    title: "Assistant".to_string(),
                                    content: "No active AI session\nOpen a file and request an AI edit to get started.".to_string(),
                                    visible: true,
                                    rect,
                                    header_color: Some(theme.assistant_panel_background.to_array()),
                                    content_color: None,
                                    corner_radius: 0.0,
                                    border_color: None,
                                    border_width: 0.0,
                                    header_only: true,
                                    content_spans: None,
                                    cursor_line: None,
                                    cursor_col: None,
                                    highlight_active_line: false,
                                    text_color: None,

                                },
                                "status_bar" => zaroxi_core_engine_render::UiBlock {
                                    id: r.id.to_string(),
                                    title: String::new(),
                                    content: "Ready  Ln 22, Col 14  UTF-8  LF  Rust".to_string(),
                                    visible: true,
                                    rect,
                                    header_color: Some(theme.status_bar_background.to_array()),
                                    content_color: Some(theme.status_bar_background.to_array()),
                                    corner_radius: 4.0,
                                    border_color: Some(
                                        theme.divider_default.adjust_brightness(0.9).to_array(),
                                    ),
                                    border_width: 1.0,
                                    header_only: false,
                                    content_spans: None,
                                    cursor_line: None,
                                    cursor_col: None,
                                    highlight_active_line: false,
                                    text_color: Some(theme.text_secondary.to_array()),
                                },
                                _ => zaroxi_core_engine_render::UiBlock {
                                    id: r.id.to_string(),
                                    title: String::new(),
                                    content: String::new(),
                                    visible: true,
                                    rect,
                                    header_color: Some(theme.surface_default.to_array()),
                                    content_color: None,
                                    corner_radius: 0.0,
                                    border_color: None,
                                    border_width: 0.0,
                                    header_only: true,
                                    content_spans: None,
                                    cursor_line: None,
                                    cursor_col: None,
                                    highlight_active_line: false,
                                    text_color: None,

                                },
                            }
                        };

                    let render_blocks: Vec<zaroxi_core_engine_render::UiBlock> =
                        self.shell.regions.iter().map(region_to_block).collect();

                    match pollster::block_on(zaroxi_core_engine_render::Renderer::new(
                        z.window(),
                        [
                            theme.app_background.r as f64,
                            theme.app_background.g as f64,
                            theme.app_background.b as f64,
                            1.0,
                        ],
                    )) {
                        Ok(mut renderer) => {
                            let app_state = zaroxi_core_engine_render::renderer::core::AppState;
                            match renderer.render_with_layout(
                                &app_state,
                                &render_layout,
                                &render_blocks,
                            ) {
                                Ok(()) => {
                                    if !self.first_render_shown {
                                        let _ = z.window().set_visible(true);
                                        let _ = z.window().pre_present_notify();
                                        self.first_render_shown = true;
                                        eprintln!(
                                            "GuiApp: first full-renderer frame; window visible"
                                        );
                                    }
                                }
                                Err(e) => {
                                    eprintln!("GuiApp: render_with_layout failed: {:?}", e);
                                }
                            }
                        }
                        Err(e) => {
                            eprintln!("GuiApp: failed to create renderer: {:?}", e);
                        }
                    }

                    eprintln!(
                        "GUI_TEXT_FRAME_SUMMARY: surface={}x{} overlay_rects={} render_blocks={}",
                        sw,
                        sh,
                        backend_text_ops,
                        render_blocks.len()
                    );
                }
            }
            _ => {}
        }
    }
}
