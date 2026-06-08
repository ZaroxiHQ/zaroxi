/*!
Editor panel: tab strip, breadcrumb, content area, minimap, bottom panel.

Phase 50: panel-owned UiBlock construction.
Phase 73: tab strip uses chrome formatters with content_spans.
*/

use crate::gui::ShellRegion;
use zaroxi_core_engine_render::UiBlock;
use zaroxi_core_engine_style::StyleTokens;
use zaroxi_core_engine_ui::chrome::TabEntry;

pub struct EditorContentData {
    pub tab_entries: Vec<TabEntry>,
    pub breadcrumb_label: String,
    pub editor_body_text: String,
    pub editor_spans: Option<Vec<(String, [f32; 4])>>,
    pub cursor_line: usize,
    pub cursor_col: usize,
    pub body_title: String,
}

impl Default for EditorContentData {
    fn default() -> Self {
        Self {
            tab_entries: Vec::new(),
            breadcrumb_label: String::new(),
            editor_body_text: String::new(),
            editor_spans: None,
            cursor_line: 0,
            cursor_col: 0,
            body_title: String::new(),
        }
    }
}

pub struct EditorPanel;

impl EditorPanel {
    pub fn build_tab_strip_block(
        r: &ShellRegion,
        tokens: &StyleTokens,
        data: &EditorContentData,
    ) -> UiBlock {
        let rect = zaroxi_core_engine_render::Rect {
            x: r.rect.x as f32,
            y: r.rect.y as f32,
            w: r.rect.width as f32,
            h: r.rect.height as f32,
        };

        let tab_spans =
            zaroxi_core_engine_ui::chrome::format_tab_strip_spans(&data.tab_entries, tokens);

        UiBlock {
            id: r.id.to_string(),
            title: String::new(),
            content: String::new(),
            visible: true,
            rect,
            header_color: Some(tokens.tab_strip_background.to_array()),
            content_color: Some(tokens.tab_strip_background.to_array()),
            corner_radius: 0.0,
            border_color: None,
            border_width: 0.0,
            header_only: false,
            content_spans: Some(tab_spans),
            cursor_line: None,
            cursor_col: None,
            highlight_active_line: false,
            selection_range: None,
            text_color: None,
            clip_rect: None,
        }
    }

    pub fn build_breadcrumb_block(
        r: &ShellRegion,
        tokens: &StyleTokens,
        data: &EditorContentData,
    ) -> UiBlock {
        let rect = zaroxi_core_engine_render::Rect {
            x: r.rect.x as f32,
            y: r.rect.y as f32,
            w: r.rect.width as f32,
            h: r.rect.height as f32,
        };

        UiBlock {
            id: r.id.to_string(),
            title: data.breadcrumb_label.clone(),
            content: String::new(),
            visible: true,
            rect,
            header_color: Some(tokens.editor_breadcrumb_background.to_array()),
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
            text_color: Some(tokens.text_muted.to_array()),
            clip_rect: None,
        }
    }

    pub fn build_content_area_block(
        r: &ShellRegion,
        tokens: &StyleTokens,
        data: &EditorContentData,
    ) -> UiBlock {
        let rect = zaroxi_core_engine_render::Rect {
            x: r.rect.x as f32,
            y: r.rect.y as f32,
            w: r.rect.width as f32,
            h: r.rect.height as f32,
        };

        UiBlock {
            id: r.id.to_string(),
            title: data.body_title.clone(),
            content: data.editor_body_text.clone(),
            visible: true,
            rect,
            header_color: Some(tokens.editor_content_background.to_array()),
            content_color: Some(tokens.editor_content_background.to_array()),
            corner_radius: 0.0,
            border_color: None,
            border_width: 0.0,
            header_only: false,
            content_spans: data.editor_spans.clone(),
            cursor_line: Some(data.cursor_line),
            cursor_col: Some(data.cursor_col),
            highlight_active_line: true,
            selection_range: None,
            clip_rect: None,
            text_color: None,
        }
    }

    pub fn build_gutter_block(r: &ShellRegion, tokens: &StyleTokens, line_count: usize) -> UiBlock {
        zaroxi_core_engine_ui::blocks::make_gutter_block(
            r.rect.x as f32,
            r.rect.y as f32,
            r.rect.width as f32,
            r.rect.height as f32,
            line_count,
            tokens,
        )
    }

    pub fn build_minimap_block(r: &ShellRegion, tokens: &StyleTokens) -> UiBlock {
        let rect = zaroxi_core_engine_render::Rect {
            x: r.rect.x as f32,
            y: r.rect.y as f32,
            w: r.rect.width as f32,
            h: r.rect.height as f32,
        };

        UiBlock {
            id: r.id.to_string(),
            title: String::new(),
            content: String::new(),
            visible: true,
            rect,
            header_color: Some(tokens.editor_content_background.adjust_brightness(0.95).to_array()),
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
            clip_rect: None,
            text_color: None,
        }
    }

    pub fn build_bottom_panel_block(
        r: &ShellRegion,
        tokens: &StyleTokens,
        terminal_tabs: Option<&[String]>,
    ) -> UiBlock {
        let rect = zaroxi_core_engine_render::Rect {
            x: r.rect.x as f32,
            y: r.rect.y as f32,
            w: r.rect.width as f32,
            h: r.rect.height as f32,
        };

        let title = terminal_tabs
            .filter(|t| !t.is_empty())
            .map(|tabs| tabs.join(" \u{2022} "))
            .unwrap_or_else(|| "Terminal \u{2022} Problems \u{2022} Output".to_string());

        let content = terminal_tabs
            .filter(|t| !t.is_empty())
            .map(|_tabs| "Ready".to_string())
            .unwrap_or_else(|| "No terminal session".to_string());

        UiBlock {
            id: r.id.to_string(),
            title,
            content,
            visible: true,
            rect,
            header_color: Some(tokens.bottom_panel_header_background.to_array()),
            content_color: Some(tokens.bottom_panel_background.to_array()),
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
            clip_rect: None,
        }
    }
}
