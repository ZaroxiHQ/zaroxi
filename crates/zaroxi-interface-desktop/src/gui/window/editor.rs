/*!
Editor panel: tab strip, breadcrumb, content area, minimap, bottom panel.

Phase 50: panel-owned UiBlock construction.
Content flows from EngineContentData (live editor state) or defaults.
*/

use crate::gui::ShellRegion;
use zaroxi_core_engine_render::UiBlock;
use zaroxi_core_engine_style::StyleTokens;

pub struct EditorContentData {
    pub tab_title: String,
    pub tab_content: String,
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
            tab_title: String::new(),
            tab_content: String::new(),
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

        UiBlock {
            id: r.id.to_string(),
            title: data.tab_title.clone(),
            content: data.tab_content.clone(),
            visible: true,
            rect,
            header_color: Some(tokens.tab_strip_background.to_array()),
            content_color: None,
            corner_radius: 0.0,
            border_color: Some(tokens.divider_default.to_array()),
            border_width: 1.0,
            header_only: true,
            content_spans: None,
            cursor_line: None,
            cursor_col: None,
            highlight_active_line: false,
            selection_range: None,
            text_color: Some(tokens.text_primary.to_array()),
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
            text_color: None,
        }
    }

    pub fn build_gutter_block(r: &ShellRegion, tokens: &StyleTokens, line_count: usize) -> UiBlock {
        let rect = zaroxi_core_engine_render::Rect {
            x: r.rect.x as f32,
            y: r.rect.y as f32,
            w: r.rect.width as f32,
            h: r.rect.height as f32,
        };

        let gutter_text =
            (1..=line_count.max(1)).map(|n| format!("{:>4}", n)).collect::<Vec<_>>().join("\n");

        UiBlock {
            id: r.id.to_string(),
            title: String::new(),
            content: gutter_text,
            visible: true,
            rect,
            header_color: Some(tokens.editor_gutter_bg.to_array()),
            content_color: Some(tokens.editor_gutter_bg.to_array()),
            corner_radius: 0.0,
            border_color: Some(tokens.divider_subtle.to_array()),
            border_width: 1.0,
            header_only: false,
            content_spans: None,
            cursor_line: None,
            cursor_col: None,
            highlight_active_line: false,
            selection_range: None,
            text_color: Some(tokens.text_faint.to_array()),
        }
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
        }
    }
}
