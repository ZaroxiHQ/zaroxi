/*!
Status bar panel.

Phase 50: panel-owned UiBlock construction.
Uses live cursor position and language from state.
*/

use crate::gui::ShellRegion;
use zaroxi_core_engine_render::UiBlock;
use zaroxi_core_engine_style::StyleTokens;

pub struct StatusBarData {
    pub status_line: usize,
    pub status_col: usize,
    pub status_language: String,
}

impl Default for StatusBarData {
    fn default() -> Self {
        Self { status_line: 0, status_col: 0, status_language: "Rust".into() }
    }
}

pub struct StatusBarPanel;

impl StatusBarPanel {
    pub fn build_block(r: &ShellRegion, tokens: &StyleTokens, data: &StatusBarData) -> UiBlock {
        let rect = zaroxi_core_engine_render::Rect {
            x: r.rect.x as f32,
            y: r.rect.y as f32,
            w: r.rect.width as f32,
            h: r.rect.height as f32,
        };

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
            header_color: Some(tokens.status_bar_background.to_array()),
            content_color: Some(tokens.status_bar_background.to_array()),
            corner_radius: 4.0,
            border_color: Some(tokens.status_divider.to_array()),
            border_width: 1.0,
            header_only: false,
            content_spans: None,
            cursor_line: None,
            cursor_col: None,
            highlight_active_line: false,
            selection_range: None,
            text_color: Some(tokens.text_secondary.to_array()),
        }
    }
}
