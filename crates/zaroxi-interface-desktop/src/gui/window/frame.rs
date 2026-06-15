/*!
frame.rs — shell composition coordinator.

Phase 50: dispatches each ShellRegion to the owning panel module for
UiBlock construction. Each panel module is the source of its own content.
app.rs only calls compose_blocks() with tokens + live state.

Phase 69: scrollbar blocks are now computed from ShellFrame regions
directly (not extracted from the widget tree) to guarantee correct
spatial placement regardless of the widget-tree layout system.
*/

use crate::gui::ShellRegion;
use crate::gui::region_dispatch::region_role;
use crate::gui::window::editor_shell::constants::{
    SB_BOTTOM_SPEC, SB_EDITOR_SPEC, SB_SIDEBAR_SPEC, ScrollbarSpec, compute_scrollbar_geometry,
};
use zaroxi_core_engine_render::UiBlock;
use zaroxi_core_engine_style::{PanelRole, StyleTokens};

use super::ai_pane::{AiPanel, AiPanelData};
use super::bottom_panel::BottomDockPanel;
use super::editor::{EditorContentData, EditorPanel};
use super::rail::{ExplorerData, RailPanel};
use super::status_bar::{StatusBarData, StatusBarPanel};
use super::toolbar::TopBarPanel;

pub struct ShellBlockContext {
    pub editor_data: EditorContentData,
    pub explorer_data: ExplorerData,
    pub status_bar_data: StatusBarData,
    pub ai_data: AiPanelData,
    pub terminal_tabs: Option<Vec<String>>,
}

/// Compute scrollbar UiBlocks directly from ShellFrame regions.
///
/// Each scrollbar is only emitted when the corresponding content overflows
/// its visible region (`items > visible_items`). This prevents phantom
/// full-height track quads from appearing when no scrolling is needed.
pub fn compute_scrollbar_blocks(
    regions: &[ShellRegion],
    tokens: &StyleTokens,
    editor_total_lines: usize,
    editor_visible_lines: usize,
    sidebar_items: usize,
    sidebar_visible: usize,
    bottom_lines: usize,
    bottom_visible: usize,
    editor_scroll_offset: f32,
) -> Vec<UiBlock> {
    let mut blocks = Vec::new();

    let editor_region =
        crate::gui::region_dispatch::find_region_by_role(regions, PanelRole::ContentArea);
    let sidebar_region =
        crate::gui::region_dispatch::find_region_by_role(regions, PanelRole::SidePanel);
    let bottom_panel_region =
        crate::gui::region_dispatch::find_region_by_role(regions, PanelRole::BottomPanel);

    // ── Editor scrollbar ──
    if let Some(editor) = editor_region {
        let needs_scroll = editor_total_lines > editor_visible_lines.max(1);
        if needs_scroll && editor.rect.width > 20 && editor.rect.height > 40 {
            let ex = editor.rect.x as f32;
            let ey = editor.rect.y as f32;
            let ew = editor.rect.width as f32;
            let eh = editor.rect.height as f32;

            // Compute proportional thumb ratio from visible/total lines
            let ratio = editor_visible_lines as f32 / editor_total_lines.max(1) as f32;
            let spec = ScrollbarSpec {
                sb_width: SB_EDITOR_SPEC.sb_width,
                inset_right: SB_EDITOR_SPEC.inset_right,
                track_inset_y: SB_EDITOR_SPEC.track_inset_y,
                track_h_reduction: SB_EDITOR_SPEC.track_h_reduction,
                thumb_ratio: ratio.clamp(0.05, 1.0),
                thumb_min_h: SB_EDITOR_SPEC.thumb_min_h,
            };
            let (sb_x, track_y, sb_w, track_h, thumb_h) =
                compute_scrollbar_geometry((ex, ey, ew, eh), &spec, 0.0);
            let track_rect =
                zaroxi_core_engine_render::Rect { x: sb_x, y: track_y, w: sb_w, h: track_h };

            blocks.push(UiBlock {
                id: "scrollbar_track_editor".to_string(),
                title: String::new(),
                content: String::new(),
                visible: true,
                rect: track_rect,
                header_color: Some(tokens.editor_scrollbar_track.to_array()),
                content_color: None,
                corner_radius: 3.0,
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

            blocks.push(UiBlock {
                id: "scrollbar_thumb_editor".to_string(),
                title: String::new(),
                content: String::new(),
                visible: true,
                rect: zaroxi_core_engine_render::Rect {
                    x: track_rect.x,
                    y: track_rect.y + editor_scroll_offset * (track_rect.h - thumb_h).max(0.0),
                    w: sb_w,
                    h: thumb_h,
                },
                header_color: Some(tokens.editor_scrollbar_thumb.to_array()),
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
                text_color: None,
                clip_rect: None,
                content_offset_x: 0.0,
                content_offset_y: 0.0,
                content_line_offset: None,
            });
        }
    }

    // ── Sidebar scrollbar ──
    if let Some(sidebar) = sidebar_region {
        let needs_scroll = sidebar_items > sidebar_visible.max(1);
        if needs_scroll && sidebar.rect.width > 20 && sidebar.rect.height > 200 {
            let sx = sidebar.rect.x as f32;
            let sy = sidebar.rect.y as f32;
            let sw = sidebar.rect.width as f32;
            let sh = sidebar.rect.height as f32;

            let (sb_x, track_y, sb_w, track_h, thumb_h) =
                compute_scrollbar_geometry((sx, sy, sw, sh), &SB_SIDEBAR_SPEC, 0.0);
            let track_rect =
                zaroxi_core_engine_render::Rect { x: sb_x, y: track_y, w: sb_w, h: track_h };

            blocks.push(UiBlock {
                id: "scrollbar_track_sidebar".to_string(),
                title: String::new(),
                content: String::new(),
                visible: true,
                rect: track_rect,
                header_color: Some(tokens.sidebar_scrollbar_track.to_array()),
                content_color: None,
                corner_radius: 3.0,
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

            blocks.push(UiBlock {
                id: "scrollbar_thumb_sidebar".to_string(),
                title: String::new(),
                content: String::new(),
                visible: true,
                rect: zaroxi_core_engine_render::Rect {
                    x: track_rect.x,
                    y: track_rect.y,
                    w: sb_w,
                    h: thumb_h,
                },
                header_color: Some(tokens.sidebar_scrollbar_thumb.to_array()),
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
                text_color: None,
                clip_rect: None,
                content_offset_x: 0.0,
                content_offset_y: 0.0,
                content_line_offset: None,
            });
        }
    }

    // ── Bottom panel scrollbar ──
    if let Some(bottom) = bottom_panel_region {
        let needs_scroll = bottom_lines > bottom_visible.max(1);
        if needs_scroll && bottom.rect.width > 20 && bottom.rect.height > 40 {
            let bx = bottom.rect.x as f32;
            let by = bottom.rect.y as f32;
            let bw = bottom.rect.width as f32;
            let bh = bottom.rect.height as f32;

            let (sb_x, track_y, sb_w, track_h, thumb_h) =
                compute_scrollbar_geometry((bx, by, bw, bh), &SB_BOTTOM_SPEC, 0.0);
            let track_rect =
                zaroxi_core_engine_render::Rect { x: sb_x, y: track_y, w: sb_w, h: track_h };

            blocks.push(UiBlock {
                id: "scrollbar_track_bottom".to_string(),
                title: String::new(),
                content: String::new(),
                visible: true,
                rect: track_rect,
                header_color: Some(tokens.bottom_scrollbar_track.to_array()),
                content_color: None,
                corner_radius: 3.0,
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

            blocks.push(UiBlock {
                id: "scrollbar_thumb_bottom".to_string(),
                title: String::new(),
                content: String::new(),
                visible: true,
                rect: zaroxi_core_engine_render::Rect {
                    x: track_rect.x,
                    y: track_rect.y,
                    w: sb_w,
                    h: thumb_h,
                },
                header_color: Some(tokens.bottom_scrollbar_thumb.to_array()),
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
                text_color: None,
                clip_rect: None,
                content_offset_x: 0.0,
                content_offset_y: 0.0,
                content_line_offset: None,
            });
        }
    }

    // Phase 69: gated debug instrumentation for scrollbar blocks.
    if std::env::var("ZAROXI_DEBUG_VBLOCKS").as_deref() == Ok("1") {
        for blk in &blocks {
            eprintln!(
                "ZAROXI_SCROLLBAR: id='{}' x={:.1} y={:.1} w={:.1} h={:.1} header_color={:?}",
                blk.id, blk.rect.x, blk.rect.y, blk.rect.w, blk.rect.h, blk.header_color,
            );
        }
    }

    blocks
}

/// Extract scrollbar widgets from the ShellWidgetTree and convert to UiBlocks.
///
/// DEPRECATED: prefer `compute_scrollbar_blocks` which derives positions from
/// ShellFrame regions (the same layout system used by compose_blocks). Retained
/// for backward compatibility and for potential interaction-driven rendering.
#[allow(dead_code)]
pub fn extract_scrollbar_blocks(
    widget_tree: &zaroxi_core_engine_ui::ShellWidgetTree,
) -> Vec<UiBlock> {
    let mut blocks = Vec::new();
    for widget in &widget_tree.widgets {
        if let zaroxi_core_engine_ui::ShellWidget::ScrollBar {
            id: _,
            track_rect: _,
            thumb_rect,
            track_fill,
            thumb_fill,
            state: _,
        } = widget
        {
            blocks.push(UiBlock {
                id: "scrollbar_track".to_string(),
                title: String::new(),
                content: String::new(),
                visible: true,
                rect: zaroxi_core_engine_render::Rect {
                    x: thumb_rect.x,
                    y: thumb_rect.y,
                    w: thumb_rect.width,
                    h: thumb_rect.height,
                },
                header_color: Some(*track_fill),
                content_color: None,
                corner_radius: 3.0,
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
            blocks.push(UiBlock {
                id: "scrollbar_thumb".to_string(),
                title: String::new(),
                content: String::new(),
                visible: true,
                rect: zaroxi_core_engine_render::Rect {
                    x: thumb_rect.x,
                    y: thumb_rect.y,
                    w: thumb_rect.width,
                    h: thumb_rect.height,
                },
                header_color: Some(*thumb_fill),
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
                text_color: None,
                clip_rect: None,
                content_offset_x: 0.0,
                content_offset_y: 0.0,
                content_line_offset: None,
            });
        }
    }

    blocks
}
/// Compose all shell regions into UiBlocks by delegating to panel modules.
pub fn compose_blocks(
    regions: &[ShellRegion],
    tokens: &StyleTokens,
    ctx: &ShellBlockContext,
) -> (Vec<UiBlock>, Option<(f32, f32, f32, f32)>) {
    let mut blocks: Vec<UiBlock> = Vec::new();
    let mut explorer_cta_rect: Option<(f32, f32, f32, f32)> = None;
    for r in regions {
        let role = region_role(r.id);
        match role {
            PanelRole::TopBar => blocks.push(TopBarPanel::build_block(r, tokens)),
            PanelRole::NavigationRail => blocks.push(RailPanel::build_rail_block(r, tokens)),
            PanelRole::SidePanel => {
                let sidebar = RailPanel::build_sidebar_block(r, tokens, &ctx.explorer_data);
                blocks.extend(sidebar.blocks);
                explorer_cta_rect = sidebar.cta_hit_rect;
            }
            PanelRole::GutterLane => {
                let line_count = ctx.editor_data.total_lines.max(1);
                blocks.push(EditorPanel::build_gutter_block(r, tokens, line_count));
            }
            PanelRole::ContentTabStrip => {
                blocks.push(EditorPanel::build_tab_strip_block(r, tokens, &ctx.editor_data));
            }
            PanelRole::ContentBreadcrumb => {
                blocks.push(EditorPanel::build_breadcrumb_block(r, tokens, &ctx.editor_data));
            }
            PanelRole::ContentArea => {
                blocks.push(EditorPanel::build_content_area_block(r, tokens, &ctx.editor_data));
            }
            PanelRole::MinimapLane => blocks.push(EditorPanel::build_minimap_block(r, tokens)),
            PanelRole::BottomPanel => blocks.push(EditorPanel::build_bottom_panel_block(
                r,
                tokens,
                ctx.terminal_tabs.as_deref(),
            )),
            PanelRole::BottomDock => blocks.push(BottomDockPanel::build_block(r, tokens)),
            PanelRole::AuxiliaryPanelHeader => blocks.push(AiPanel::build_header_block(r, tokens)),
            PanelRole::AuxiliaryPanelContent => {
                blocks.push(AiPanel::build_content_block(r, tokens, &ctx.ai_data));
            }
            PanelRole::StatusBar => {
                blocks.push(StatusBarPanel::build_block(r, tokens, &ctx.status_bar_data));
            }
        };
    }

    log::debug!("ZAROXI_PANEL_COMPOSE: built {} blocks from panel modules", blocks.len());

    // Phase 69: gated debug instrumentation for vertical block audit.
    // Enable with ZAROXI_DEBUG_VBLOCKS=1 to log every block taller than wide.
    if std::env::var("ZAROXI_DEBUG_VBLOCKS").as_deref() == Ok("1") {
        for blk in &blocks {
            let tall = blk.rect.h > blk.rect.w && blk.rect.h > 4.0;
            let narrow = blk.rect.w > 0.0 && blk.rect.w <= 10.0;
            if tall || narrow {
                eprintln!(
                    "ZAROXI_VBLOCK: id='{}' x={:.1} y={:.1} w={:.1} h={:.1} border_color={:?} border_w={:.2} header_only={} header_color={:?}",
                    blk.id,
                    blk.rect.x,
                    blk.rect.y,
                    blk.rect.w,
                    blk.rect.h,
                    blk.border_color,
                    blk.border_width,
                    blk.header_only,
                    blk.header_color,
                );
            }
        }
    }

    (blocks, explorer_cta_rect)
}
