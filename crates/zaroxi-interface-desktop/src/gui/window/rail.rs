/*!
Left rail and sidebar panels.

Phase 50: panel-owned UiBlock construction.
*/

use crate::gui::ShellRegion;
use zaroxi_core_engine_render::UiBlock;
use zaroxi_core_engine_style::StyleTokens;

pub struct ExplorerData {
    pub sidebar_items: String,
}

impl Default for ExplorerData {
    fn default() -> Self {
        Self {
            sidebar_items: "PROJECT\n  src/main.rs\n  src/lib.rs\n  Cargo.toml\nGIT\n  clean\nOUTLINE\n  fn main\n  struct App".into(),
        }
    }
}

pub struct RailPanel;

impl RailPanel {
    pub fn build_rail_block(r: &ShellRegion, tokens: &StyleTokens) -> UiBlock {
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
            header_color: Some(tokens.rail_background.to_array()),
            content_color: Some(tokens.rail_background.to_array()),
            corner_radius: 0.0,
            border_color: Some(tokens.sidebar_border.to_array()),
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

    pub fn build_sidebar_block(
        r: &ShellRegion,
        tokens: &StyleTokens,
        data: &ExplorerData,
    ) -> UiBlock {
        let rect = zaroxi_core_engine_render::Rect {
            x: r.rect.x as f32,
            y: r.rect.y as f32,
            w: r.rect.width as f32,
            h: r.rect.height as f32,
        };

        UiBlock {
            id: r.id.to_string(),
            title: "Explorer".to_string(),
            content: data.sidebar_items.clone(),
            visible: true,
            rect,
            header_color: Some(tokens.sidebar_background.to_array()),
            content_color: Some(tokens.sidebar_background.to_array()),
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
