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
    /// Manual cursor position set by mouse clicks in the editor area.
    pub editor_cursor_line: usize,
    pub editor_cursor_col: usize,
    /// Drag-start line/col for selection extending.
    pub selection_anchor: Option<(usize, usize)>,
    /// Theme mode: Dark, Light, or System (default).
    pub theme_mode: zaroxi_interface_theme::theme::ZaroxiTheme,
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
                                // Editor area click: position cursor
                                if let Some(pos) = self.cursor_pos {
                                    let editor_region =
                                        crate::gui::region_dispatch::find_region_by_role(
                                            &self.shell.regions,
                                            zaroxi_core_engine_style::PanelRole::ContentArea,
                                        );
                                    if let Some(ed) = editor_region {
                                        let ex = ed.rect.x as f32;
                                        let ey = ed.rect.y as f32;
                                        let px = pos.x as f32;
                                        let py = pos.y as f32;
                                        if px >= ex
                                            && py >= ey
                                            && px < ex + ed.rect.width as f32
                                            && py < ey + ed.rect.height as f32
                                        {
                                            let content_pad = 8.0;
                                            let header_h = 28.0;
                                            let line_h = 16.0;
                                            let char_w = 8.0;
                                            let content_x = ex + content_pad;
                                            let content_y = ey + header_h + content_pad;
                                            let rel_y = py - content_y;
                                            let rel_x = px - content_x;
                                            let visible_line = (rel_y / line_h).max(0.0) as usize;
                                            let col = (rel_x / char_w).max(0.0) as usize;

                                            // Compute usable height and viewport for scroll offset
                                            let usable_h = ed.rect.height as f32
                                                - header_h
                                                - content_pad * 2.0;

                                            // Convert visible line to absolute line using scroll offset
                                            let total_lines = self
                                                .shell
                                                .work_content
                                                .as_ref()
                                                .and_then(|w| w.editor_body.as_ref())
                                                .map(|cv| cv.lines.len().max(1))
                                                .unwrap_or(1);
                                            let visible_lines_c =
                                                (usable_h / line_h).max(1.0) as usize;
                                            let max_scroll_c = (total_lines
                                                .saturating_sub(visible_lines_c))
                                            .max(1);
                                            let first_visible = (self.editor_scroll_offset
                                                * max_scroll_c as f32)
                                                as usize;
                                            let absolute_line = first_visible + visible_line;
                                            self.editor_cursor_line = absolute_line;
                                            self.editor_cursor_col = col;
                                            self.selection_anchor = Some((absolute_line, col));
                                            if let Some(z) = self.maybe_window.as_ref() {
                                                let _ = z.window().request_redraw();
                                            }
                                        }
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
                if let Some(z) = self.maybe_window.as_mut() {
                    let _ = z.window().pre_present_notify();

                    let (sw, sh) = z.size();
                    if sw > 0 && sh > 0 {
                        let system_is_dark = z
                            .window()
                            .theme()
                            .map(|t| matches!(t, winit::window::Theme::Dark))
                            .unwrap_or(true);
                        let resolved = self.theme_mode.resolve(system_is_dark);
                        let actual = crate::gui::Size { width: sw, height: sh };
                        self.shell = crate::gui::ShellFrame::new(actual, resolved);
                    }

                    self.shell.work_content = self.work_content.clone();

                    let system_is_dark = z
                        .window()
                        .theme()
                        .map(|t| matches!(t, winit::window::Theme::Dark))
                        .unwrap_or(true);
                    let variant = self.theme_mode.resolve(system_is_dark);
                    let sem = variant.colors(false);

                    if !self.first_render_shown {
                        eprintln!(
                            "ZAROXI_THEME_TRACE: mode={:?} system_is_dark={} resolved={:?}",
                            self.theme_mode, system_is_dark, variant
                        );
                        eprintln!(
                            "ZAROXI_THEME_TRACE: sem.shell_background={:?} sem.app_background={:?} sem.editor_background={:?}",
                            sem.shell_background, sem.app_background, sem.editor_background
                        );
                    }

                    let tokens = super::style_tokens_adapter::resolve_style_tokens(
                        &sem,
                        &Default::default(),
                    );

                    if !self.first_render_shown {
                        eprintln!(
                            "ZAROXI_STYLE_TOKENS: app_bg={:?} titlebar_bg={:?} editor_bg={:?} sidebar_bg={:?}",
                            tokens.app_background.to_array(),
                            tokens.titlebar_background.to_array(),
                            tokens.editor_content_background.to_array(),
                            tokens.sidebar_background.to_array(),
                        );
                    }

                    // Build the engine-side widget tree for hover tracking.
                    let layout = zaroxi_core_engine_ui::ShellLayout::from_window_size(sw, sh);
                    let mut widget_tree =
                        zaroxi_core_engine_ui::build_shell_widget_tree(&layout, &tokens);
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

                    // Update scrollbar thumb positions to reflect scroll state.
                    // Update scrollbar thumb positions to reflect scroll state.
                    if let Some(ref mut tree) = self.widget_tree {
                        for i in 0..tree.widgets.len() {
                            let new_widget = match &tree.widgets[i] {
                                zaroxi_core_engine_ui::ShellWidget::ScrollbarTrack {
                                    id,
                                    track_rect,
                                    thumb_rect,
                                    track_fill,
                                    thumb_fill,
                                    state,
                                } => {
                                    let offset = if matches!(
                                        id,
                                        zaroxi_core_engine_ui::WidgetId::Scrollbar { index: 1 }
                                    ) {
                                        self.editor_scroll_offset
                                    } else {
                                        self.terminal_scroll_offset
                                    };
                                    let travel = (track_rect.height - thumb_rect.height).max(1.0);
                                    let new_y = track_rect.y + offset * travel;
                                    let mut new_thumb = *thumb_rect;
                                    new_thumb.y = new_y;
                                    Some(zaroxi_core_engine_ui::ShellWidget::ScrollbarTrack {
                                        id: id.clone(),
                                        track_rect: *track_rect,
                                        thumb_rect: new_thumb,
                                        track_fill: *track_fill,
                                        thumb_fill: *thumb_fill,
                                        state: *state,
                                    })
                                }
                                _ => None,
                            };
                            if let Some(w) = new_widget {
                                tree.widgets[i] = w;
                            }
                        }
                    }

                    let find_rect = |role: zaroxi_core_engine_style::PanelRole| -> zaroxi_core_engine_render::Rect {
                        if let Some(r) =
                            crate::gui::region_dispatch::find_region_by_role(&self.shell.regions, role)
                        {
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
                        title_bar: find_rect(zaroxi_core_engine_style::PanelRole::TopBar),
                        sidebar: find_rect(zaroxi_core_engine_style::PanelRole::SidePanel),
                        editor: find_rect(zaroxi_core_engine_style::PanelRole::ContentArea),
                        right_panel: find_rect(
                            zaroxi_core_engine_style::PanelRole::AuxiliaryPanelContent,
                        ),
                        bottom_panel: find_rect(zaroxi_core_engine_style::PanelRole::BottomDock),
                        status_bar: find_rect(zaroxi_core_engine_style::PanelRole::StatusBar),
                        colors: zaroxi_core_engine_render::PanelColors {
                            panel_header_background: tokens.panel_header_background.to_array(),
                            panel_background: tokens.app_background.to_array(),
                            editor_cursor: tokens.editor_cursor.to_array(),
                            editor_selection: tokens.editor_selection.to_array(),
                            editor_line_highlight: tokens.editor_line_highlight.to_array(),
                            text_default: tokens.text_primary.to_array(),
                        },
                    };

                    // Extract live work content for dynamic shell text.
                    let editor_rect = find_rect(zaroxi_core_engine_style::PanelRole::ContentArea);
                    let editor_body_text = self
                        .shell
                        .work_content
                        .as_ref()
                        .and_then(|w| w.editor_body.as_ref())
                        .map(|cv| {
                            let line_h = 16.0;
                            let pad = 8.0;
                            let header_h = 28.0;
                            let usable_h = (editor_rect.h - header_h - pad * 2.0).max(0.0);
                            let visible_lines = (usable_h / line_h).max(1.0) as usize;
                            let total_lines = cv.lines.len().max(1);
                            let max_scroll = (total_lines.saturating_sub(visible_lines)).max(1);
                            let first_line =
                                (self.editor_scroll_offset * max_scroll as f32) as usize;
                            let mut lines_with_numbers = String::new();
                            let end = (first_line + visible_lines).min(cv.lines.len());
                            for i in first_line..end {
                                let num = i + 1;
                                if let Some(line) = cv.lines.get(i) {
                                    lines_with_numbers
                                        .push_str(&format!("{:>3} │ {}\n", num, line));
                                }
                            }
                            lines_with_numbers
                        })
                        .unwrap_or_else(|| "fn main() {\n    println!(\"hello\");\n}".to_string());

                    // Auto-scroll: ensure cursor is visible in the viewport.
                    {
                        let line_h = 16.0;
                        let pad = 8.0;
                        let header_h = 28.0;
                        let usable_h = (editor_rect.h - header_h - pad * 2.0).max(0.0);
                        let visible_lines = (usable_h / line_h).max(1.0) as usize;
                        let total_lines = self
                            .shell
                            .work_content
                            .as_ref()
                            .and_then(|w| w.editor_body.as_ref())
                            .map(|cv| cv.lines.len().max(1))
                            .unwrap_or(1);
                        let max_scroll = (total_lines.saturating_sub(visible_lines)).max(1);
                        let first_visible =
                            (self.editor_scroll_offset * max_scroll as f32) as usize;
                        let last_visible = first_visible + visible_lines;
                        let cl = self.editor_cursor_line;
                        if cl < first_visible {
                            self.editor_scroll_offset =
                                (cl as f32 / max_scroll as f32).clamp(0.0, 1.0);
                        } else if cl >= last_visible {
                            self.editor_scroll_offset =
                                ((cl.saturating_sub(visible_lines - 1)) as f32 / max_scroll as f32)
                                    .clamp(0.0, 1.0);
                        }
                    }

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
                    // Filter to visible viewport only by slicing lines.
                    let editor_spans: Option<Vec<(String, [f32; 4])>> =
                        self.shell.work_content.as_ref().and_then(|w| w.editor_body.as_ref()).map(
                            |cv| {
                                let line_h = 16.0;
                                let pad = 8.0;
                                let header_h = 28.0;
                                let usable_h = (editor_rect.h - header_h - pad * 2.0).max(0.0);
                                let visible_lines = (usable_h / line_h).max(1.0) as usize;
                                let total_lines = cv.lines.len().max(1);
                                let max_scroll = (total_lines.saturating_sub(visible_lines)).max(1);
                                let first_line =
                                    (self.editor_scroll_offset * max_scroll as f32) as usize;
                                let end = (first_line + visible_lines).min(cv.lines.len());
                                // Build a reverse mapping: line index ranges in source text
                                let mut byte_start = 0usize;
                                let mut line_bytes: Vec<(usize, usize)> = Vec::new();
                                for (_i, line) in cv.lines.iter().enumerate() {
                                    let byte_end = byte_start + line.len();
                                    line_bytes.push((byte_start, byte_end));
                                    byte_start = byte_end + 1; // +1 for newline
                                }
                                // Colorize only visible lines
                                let visible_slice: Vec<String> = cv.lines[first_line..end].to_vec();
                                let all_spans =
                                    super::syntax_color::colorize_source(&visible_slice, &sem);
                                all_spans
                            },
                        );

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

                    let status_lang = self
                        .shell
                        .work_content
                        .as_ref()
                        .and_then(|w| w.active_file.as_ref())
                        .and_then(|f| f.rsplit('.').next())
                        .map(|ext| match ext {
                            "rs" => "Rust",
                            "toml" => "TOML",
                            "md" => "Markdown",
                            "json" => "JSON",
                            "py" => "Python",
                            "js" => "JavaScript",
                            "ts" => "TypeScript",
                            _ => ext,
                        })
                        .unwrap_or("Rust");

                    let ai_text = self
                        .shell
                        .work_content
                        .as_ref()
                        .and_then(|w| w.ai_panel_content.as_ref())
                        .map(|cv| cv.lines.join("\n"));

                    let ctx = super::frame::ShellBlockContext {
                        editor_data: super::editor::EditorContentData {
                            tab_title,
                            tab_content,
                            breadcrumb_label,
                            editor_body_text,
                            editor_spans,
                            cursor_line: editor_cursor_line,
                            cursor_col: editor_cursor_col,
                        },
                        explorer_data: super::rail::ExplorerData { sidebar_items },
                        status_bar_data: super::status_bar::StatusBarData {
                            status_line: self.editor_cursor_line,
                            status_col: self.editor_cursor_col,
                            status_language: status_lang.to_string(),
                        },
                        ai_data: super::ai_pane::AiPanelData { ai_content: ai_text },
                    };

                    let render_blocks: Vec<zaroxi_core_engine_render::UiBlock> =
                        super::frame::compose_blocks(&self.shell.regions, &tokens, &ctx);

                    match pollster::block_on(zaroxi_core_engine_render::Renderer::new(
                        z.window(),
                        [
                            tokens.app_background.r as f64,
                            tokens.app_background.g as f64,
                            tokens.app_background.b as f64,
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
                        "GUI_TEXT_FRAME_SUMMARY: surface={}x{} render_blocks={}",
                        sw,
                        sh,
                        render_blocks.len()
                    );
                }
            }
            _ => {}
        }
    }
}
