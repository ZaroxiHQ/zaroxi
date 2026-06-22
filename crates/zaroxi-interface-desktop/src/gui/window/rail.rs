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
    EXPLORER_GLYPH_COL_W, EXPLORER_INDENT_PX, EXPLORER_MAX_Y_INSET, EXPLORER_ROW_H,
    EXPLORER_ROW_TEXT_INSET, EXPLORER_ROW_VIS_H, EXPLORER_ROW_W_REDUCTION, EXPLORER_TITLE_PAD,
    SIDEBAR_PAD, explorer_cta_button_rect,
};
use crate::gui::window::explorer_panel::icons;

/// Build a transparent (fill-less) text-only row block placed at an exact column.
/// Used for the explorer glyph and filename columns so each draws independently
/// of the other's width — keeping the filename column aligned regardless of a
/// double-width Nerd Font icon.
fn explorer_text_block(
    id: String,
    text: String,
    x: f32,
    y: f32,
    w: f32,
    h: f32,
    color: [f32; 4],
) -> UiBlock {
    UiBlock {
        id,
        title: text,
        content: String::new(),
        visible: true,
        rect: zaroxi_core_engine_render::Rect { x, y, w, h },
        header_color: Some([0.0, 0.0, 0.0, 0.0]),
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
        text_color: Some(color),
        clip_rect: None,
        content_offset_x: 0.0,
        content_offset_y: 0.0,
        content_line_offset: None,
    }
}

pub struct ExplorerData {
    pub sidebar_sections: Vec<PanelSection>,
    pub sidebar_empty: bool,
    pub empty_button_label: Option<String>,
    /// Structured panel items for per-row hit-target alignment (Editor Phase 4).
    pub panel_items: Option<Vec<ExplorerPanelItem>>,
    /// First visible explorer row (vertical scroll offset, in rows). Kept in
    /// sync with the widget tree via `ShellWorkContent::explorer_scroll_top`.
    pub scroll_top: usize,
}

impl Default for ExplorerData {
    fn default() -> Self {
        Self {
            sidebar_sections: Vec::new(),
            sidebar_empty: true,
            empty_button_label: None,
            panel_items: None,
            scroll_top: 0,
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
                // Tree begins flush at the top of the explorer content area: only a
                // small top inset, no search/divider/header band. (The non-rendered
                // search/header scaffolding in the widget tree is non-interactive and
                // does not affect hit testing.)
                let mut y_off = rect.y + pad;
                let max_y = rect.y + rect.h - EXPLORER_MAX_Y_INSET;
                let row_y_inset = (EXPLORER_ROW_H - EXPLORER_ROW_VIS_H) / 2.0;

                for (item_idx, item) in items.iter().enumerate() {
                    // Vertical scroll: skip the first `scroll_top` rows. The
                    // absolute `item_idx` stays in the block id so it matches the
                    // widget tree (hit-test / hover bridging).
                    if item_idx < data.scroll_top {
                        continue;
                    }
                    let row_h = EXPLORER_ROW_H;
                    if y_off + row_h > max_y {
                        break;
                    }
                    let indent_px = item.depth as f32 * EXPLORER_INDENT_PX;
                    let row_x = rect.x + pad + EXPLORER_ROW_TEXT_INSET + indent_px;
                    let row_y = y_off + row_y_inset;
                    let row_w = (inner_w - EXPLORER_ROW_W_REDUCTION - indent_px).max(4.0);
                    let row_h_vis = EXPLORER_ROW_VIS_H;

                    // Tree styling: inactive rows have no background at all
                    // (transparent), so the list reads as a file tree rather than
                    // a stack of buttons. The active/open row gets a flat,
                    // square-cornered highlight — calm and integrated, not a pill.
                    let fill = if item.is_active {
                        tokens.rail_item_active.to_array()
                    } else {
                        [0.0, 0.0, 0.0, 0.0]
                    };
                    let text_c = if item.is_active {
                        tokens.text_primary.to_array()
                    } else {
                        tokens.text_secondary.to_array()
                    };

                    // 1. Background / selection / hover block. Carries no text;
                    //    the hover bridge patches this block's `header_color`.
                    blocks.push(UiBlock {
                        id: format!("explorer_row_{}", item_idx),
                        title: String::new(),
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
                        clip_rect: None,
                        content_offset_x: 0.0,
                        content_offset_y: 0.0,
                        content_line_offset: None,
                    });

                    // 2. Disclosure + type-icon column (fixed width). Drawn at
                    //    `row_x`; clipped to the glyph column so a wide icon can't
                    //    bleed into the name column.
                    let glyph_text = icons::glyph_prefix(item.is_dir, item.expanded, &item.label);
                    blocks.push(explorer_text_block(
                        format!("explorer_glyph_{}", item_idx),
                        glyph_text,
                        row_x - EXPLORER_TITLE_PAD,
                        row_y,
                        EXPLORER_GLYPH_COL_W + EXPLORER_TITLE_PAD,
                        row_h_vis,
                        text_c,
                    ));

                    // 3. Filename column — fixed left edge at `row_x + glyph col`,
                    //    extending to the row's right edge (clips long names).
                    let name_text_x = row_x + EXPLORER_GLYPH_COL_W;
                    let name_x = name_text_x - EXPLORER_TITLE_PAD;
                    let name_w = (row_x + row_w - name_x).max(4.0);
                    blocks.push(explorer_text_block(
                        format!("explorer_name_{}", item_idx),
                        item.label.clone(),
                        name_x,
                        row_y,
                        name_w,
                        row_h_vis,
                        text_c,
                    ));
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
