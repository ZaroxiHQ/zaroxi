/*!
Top toolbar / titlebar panel.

Phase 50: panel-owned UiBlock construction.
*/

use crate::gui::ShellRegion;
use zaroxi_core_engine_render::UiBlock;
use zaroxi_core_engine_style::StyleTokens;

pub struct TopBarPanel;

impl TopBarPanel {
    pub fn build_block(r: &ShellRegion, tokens: &StyleTokens) -> UiBlock {
        let rect = zaroxi_core_engine_render::Rect {
            x: r.rect.x as f32,
            y: r.rect.y as f32,
            w: r.rect.width as f32,
            h: r.rect.height as f32,
        };

        UiBlock {
            id: r.id.to_string(),
            title: "Zaroxi Studio".to_string(),
            content: String::new(),
            visible: true,
            rect,
            header_color: Some(tokens.titlebar_background.to_array()),
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
            text_color: Some(tokens.text_primary.to_array()),
            clip_rect: None,
            content_offset_x: 0.0,
            content_offset_y: 0.0,
            content_line_offset: None,
        }
    }
}
