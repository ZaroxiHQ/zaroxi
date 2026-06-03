/*!
Panel block builders: construct UiBlock structures for each shell region
based on resolved StyleTokens and live workspace content.

This module extracts the region-to-UiBlock mapping from app.rs so each
panel owns its own composition logic. All builders are pure functions
that accept tokens + content data and return UiBlock.
*/

use crate::gui::ShellRegion;
use crate::gui::region_dispatch::region_role;
use zaroxi_core_engine_render::UiBlock;
use zaroxi_core_engine_style::{PanelRole, StyleTokens, ThemeColor};

fn shell_rect(r: &ShellRegion) -> zaroxi_core_engine_render::Rect {
    zaroxi_core_engine_render::Rect {
        x: r.rect.x as f32,
        y: r.rect.y as f32,
        w: r.rect.width as f32,
        h: r.rect.height as f32,
    }
}

fn color_array(c: ThemeColor) -> [f32; 4] {
    c.to_array()
}

pub struct PanelContentData {
    pub tab_title: String,
    pub tab_content: String,
    pub breadcrumb_label: String,
    pub sidebar_items: String,
    pub editor_body_text: String,
    pub editor_spans: Option<Vec<(String, [f32; 4])>>,
    pub editor_cursor_line: usize,
    pub editor_cursor_col: usize,
    pub status_line: usize,
    pub status_col: usize,
    pub status_language: String,
    pub ai_content: Option<String>,
}

impl Default for PanelContentData {
    fn default() -> Self {
        Self {
            tab_title: "main.rs".into(),
            tab_content: "lib.rs  mod.rs".into(),
            breadcrumb_label: "src > app > main.rs".into(),
            sidebar_items: "PROJECT\n  src/main.rs\n  src/lib.rs\n  Cargo.toml\nGIT\n  clean\nOUTLINE\n  fn main\n  struct App".into(),
            editor_body_text: "fn main() {\n    println!(\"hello\");\n}".into(),
            editor_spans: None,
            editor_cursor_line: 0,
            editor_cursor_col: 0,
            status_line: 22,
            status_col: 14,
            status_language: "Rust".into(),
            ai_content: None,
        }
    }
}

pub fn region_to_block(r: &ShellRegion, tokens: &StyleTokens, data: &PanelContentData) -> UiBlock {
    let rect = shell_rect(r);

    match region_role(r.id) {
        PanelRole::TopBar => build_topbar_block(r, rect, tokens),
        PanelRole::NavigationRail => build_rail_block(r, rect, tokens),
        PanelRole::SidePanel => build_sidebar_block(r, rect, tokens, data),
        PanelRole::ContentTabStrip => build_tab_strip_block(r, rect, tokens, data),
        PanelRole::ContentBreadcrumb => build_breadcrumb_block(r, rect, tokens, data),
        PanelRole::ContentArea => build_content_area_block(r, rect, tokens, data),
        PanelRole::MinimapLane => build_minimap_block(r, rect, tokens),
        PanelRole::BottomPanel => build_bottom_panel_block(r, rect, tokens),
        PanelRole::BottomDock => build_bottom_dock_block(r, rect, tokens),
        PanelRole::AuxiliaryPanelHeader => build_aux_header_block(r, rect, tokens),
        PanelRole::AuxiliaryPanelContent => build_aux_content_block(r, rect, tokens, data),
        PanelRole::StatusBar => build_status_bar_block(r, rect, tokens, data),
    }
}

fn build_topbar_block(
    r: &ShellRegion,
    rect: zaroxi_core_engine_render::Rect,
    tokens: &StyleTokens,
) -> UiBlock {
    UiBlock {
        id: r.id.to_string(),
        title: "Zaroxi Studio".to_string(),
        content: String::new(),
        visible: true,
        rect,
        header_color: Some(color_array(tokens.status_bar_background)),
        content_color: None,
        corner_radius: 0.0,
        border_color: Some(color_array(tokens.divider_default)),
        border_width: 1.0,
        header_only: true,
        content_spans: None,
        cursor_line: None,
        cursor_col: None,
        highlight_active_line: false,
        selection_range: None,
        text_color: Some(color_array(tokens.text_primary)),
    }
}

fn build_rail_block(
    r: &ShellRegion,
    rect: zaroxi_core_engine_render::Rect,
    tokens: &StyleTokens,
) -> UiBlock {
    UiBlock {
        id: r.id.to_string(),
        title: String::new(),
        content: String::new(),
        visible: true,
        rect,
        header_color: Some(color_array(tokens.rail_background)),
        content_color: Some(color_array(tokens.rail_background)),
        corner_radius: 0.0,
        border_color: Some(color_array(tokens.sidebar_border)),
        border_width: 1.0,
        header_only: false,
        content_spans: None,
        cursor_line: None,
        cursor_col: None,
        highlight_active_line: false,
        selection_range: None,
        text_color: None,
    }
}

fn build_sidebar_block(
    r: &ShellRegion,
    rect: zaroxi_core_engine_render::Rect,
    tokens: &StyleTokens,
    data: &PanelContentData,
) -> UiBlock {
    UiBlock {
        id: r.id.to_string(),
        title: "Explorer".to_string(),
        content: data.sidebar_items.clone(),
        visible: true,
        rect,
        header_color: Some(color_array(tokens.sidebar_background)),
        content_color: Some(color_array(tokens.sidebar_background)),
        corner_radius: 0.0,
        border_color: None,
        border_width: 0.0,
        header_only: false,
        content_spans: None,
        cursor_line: None,
        cursor_col: None,
        highlight_active_line: false,
        selection_range: None,
        text_color: None,
    }
}

fn build_tab_strip_block(
    r: &ShellRegion,
    rect: zaroxi_core_engine_render::Rect,
    tokens: &StyleTokens,
    data: &PanelContentData,
) -> UiBlock {
    UiBlock {
        id: r.id.to_string(),
        title: data.tab_title.clone(),
        content: data.tab_content.clone(),
        visible: true,
        rect,
        header_color: Some(color_array(tokens.tab_strip_background)),
        content_color: None,
        corner_radius: 4.0,
        border_color: Some(color_array(tokens.divider_default)),
        border_width: 1.0,
        header_only: true,
        content_spans: None,
        cursor_line: None,
        cursor_col: None,
        highlight_active_line: false,
        selection_range: None,
        text_color: Some(color_array(tokens.text_primary)),
    }
}

fn build_breadcrumb_block(
    r: &ShellRegion,
    rect: zaroxi_core_engine_render::Rect,
    tokens: &StyleTokens,
    data: &PanelContentData,
) -> UiBlock {
    UiBlock {
        id: r.id.to_string(),
        title: data.breadcrumb_label.clone(),
        content: String::new(),
        visible: true,
        rect,
        header_color: Some(color_array(tokens.editor_breadcrumb_background)),
        content_color: None,
        corner_radius: 0.0,
        border_color: Some(color_array(tokens.divider_subtle)),
        border_width: 1.0,
        header_only: true,
        content_spans: None,
        cursor_line: None,
        cursor_col: None,
        highlight_active_line: false,
        selection_range: None,
        text_color: Some(color_array(tokens.text_muted)),
    }
}

fn build_content_area_block(
    r: &ShellRegion,
    rect: zaroxi_core_engine_render::Rect,
    tokens: &StyleTokens,
    data: &PanelContentData,
) -> UiBlock {
    UiBlock {
        id: r.id.to_string(),
        title: String::new(),
        content: data.editor_body_text.clone(),
        visible: true,
        rect,
        header_color: Some(color_array(tokens.editor_content_background)),
        content_color: Some(color_array(tokens.editor_content_background)),
        corner_radius: 0.0,
        border_color: None,
        border_width: 0.0,
        header_only: false,
        content_spans: data.editor_spans.clone(),
        cursor_line: Some(data.editor_cursor_line),
        cursor_col: Some(data.editor_cursor_col),
        highlight_active_line: true,
        selection_range: None,
        text_color: None,
    }
}

fn build_minimap_block(
    r: &ShellRegion,
    rect: zaroxi_core_engine_render::Rect,
    tokens: &StyleTokens,
) -> UiBlock {
    UiBlock {
        id: r.id.to_string(),
        title: String::new(),
        content: String::new(),
        visible: true,
        rect,
        header_color: Some(color_array(tokens.editor_content_background.adjust_brightness(0.95))),
        content_color: None,
        corner_radius: 0.0,
        border_color: None,
        border_width: 0.0,
        header_only: true,
        content_spans: None,
        cursor_line: None,
        cursor_col: None,
        highlight_active_line: false,
        selection_range: None,
        text_color: None,
    }
}

fn build_bottom_panel_block(
    r: &ShellRegion,
    rect: zaroxi_core_engine_render::Rect,
    tokens: &StyleTokens,
) -> UiBlock {
    UiBlock {
        id: r.id.to_string(),
        title: "Terminal • Problems • Output".to_string(),
        content: "$ cargo build\n   Compiling zaroxi v0.1.0\n    Finished dev [unoptimized]"
            .to_string(),
        visible: true,
        rect,
        header_color: Some(color_array(tokens.panel_header_background)),
        content_color: Some(color_array(tokens.bottom_panel_background)),
        corner_radius: 4.0,
        border_color: Some(color_array(tokens.divider_default)),
        border_width: 1.0,
        header_only: false,
        content_spans: None,
        cursor_line: None,
        cursor_col: None,
        highlight_active_line: false,
        selection_range: None,
        text_color: None,
    }
}

fn build_bottom_dock_block(
    r: &ShellRegion,
    rect: zaroxi_core_engine_render::Rect,
    tokens: &StyleTokens,
) -> UiBlock {
    UiBlock {
        id: r.id.to_string(),
        title: String::new(),
        content: String::new(),
        visible: r.rect.height > 0,
        rect,
        header_color: Some(color_array(tokens.app_background)),
        content_color: None,
        corner_radius: 0.0,
        border_color: None,
        border_width: 0.0,
        header_only: true,
        content_spans: None,
        cursor_line: None,
        cursor_col: None,
        highlight_active_line: false,
        selection_range: None,
        text_color: None,
    }
}

fn build_aux_header_block(
    r: &ShellRegion,
    rect: zaroxi_core_engine_render::Rect,
    tokens: &StyleTokens,
) -> UiBlock {
    UiBlock {
        id: r.id.to_string(),
        title: "AI Assistant".to_string(),
        content: String::new(),
        visible: true,
        rect,
        header_color: Some(color_array(tokens.panel_header_background)),
        content_color: None,
        corner_radius: 0.0,
        border_color: Some(color_array(tokens.divider_default)),
        border_width: 1.0,
        header_only: true,
        content_spans: None,
        cursor_line: None,
        cursor_col: None,
        highlight_active_line: false,
        selection_range: None,
        text_color: Some(color_array(tokens.text_primary)),
    }
}

fn build_aux_content_block(
    r: &ShellRegion,
    rect: zaroxi_core_engine_render::Rect,
    tokens: &StyleTokens,
    data: &PanelContentData,
) -> UiBlock {
    let content = data.ai_content.clone().unwrap_or_else(|| {
        "No active AI session\nOpen a file and request an AI edit to get started.".to_string()
    });
    UiBlock {
        id: r.id.to_string(),
        title: "Assistant".to_string(),
        content,
        visible: true,
        rect,
        header_color: Some(color_array(tokens.assistant_panel_background)),
        content_color: None,
        corner_radius: 0.0,
        border_color: None,
        border_width: 0.0,
        header_only: true,
        content_spans: None,
        cursor_line: None,
        cursor_col: None,
        highlight_active_line: false,
        selection_range: None,
        text_color: None,
    }
}

fn build_status_bar_block(
    r: &ShellRegion,
    rect: zaroxi_core_engine_render::Rect,
    tokens: &StyleTokens,
    data: &PanelContentData,
) -> UiBlock {
    let status_text = format!(
        "Ready  Ln {}, Col {}  UTF-8  LF  {}",
        data.status_line + 1,
        data.status_col + 1,
        data.status_language,
    );
    UiBlock {
        id: r.id.to_string(),
        title: String::new(),
        content: status_text,
        visible: true,
        rect,
        header_color: Some(color_array(tokens.status_bar_background)),
        content_color: Some(color_array(tokens.status_bar_background)),
        corner_radius: 4.0,
        border_color: Some(color_array(tokens.status_divider)),
        border_width: 1.0,
        header_only: false,
        content_spans: None,
        cursor_line: None,
        cursor_col: None,
        highlight_active_line: false,
        selection_range: None,
        text_color: Some(color_array(tokens.text_secondary)),
    }
}
