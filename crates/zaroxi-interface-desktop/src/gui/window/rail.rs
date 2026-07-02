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

use crate::gui::window::destination::{DestSidebarRow, WorkbenchDestination};

use crate::gui::window::editor_shell::constants::{
    EXPLORER_GLYPH_COL_W, EXPLORER_INDENT_PX, EXPLORER_MAX_Y_INSET, EXPLORER_ROW_H,
    EXPLORER_ROW_TEXT_INSET, EXPLORER_ROW_VIS_H, EXPLORER_ROW_W_REDUCTION,
    EXPLORER_SEARCH_TO_ROWS_GAP, EXPLORER_TITLE_PAD, SEARCH_BAR_H, SIDEBAR_PAD,
    explorer_cta_button_rect,
};
use crate::gui::window::explorer_panel::icons;

/// Push the filename column for one row: truncates with an ellipsis to fit the
/// column width (using the exact monospace `char_advance`) and, when a search
/// query is active, highlights the matched substring in `match_color`. Pieces
/// are positioned by character offset so highlighting stays pixel-aligned.
#[allow(clippy::too_many_arguments)]
fn push_name_blocks(
    blocks: &mut Vec<UiBlock>,
    idx: usize,
    name: &str,
    query: &str,
    name_text_x: f32,
    name_w: f32,
    row_y: f32,
    row_h: f32,
    char_advance: f32,
    base_color: [f32; 4],
    match_color: [f32; 4],
) {
    let adv = if char_advance > 0.5 { char_advance } else { 8.0 };
    let right_edge = name_text_x + name_w;

    // Ellipsis truncation to the visible column width.
    let max_chars = (name_w / adv).floor().max(1.0) as usize;
    let chars: Vec<char> = name.chars().collect();
    let display: String = if chars.len() > max_chars {
        let keep = max_chars.saturating_sub(1).max(1);
        let mut s: String = chars[..keep].iter().collect();
        s.push('\u{2026}'); // …
        s
    } else {
        name.to_string()
    };

    // Locate the matched run in the (possibly truncated) display string.
    let match_range = if query.is_empty() {
        None
    } else {
        display.to_lowercase().find(&query.to_lowercase()).map(|byte_start| {
            let char_start = display[..byte_start].chars().count();
            (char_start, query.chars().count())
        })
    };

    let mut push_piece = |sub: &str, char_off: usize, color: [f32; 4], piece_id: String| {
        if sub.is_empty() {
            return;
        }
        let tx = name_text_x + char_off as f32 * adv;
        let bx = tx - EXPLORER_TITLE_PAD;
        let bw = (right_edge - bx).max(4.0);
        blocks.push(explorer_text_block(piece_id, sub.to_string(), bx, row_y, bw, row_h, color));
    };

    match match_range {
        Some((start, len)) => {
            let dchars: Vec<char> = display.chars().collect();
            let mid_end = (start + len).min(dchars.len());
            let pre: String = dchars[..start].iter().collect();
            let mid: String = dchars[start..mid_end].iter().collect();
            let post: String = dchars[mid_end..].iter().collect();
            push_piece(&pre, 0, base_color, format!("explorer_name_{}_a", idx));
            push_piece(&mid, start, match_color, format!("explorer_name_{}_b", idx));
            push_piece(&post, mid_end, base_color, format!("explorer_name_{}_c", idx));
        }
        None => {
            push_piece(&display, 0, base_color, format!("explorer_name_{}", idx));
        }
    }
}

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
        rect: zaroxi_core_engine_render::Rect { x, y, w, h },
        header_color: Some([0.0, 0.0, 0.0, 0.0]),
        header_only: true,
        text_color: Some(color),
        ..Default::default()
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
    /// Current search/filter query (empty = no filter). Rendered in the search
    /// box at the top of the explorer.
    pub search_query: String,
    /// Whether the search box currently holds keyboard focus.
    pub search_active: bool,
    /// Whether a workspace is loaded (controls whether the search box renders).
    pub has_workspace: bool,
    /// Exact monospace glyph advance (px) for ellipsis + match-run positioning.
    pub char_advance: f32,
    /// Keyboard-selected row (absolute index) while searching, if any.
    pub selected_row: Option<usize>,
    /// Whether the search caret should be drawn this frame (focus + blink phase).
    pub search_caret_visible: bool,
}

impl Default for ExplorerData {
    fn default() -> Self {
        Self {
            sidebar_sections: Vec::new(),
            sidebar_empty: true,
            empty_button_label: None,
            panel_items: None,
            scroll_top: 0,
            search_query: String::new(),
            search_active: false,
            has_workspace: false,
            char_advance: 8.0,
            selected_row: None,
            search_caret_visible: false,
        }
    }
}

/// Result of building the sidebar explorer section.
pub struct SidebarBlocks {
    pub blocks: Vec<UiBlock>,
    /// Hit rect for the CTA button, if present (x, y, w, h).
    pub cta_hit_rect: Option<(f32, f32, f32, f32)>,
    /// Hit rect for the search input box, if rendered (x, y, w, h). Clicking it
    /// focuses the explorer search for keyboard input.
    pub search_hit_rect: Option<(f32, f32, f32, f32)>,
    /// Hit rects for destination sidebar rows (Extensions list / Settings
    /// categories), in row order. Empty for the Explorer destination.
    pub row_hit_rects: Vec<(f32, f32, f32, f32)>,
}

pub struct RailPanel;

impl RailPanel {
    pub fn build_rail_block(r: &ShellRegion, tokens: &StyleTokens) -> UiBlock {
        UiBlock {
            id: r.id.to_string(),
            rect: r.into(),
            header_color: Some(tokens.rail_background.to_array()),
            content_color: None,
            ..Default::default()
        }
    }

    pub fn build_sidebar_block(
        r: &ShellRegion,
        tokens: &StyleTokens,
        data: &ExplorerData,
        dest: WorkbenchDestination,
        sidebar_list: &[DestSidebarRow],
    ) -> SidebarBlocks {
        let mut blocks = Vec::new();
        let mut cta_hit_rect: Option<(f32, f32, f32, f32)> = None;

        let rect: zaroxi_core_engine_render::Rect = r.into();

        // Background surface block
        blocks.push(UiBlock {
            id: r.id.to_string(),
            rect,
            header_color: Some(tokens.sidebar_background.to_array()),
            content_color: Some(tokens.sidebar_background.to_array()),
            ..Default::default()
        });

        // ── Destination sidebar (replaces the explorer tree) ──
        // For any non-Explorer destination the sidebar becomes a titled list of
        // destination rows (Extensions / Settings categories / facet rows). This
        // is what makes the left column visibly stop being the explorer.
        if !dest.is_explorer() {
            return Self::build_destination_sidebar(blocks, rect, tokens, dest, sidebar_list);
        }

        let pad = SIDEBAR_PAD;
        let inner_w = rect.w - pad * 2.0;

        // ── Search box (rendered when a workspace is loaded) ──
        let mut search_hit_rect: Option<(f32, f32, f32, f32)> = None;
        let mut content_top = rect.y + pad;
        if data.has_workspace {
            let sb_x = rect.x + pad;
            let sb_y = rect.y + pad;
            let sb_w = inner_w;
            let sb_h = SEARCH_BAR_H;

            // A blinking text caret (▏) marks the insertion point while the box
            // holds keyboard focus.
            let caret = if data.search_caret_visible { "\u{258F}" } else { "" };
            let (display, text_color) = if data.search_query.is_empty() {
                if data.search_active {
                    // Focused but empty: show the caret instead of the placeholder.
                    (
                        format!(" {}  {}", icons::glyph::SEARCH, caret),
                        tokens.text_primary.to_array(),
                    )
                } else {
                    (
                        format!(" {}  Search files…", icons::glyph::SEARCH),
                        tokens.text_muted.to_array(),
                    )
                }
            } else {
                (
                    format!(" {}  {}{}", icons::glyph::SEARCH, data.search_query, caret),
                    tokens.text_primary.to_array(),
                )
            };
            // Focus ring (accent border) when the box holds keyboard focus.
            let (border_color, border_width) = if data.search_active {
                (Some(tokens.accent.to_array()), 1.0)
            } else {
                (None, 0.0)
            };

            blocks.push(UiBlock {
                id: "explorer_search_box".to_string(),
                title: display,
                rect: zaroxi_core_engine_render::Rect { x: sb_x, y: sb_y, w: sb_w, h: sb_h },
                header_color: Some(tokens.sidebar_input.to_array()),
                corner_radius: 4.0,
                border_color,
                border_width,
                header_only: true,
                text_color: Some(text_color),
                ..Default::default()
            });

            search_hit_rect = Some((sb_x, sb_y, sb_w, sb_h));
            content_top = sb_y + sb_h + EXPLORER_SEARCH_TO_ROWS_GAP;
        }

        // ── Per-row blocks (aligned with widget tree hit regions) ──
        if let Some(ref items) = data.panel_items
            && !items.is_empty()
        {
            // Rows start below the rendered search box (see `content_top`).
            let mut y_off = content_top;
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
                // a stack of buttons. The active/open row — and the
                // keyboard-selected row while searching — get a flat,
                // square-cornered highlight (calm and integrated, not a pill).
                let is_selected = data.selected_row == Some(item_idx);
                let fill = if item.is_active || is_selected {
                    tokens.rail_item_active.to_array()
                } else {
                    [0.0, 0.0, 0.0, 0.0]
                };
                let text_c = if item.is_active || is_selected {
                    tokens.text_primary.to_array()
                } else {
                    tokens.text_secondary.to_array()
                };

                // 1. Background / selection / hover block. Carries no text;
                //    the hover bridge patches this block's `header_color`.
                blocks.push(UiBlock {
                    id: format!("explorer_row_{}", item_idx),
                    rect: zaroxi_core_engine_render::Rect {
                        x: row_x,
                        y: row_y,
                        w: row_w,
                        h: row_h_vis,
                    },
                    header_color: Some(fill),
                    header_only: true,
                    ..Default::default()
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

                // 3. Filename column — fixed left edge at `row_x + glyph col`.
                //    Truncates with an ellipsis and highlights the matched
                //    substring while a search query is active.
                let name_text_x = row_x + EXPLORER_GLYPH_COL_W;
                let name_w = (row_x + row_w - name_text_x).max(4.0);
                push_name_blocks(
                    &mut blocks,
                    item_idx,
                    &item.label,
                    &data.search_query,
                    name_text_x,
                    name_w,
                    row_y,
                    row_h_vis,
                    data.char_advance,
                    text_c,
                    tokens.accent.to_array(),
                );
                y_off += row_h;
            }
            return SidebarBlocks {
                blocks,
                cta_hit_rect,
                search_hit_rect,
                row_hit_rects: Vec::new(),
            };
        }

        // A workspace is loaded but there are no rows to show (empty folder or a
        // search that matched nothing). Keep the search box + a quiet hint; do
        // NOT fall through to the legacy text block (it would overlap the box).
        if data.has_workspace {
            if !data.search_query.is_empty() {
                blocks.push(explorer_text_block(
                    "explorer_no_matches".to_string(),
                    "  No matches".to_string(),
                    rect.x + pad,
                    content_top,
                    inner_w,
                    EXPLORER_ROW_VIS_H,
                    tokens.text_muted.to_array(),
                ));
            }
            return SidebarBlocks {
                blocks,
                cta_hit_rect,
                search_hit_rect,
                row_hit_rects: Vec::new(),
            };
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
            rect,
            content_spans,
            ..Default::default()
        });

        // CTA button for empty state
        if empty_message && let Some(ref btn_label) = data.empty_button_label {
            let (btn_x, btn_y, btn_w, btn_h) =
                explorer_cta_button_rect((rect.x, rect.y, rect.w, rect.h));
            let btn_rect =
                zaroxi_core_engine_render::Rect { x: btn_x, y: btn_y, w: btn_w, h: btn_h };

            blocks.push(UiBlock {
                id: "explorer_open_workspace_btn".to_string(),
                title: btn_label.clone(),
                content: btn_label.clone(),
                rect: btn_rect,
                header_color: Some(tokens.accent.to_array()),
                content_color: Some(tokens.accent.to_array()),
                corner_radius: 4.0,
                content_spans: Some(vec![(btn_label.clone(), tokens.text_primary.to_array())]),
                text_color: Some(tokens.text_primary.to_array()),
                ..Default::default()
            });

            cta_hit_rect = Some((btn_x, btn_y, btn_w, btn_h));
        }

        SidebarBlocks { blocks, cta_hit_rect, search_hit_rect, row_hit_rects: Vec::new() }
    }

    /// Build the sidebar for a non-Explorer destination: a titled header plus a
    /// list of destination rows. Returns the per-row hit rects so the host can
    /// route clicks to selection changes (Extensions / Settings).
    fn build_destination_sidebar(
        mut blocks: Vec<UiBlock>,
        rect: zaroxi_core_engine_render::Rect,
        tokens: &StyleTokens,
        dest: WorkbenchDestination,
        sidebar_list: &[DestSidebarRow],
    ) -> SidebarBlocks {
        let pad = SIDEBAR_PAD;
        let inner_w = (rect.w - pad * 2.0).max(4.0);
        let mut y = rect.y + pad;

        // Header title.
        let header_h = 28.0;
        blocks.push(UiBlock {
            id: "dest_sidebar_header".to_string(),
            title: dest.sidebar_title().to_string(),
            rect: zaroxi_core_engine_render::Rect { x: rect.x + pad, y, w: inner_w, h: header_h },
            header_color: Some(tokens.panel_header_background.to_array()),
            header_only: true,
            text_color: Some(tokens.panel_header_text.to_array()),
            ..Default::default()
        });
        y += header_h + 8.0;

        let row_h = 30.0;
        let mut row_hit_rects = Vec::new();
        for (i, row) in sidebar_list.iter().enumerate() {
            let row_y = y + i as f32 * (row_h + 2.0);
            if row_y + row_h > rect.y + rect.h {
                break;
            }
            let row_rect =
                zaroxi_core_engine_render::Rect { x: rect.x + pad, y: row_y, w: inner_w, h: row_h };
            let fill = if row.selected {
                tokens.rail_item_active.to_array()
            } else {
                [0.0, 0.0, 0.0, 0.0]
            };
            let text_c = if row.selected {
                tokens.text_primary.to_array()
            } else {
                tokens.text_secondary.to_array()
            };
            // Row background / selection highlight.
            blocks.push(UiBlock {
                id: format!("dest_row_bg_{}", i),
                rect: row_rect,
                header_color: Some(fill),
                corner_radius: 4.0,
                header_only: true,
                ..Default::default()
            });
            // Primary label.
            blocks.push(explorer_text_block(
                format!("dest_row_label_{}", i),
                format!("  {}", row.label),
                row_rect.x + 4.0,
                row_y,
                (inner_w - 64.0).max(4.0),
                row_h,
                text_c,
            ));
            // Trailing badge (Installed / Available / count).
            if !row.secondary.is_empty() {
                blocks.push(explorer_text_block(
                    format!("dest_row_badge_{}", i),
                    row.secondary.clone(),
                    row_rect.x + inner_w - 68.0,
                    row_y,
                    64.0,
                    row_h,
                    tokens.text_muted.to_array(),
                ));
            }
            row_hit_rects.push((row_rect.x, row_rect.y, row_rect.w, row_rect.h));
        }

        SidebarBlocks { blocks, cta_hit_rect: None, search_hit_rect: None, row_hit_rects }
    }
}
