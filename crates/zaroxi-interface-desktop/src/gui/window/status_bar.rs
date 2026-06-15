/*!
Status bar panel.

Phase 50: panel-owned UiBlock construction.
Phase 73: uses chrome StatusBarZones for structured left/right rendering.
*/

use crate::gui::ShellRegion;
use zaroxi_core_engine_render::UiBlock;
use zaroxi_core_engine_style::StyleTokens;
use zaroxi_core_engine_ui::chrome::StatusBarZones;

pub struct StatusBarData {
    pub status_line: usize,
    pub status_col: usize,
    pub status_language: String,
    pub status_zones: Option<StatusBarZones>,
}

impl Default for StatusBarData {
    fn default() -> Self {
        Self { status_line: 0, status_col: 0, status_language: String::new(), status_zones: None }
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

        let content: String;
        let content_spans: Option<Vec<(String, [f32; 4])>>;

        if let Some(ref zones) = data.status_zones {
            let spans = zaroxi_core_engine_ui::chrome::format_status_bar_spans(zones, tokens);
            content = spans.iter().map(|(t, _)| t.clone()).collect::<Vec<_>>().join("");
            content_spans = Some(spans);
        } else {
            let fallback = format!(
                "Ready  Ln {}, Col {}  UTF-8  LF  {}",
                data.status_line + 1,
                data.status_col + 1,
                data.status_language,
            );
            content = fallback;
            content_spans = None;
        }

        UiBlock {
            id: r.id.to_string(),
            title: String::new(),
            content,
            visible: true,
            rect,
            header_color: Some(tokens.status_bar_background.to_array()),
            content_color: Some(tokens.status_bar_background.to_array()),
            corner_radius: 0.0,
            border_color: None,
            border_width: 0.0,
            header_only: false,
            content_spans,
            cursor_line: None,
            cursor_col: None,
            highlight_active_line: false,
            selection_range: None,
            text_color: Some(tokens.text_secondary.to_array()),
            clip_rect: None,
            content_offset_x: 0.0,
            content_offset_y: 0.0,
            content_line_offset: None,
        }
    }
}
