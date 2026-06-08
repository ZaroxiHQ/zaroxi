/*!
Left rail and sidebar panels.

Phase 50: panel-owned UiBlock construction.
Phase 73: ExplorerData holds PanelSections for chrome-aware formatting.
*/

use crate::gui::ShellRegion;
use zaroxi_core_engine_render::UiBlock;
use zaroxi_core_engine_style::StyleTokens;
use zaroxi_core_engine_ui::chrome::PanelSection;

pub struct ExplorerData {
    pub sidebar_sections: Vec<PanelSection>,
    pub sidebar_empty: bool,
    /// Label for a visible primary-action button in the sidebar (e.g. "Open Workspace").
    pub empty_button_label: Option<String>,
}

impl Default for ExplorerData {
    fn default() -> Self {
        Self { sidebar_sections: Vec::new(), sidebar_empty: true, empty_button_label: None }
    }
}

/// Result of building the sidebar explorer section.
pub struct SidebarBlocks {
    pub blocks: Vec<UiBlock>,
    /// Hit rect for the CTA button, if present (x, y, w, h).
    pub cta_hit_rect: Option<(f32, f32, f32, f32)>,
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

    pub fn build_sidebar_block(
        r: &ShellRegion,
        tokens: &StyleTokens,
        data: &ExplorerData,
    ) -> SidebarBlocks {
        let mut blocks = Vec::new();
        let mut cta_hit_rect: Option<(f32, f32, f32, f32)> = None;

        let rect = zaroxi_core_engine_render::Rect {
            x: r.rect.x as f32,
            y: r.rect.y as f32,
            w: r.rect.width as f32,
            h: r.rect.height as f32,
        };

        // Background / text content block
        let content: String;
        let content_spans: Option<Vec<(String, [f32; 4])>>;

        let empty_message = if data.sidebar_empty {
            let spans = zaroxi_core_engine_ui::chrome::format_explorer_spans(&[], tokens);
            content = spans.iter().map(|(t, _)| t.clone()).collect::<Vec<_>>().join("");
            content_spans = Some(spans);
            true
        } else {
            let spans = zaroxi_core_engine_ui::chrome::format_explorer_spans(
                &data.sidebar_sections,
                tokens,
            );
            content = spans.iter().map(|(t, _)| t.clone()).collect::<Vec<_>>().join("");
            content_spans = Some(spans);
            false
        };

        blocks.push(UiBlock {
            id: r.id.to_string(),
            title: "Explorer".to_string(),
            content,
            visible: true,
            rect,
            header_color: Some(tokens.sidebar_background.to_array()),
            content_color: Some(tokens.sidebar_background.to_array()),
            corner_radius: 0.0,
            border_color: None,
            border_width: 0.0,
            header_only: false,
            content_spans,
            cursor_line: None,
            cursor_col: None,
            highlight_active_line: false,
            selection_range: None,
            clip_rect: None,
            text_color: None,
        });

        // Button block for empty state CTA.
        // Position matches the widget tree button (shell_builder.rs):
        //   sidebar_rect.x + pad + 10.0, y_off + 8.0
        // where y_off = rect.y + pad + search_h(26) + search_gap(8) + divider_space(12).
        if empty_message {
            if let Some(ref btn_label) = data.empty_button_label {
                let pad = 10.0;
                let search_h = 26.0;
                let search_gap = 8.0;
                let divider_space = 12.0;
                let btn_button_y = 8.0;
                let btn_w = 140.0;
                let btn_h = 30.0;
                let btn_x = rect.x + pad + 10.0;
                let btn_y = rect.y + pad + search_h + search_gap + divider_space + btn_button_y;
                let btn_rect =
                    zaroxi_core_engine_render::Rect { x: btn_x, y: btn_y, w: btn_w, h: btn_h };

                blocks.push(UiBlock {
                    id: "explorer_open_workspace_btn".to_string(),
                    title: btn_label.clone(),
                    content: btn_label.clone(),
                    visible: true,
                    rect: btn_rect,
                    header_color: Some(tokens.accent.to_array()),
                    content_color: Some(tokens.accent.to_array()),
                    corner_radius: 4.0,
                    border_color: None,
                    border_width: 0.0,
                    header_only: false,
                    content_spans: Some(vec![(btn_label.clone(), tokens.text_primary.to_array())]),
                    cursor_line: None,
                    cursor_col: None,
                    highlight_active_line: false,
                    selection_range: None,
                    text_color: Some(tokens.text_primary.to_array()),
                    clip_rect: None,
                });

                cta_hit_rect = Some((btn_x, btn_y, btn_w, btn_h));
            }
        }

        SidebarBlocks { blocks, cta_hit_rect }
    }
}
