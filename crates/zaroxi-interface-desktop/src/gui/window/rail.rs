/*!
Left rail and sidebar panels.

Phase 50: panel-owned UiBlock construction.
Phase 73: ExplorerData holds PanelSections for chrome-aware formatting.
*/

use crate::gui::ShellRegion;
use zaroxi_core_engine_render::UiBlock;
use zaroxi_core_engine_style::StyleTokens;
use zaroxi_core_engine_ui::ExplorerPanelItem;
use zaroxi_core_engine_ui::chrome::PanelSection;

use crate::gui::window::editor_shell::constants::{
    DIVIDER_SPACE, EXPLORER_HEADER_H, EXPLORER_INDENT_PX, EXPLORER_ROW_H, SEARCH_BAR_H,
    SEARCH_TO_DIVIDER_GAP, SIDEBAR_PAD, explorer_cta_button_rect,
};

pub struct ExplorerData {
    pub sidebar_sections: Vec<PanelSection>,
    pub sidebar_empty: bool,
    pub empty_button_label: Option<String>,
    /// Structured panel items for per-row hit-target alignment (Editor Phase 4).
    pub panel_items: Option<Vec<ExplorerPanelItem>>,
    /// Panel title used to compute initial y-offset for row positioning.
    pub panel_title: Option<String>,
}

impl Default for ExplorerData {
    fn default() -> Self {
        Self {
            sidebar_sections: Vec::new(),
            sidebar_empty: true,
            empty_button_label: None,
            panel_items: None,
            panel_title: None,
        }
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
            content_offset_x: 0.0,
            content_offset_y: 0.0,
            content_line_offset: None,
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

        // Background surface block
        blocks.push(UiBlock {
            id: r.id.to_string(),
            title: String::new(),
            content: String::new(),
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
            clip_rect: None,
            content_offset_x: 0.0,
            content_offset_y: 0.0,
            content_line_offset: None,
            text_color: None,
        });

        // ── Per-row blocks (aligned with widget tree hit regions) ──
        if let Some(ref items) = data.panel_items {
            if !items.is_empty() {
                let pad = SIDEBAR_PAD;
                let inner_w = rect.w - pad * 2.0;
                let mut y_off = rect.y + pad + SEARCH_BAR_H + SEARCH_TO_DIVIDER_GAP + DIVIDER_SPACE;
                if data.panel_title.is_some() {
                    y_off += EXPLORER_HEADER_H + 4.0;
                }
                let max_y = rect.y + rect.h - 12.0;

                for (item_idx, item) in items.iter().enumerate() {
                    let row_h = EXPLORER_ROW_H;
                    if y_off + row_h > max_y {
                        break;
                    }
                    let indent_px = item.depth as f32 * EXPLORER_INDENT_PX;
                    let row_x = rect.x + pad + 14.0 + indent_px;
                    let row_y = y_off + 2.0;
                    let row_w = (inner_w - 20.0 - indent_px).max(4.0);
                    let row_h_vis = 14.0_f32;

                    let fill = if item.is_active {
                        tokens.rail_item_active.to_array()
                    } else {
                        tokens.sidebar_file_item.to_array()
                    };
                    let text_c = if item.is_active {
                        tokens.text_primary.to_array()
                    } else {
                        tokens.text_secondary.to_array()
                    };

                    blocks.push(UiBlock {
                        id: format!("explorer_row_{}", item_idx),
                        title: item.label.clone(),
                        content: String::new(),
                        visible: true,
                        rect: zaroxi_core_engine_render::Rect {
                            x: row_x,
                            y: row_y,
                            w: row_w,
                            h: row_h_vis,
                        },
                        header_color: Some(fill),
                        content_color: None,
                        corner_radius: 2.0,
                        border_color: None,
                        border_width: 0.0,
                        header_only: true,
                        content_spans: None,
                        cursor_line: None,
                        cursor_col: None,
                        highlight_active_line: false,
                        selection_range: None,
                        text_color: Some(text_c),
                        clip_rect: None,
                        content_offset_x: 0.0,
                        content_offset_y: 0.0,
                        content_line_offset: None,
                    });
                    y_off += row_h;
                }
                return SidebarBlocks { blocks, cta_hit_rect };
            }
        }

        // ── Fallback: single text-block rendering (legacy) ──
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
            id: format!("{}-content", r.id),
            title: "Explorer".to_string(),
            content,
            visible: true,
            rect,
            header_color: None,
            content_color: None,
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
            content_offset_x: 0.0,
            content_offset_y: 0.0,
            content_line_offset: None,
            text_color: None,
        });

        // CTA button for empty state
        if empty_message {
            if let Some(ref btn_label) = data.empty_button_label {
                let (btn_x, btn_y, btn_w, btn_h) =
                    explorer_cta_button_rect((rect.x, rect.y, rect.w, rect.h));
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
                    content_offset_x: 0.0,
                    content_offset_y: 0.0,
                    content_line_offset: None,
                });

                cta_hit_rect = Some((btn_x, btn_y, btn_w, btn_h));
            }
        }

        SidebarBlocks { blocks, cta_hit_rect }
    }
}
