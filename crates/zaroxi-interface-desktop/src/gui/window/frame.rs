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
use zaroxi_core_engine_render::{Rect, UiBlock};
use zaroxi_core_engine_style::{PanelRole, StyleTokens};

use super::ai_pane::{AiPanel, AiPanelData};
use super::bottom_panel::BottomDockPanel;
use super::destination::{DestSidebarRow, WorkbenchDestination};
use super::editor::{EditorContentData, EditorPanel};
use super::rail::{ExplorerData, RailPanel};
use super::status_bar::{StatusModel, StatusView};
use super::toolbar::TopBarPanel;

pub struct ShellBlockContext {
    pub editor_data: EditorContentData,
    pub explorer_data: ExplorerData,
    pub status_bar_data: StatusModel,
    pub ai_data: AiPanelData,
    pub terminal_tabs: Option<Vec<String>>,
    /// Active workbench destination — the single routing concept that decides
    /// whether the sidebar/editor render the Explorer (file) view or a
    /// destination-specific view (Extensions / Settings / placeholder).
    pub destination: WorkbenchDestination,
    /// Destination sidebar rows (Extensions list / Settings categories / facet
    /// rows). Empty for Explorer (the explorer tree owns the sidebar).
    pub sidebar_list: Vec<DestSidebarRow>,
    /// When `true`, the cockpit overlay is actively producing status text
    /// and the shell path should emit only the background strip (no
    /// breadcrumb) to avoid duplicated text.  False during startup and the
    /// first few frames before the cockpit pipeline has produced its first
    /// text run.
    pub cockpit_text_active: bool,
    /// When `true`, the Welcome page is active — no file editor content
    /// (gutter, text, empty state) should be rendered. The cockpit provides
    /// the Welcome screen instead.
    pub welcome_active: bool,
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
    sidebar_scroll_offset: f32,
) -> Vec<UiBlock> {
    let mut blocks = Vec::new();

    let editor_region =
        crate::gui::region_dispatch::find_region_by_role(regions, PanelRole::ContentArea);
    let sidebar_region =
        crate::gui::region_dispatch::find_region_by_role(regions, PanelRole::SidePanel);
    let bottom_panel_region =
        crate::gui::region_dispatch::find_region_by_role(regions, PanelRole::BottomPanel);

    // ── Editor scrollbar (thumb-only Zed-style, no visible track) ──
    // The thumb floats at the outer edge of the content area. No track bar
    // is rendered — the track was a full-height rectangle that overlapped the
    // minimap rail, creating a stray vertical line between the editor text and
    // the minimap. Only the draggable thumb is visible, like a modern IDE.
    if let Some(editor) = editor_region {
        let needs_scroll = editor_total_lines > editor_visible_lines.max(1);
        if needs_scroll && editor.rect.width > 20 && editor.rect.height > 40 {
            let ex = editor.rect.x as f32;
            let ey = editor.rect.y as f32;
            let ew = editor.rect.width as f32;
            let eh = editor.rect.height as f32;

            let ratio = editor_visible_lines as f32 / editor_total_lines.max(1) as f32;
            // Slightly wider + no inset so the thumb hugs the outer edge, outside
            // the minimap, evenly away from the minimap texture.
            let sb_w = 8.0f32;
            let inset_r = 0.0f32;
            let spec = ScrollbarSpec {
                sb_width: sb_w,
                inset_right: inset_r,
                track_inset_y: SB_EDITOR_SPEC.track_inset_y,
                track_h_reduction: SB_EDITOR_SPEC.track_h_reduction,
                thumb_ratio: ratio.clamp(0.05, 1.0),
                thumb_min_h: SB_EDITOR_SPEC.thumb_min_h,
            };
            let (sb_x, track_y, _sb_w, track_h, thumb_h) =
                compute_scrollbar_geometry((ex, ey, ew, eh), &spec, 0.0);

            // ── Thumb only (no track background) ──
            blocks.push(UiBlock {
                id: "scrollbar_thumb_editor".to_string(),
                rect: zaroxi_core_engine_render::Rect {
                    x: sb_x,
                    y: track_y + editor_scroll_offset * (track_h - thumb_h).max(0.0),
                    w: sb_w,
                    h: thumb_h,
                },
                header_color: Some(tokens.editor_scrollbar_thumb.to_array()),
                corner_radius: 4.0,
                header_only: true,
                ..Default::default()
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

            // Proportional thumb sized to the visible/total row ratio.
            let ratio = sidebar_visible as f32 / sidebar_items.max(1) as f32;
            let spec = ScrollbarSpec {
                sb_width: SB_SIDEBAR_SPEC.sb_width,
                inset_right: SB_SIDEBAR_SPEC.inset_right,
                track_inset_y: SB_SIDEBAR_SPEC.track_inset_y,
                track_h_reduction: SB_SIDEBAR_SPEC.track_h_reduction,
                thumb_ratio: ratio.clamp(0.05, 1.0),
                thumb_min_h: SB_SIDEBAR_SPEC.thumb_min_h,
            };
            let (sb_x, track_y, sb_w, track_h, thumb_h) =
                compute_scrollbar_geometry((sx, sy, sw, sh), &spec, 0.0);
            let track_rect =
                zaroxi_core_engine_render::Rect { x: sb_x, y: track_y, w: sb_w, h: track_h };

            blocks.push(UiBlock {
                id: "scrollbar_track_sidebar".to_string(),
                rect: track_rect,
                header_color: Some(tokens.sidebar_scrollbar_track.to_array()),
                corner_radius: 3.0,
                header_only: true,
                ..Default::default()
            });

            blocks.push(UiBlock {
                id: "scrollbar_thumb_sidebar".to_string(),
                rect: zaroxi_core_engine_render::Rect {
                    x: track_rect.x,
                    y: track_rect.y + sidebar_scroll_offset * (track_rect.h - thumb_h).max(0.0),
                    w: sb_w,
                    h: thumb_h,
                },
                header_color: Some(tokens.sidebar_scrollbar_thumb.to_array()),
                corner_radius: 2.0,
                header_only: true,
                ..Default::default()
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
                rect: track_rect,
                header_color: Some(tokens.bottom_scrollbar_track.to_array()),
                corner_radius: 3.0,
                header_only: true,
                ..Default::default()
            });

            blocks.push(UiBlock {
                id: "scrollbar_thumb_bottom".to_string(),
                rect: zaroxi_core_engine_render::Rect {
                    x: track_rect.x,
                    y: track_rect.y,
                    w: sb_w,
                    h: thumb_h,
                },
                header_color: Some(tokens.bottom_scrollbar_thumb.to_array()),
                corner_radius: 2.0,
                header_only: true,
                ..Default::default()
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
                rect: zaroxi_core_engine_render::Rect {
                    x: thumb_rect.x,
                    y: thumb_rect.y,
                    w: thumb_rect.width,
                    h: thumb_rect.height,
                },
                header_color: Some(*track_fill),
                corner_radius: 3.0,
                header_only: true,
                ..Default::default()
            });
            blocks.push(UiBlock {
                id: "scrollbar_thumb".to_string(),
                rect: zaroxi_core_engine_render::Rect {
                    x: thumb_rect.x,
                    y: thumb_rect.y,
                    w: thumb_rect.width,
                    h: thumb_rect.height,
                },
                header_color: Some(*thumb_fill),
                corner_radius: 2.0,
                header_only: true,
                ..Default::default()
            });
        }
    }

    blocks
}
/// Compose all shell regions into UiBlocks by delegating to panel modules.
/// An optional UI hit rectangle (x, y, w, h).
type HitRect = Option<(f32, f32, f32, f32)>;

/// Output of [`compose_blocks`]: the draw blocks, the explorer CTA + search hit
/// rects, and the destination sidebar row hit rects (in row order).
type ComposedBlocks = (Vec<UiBlock>, HitRect, HitRect, Vec<(f32, f32, f32, f32)>);

pub fn compose_blocks(
    regions: &[ShellRegion],
    tokens: &StyleTokens,
    ctx: &ShellBlockContext,
) -> ComposedBlocks {
    let mut blocks: Vec<UiBlock> = Vec::new();
    let mut explorer_cta_rect: HitRect = None;
    let mut explorer_search_rect: HitRect = None;
    let mut sidebar_row_hit_rects: Vec<(f32, f32, f32, f32)> = Vec::new();
    for r in regions {
        let role = region_role(r.id);
        match role {
            PanelRole::TopBar => blocks.push(TopBarPanel::build_block(r, tokens)),
            PanelRole::NavigationRail => {
                // Rail background strip via the shell shape pass (rendered
                // before the text pass so cockpit icon glyphs draw on top).
                // Rail item highlights + icons are cockpit-owned (ActivityRail
                // widget in the vello overlay + cosmic-text layers).
                blocks.push(RailPanel::build_rail_block(r, tokens));
            }
            PanelRole::SidePanel => {
                let sidebar = RailPanel::build_sidebar_block(
                    r,
                    tokens,
                    &ctx.explorer_data,
                    ctx.destination,
                    &ctx.sidebar_list,
                );
                blocks.extend(sidebar.blocks);
                explorer_cta_rect = sidebar.cta_hit_rect;
                explorer_search_rect = sidebar.search_hit_rect;
                sidebar_row_hit_rects = sidebar.row_hit_rects;
            }
            PanelRole::GutterLane => {
                if !ctx.welcome_active {
                    blocks.push(EditorPanel::build_gutter_block(
                        r,
                        tokens,
                        ctx.editor_data.total_lines,
                        ctx.editor_data.visible_line_range,
                        ctx.destination,
                        &ctx.editor_data.visual_to_logical,
                        ctx.editor_data.total_visual_lines,
                    ));
                }
            }
            PanelRole::ContentTabStrip => {
                blocks.push(EditorPanel::build_tab_strip_block(
                    r,
                    tokens,
                    &ctx.editor_data,
                    ctx.destination,
                ));
            }
            PanelRole::ContentBreadcrumb => {
                if !ctx.welcome_active {
                    blocks.push(EditorPanel::build_breadcrumb_block(r, tokens));
                }
            }
            PanelRole::ContentArea => {
                if ctx.welcome_active {
                    blocks.push(UiBlock::default());
                } else {
                    blocks.push(EditorPanel::build_content_area_block(
                        r,
                        tokens,
                        &ctx.editor_data,
                        ctx.destination,
                    ));
                }
            }
            PanelRole::MinimapLane => {
                // No legacy shell minimap surface. The overview/minimap is owned
                // by the cockpit/widget layer (editor-edge). No shell region maps
                // to this role anymore; the arm exists only for match exhaustiveness.
            }
            PanelRole::BottomPanel => blocks.push(EditorPanel::build_bottom_panel_block(
                r,
                tokens,
                ctx.terminal_tabs.as_deref(),
            )),
            PanelRole::BottomDock => blocks.push(BottomDockPanel::build_block(r, tokens)),
            PanelRole::AuxiliaryPanelHeader => blocks.push(AiPanel::build_header_block(r, tokens)),
            PanelRole::AuxiliaryPanelContent => {
                blocks.extend(AiPanel::build_content_block(r, tokens, &ctx.ai_data));
            }
            PanelRole::StatusBar => {
                // Ownership: the cockpit/widget status bar is the default owner.
                // The legacy shell status block renders ONLY under the explicit
                // legacy fallback, so exactly one status surface is ever active
                // (never both; the cockpit overlay covers the default case).
                let legacy = super::cockpit::legacy_shell_surfaces();
                if std::env::var("ZAROXI_STATUS_TRACE").as_deref() == Ok("1") {
                    eprintln!(
                        "ZAROXI_STATUS_TRACE: status_owner={} legacy_fallback_enabled={}",
                        if legacy { "legacy" } else { "cockpit" },
                        legacy,
                    );
                }
                if legacy {
                    blocks.push(StatusView::build_block(r, tokens, &ctx.status_bar_data));
                } else if ctx.cockpit_text_active {
                    // Cockpit owns the status TEXT (drawn by the cosmic-text pass).
                    // Draw only the elevated strip background here, in the shell
                    // shape pass that runs BEFORE the text pass, so the cockpit
                    // status text lands on top of it (the vello overlay composite
                    // runs after text, so the strip background cannot live there).
                    blocks.push(StatusView::build_background_block(r, tokens));
                } else {
                    // Cockpit text is not yet active (startup / first few frames).
                    // Emit the breadcrumb immediately so the status bar is never
                    // blank while the cockpit pipeline initialises. Once the
                    // cockpit begins producing text we switch to background-only.
                    blocks.push(StatusView::build_background_with_breadcrumb(
                        r,
                        tokens,
                        &ctx.status_bar_data,
                    ));
                }
            }
        };
    }

    // ── Panel seams — structural 1px separators between major regions ────────
    // Single-edge only (never boxed). Hierarchy: `divider_default` (subtle) for
    // region splits; `border_strong` for the editor↔AI and editor↔bottom seams.
    // Appended last so the seams paint on top of the panel fills.
    {
        let region_rect = |role: PanelRole| -> Option<(f32, f32, f32, f32)> {
            regions.iter().find(|r| region_role(r.id) == role).map(|r| {
                (r.rect.x as f32, r.rect.y as f32, r.rect.width as f32, r.rect.height as f32)
            })
        };
        let seam_subtle = tokens.divider_default.to_array();
        let seam_strong = tokens.border_strong.to_array();
        let seam = |id: &str, x: f32, y: f32, w: f32, h: f32, color: [f32; 4]| UiBlock {
            id: id.to_string(),
            rect: Rect { x, y, w, h },
            header_only: true,
            header_color: Some(color),
            ..Default::default()
        };

        // Explorer ↔ Editor — right edge of the file explorer (subtle).
        if let Some((x, y, w, h)) =
            region_rect(PanelRole::SidePanel).filter(|&(_, _, w, h)| w > 1.0 && h > 0.0)
        {
            blocks.push(seam("seam.explorer_editor", x + w - 1.0, y, 1.0, h, seam_subtle));
        }

        // Editor ↔ AI assistant — left edge of the assistant column (strong).
        // Spans header + content as one clean full-height seam.
        {
            let ai: Vec<(f32, f32, f32, f32)> = [
                region_rect(PanelRole::AuxiliaryPanelHeader),
                region_rect(PanelRole::AuxiliaryPanelContent),
            ]
            .into_iter()
            .flatten()
            .collect();
            if let Some(first) = ai.first().copied() {
                let top = ai.iter().map(|r| r.1).fold(f32::INFINITY, f32::min);
                let bottom = ai.iter().map(|r| r.1 + r.3).fold(f32::NEG_INFINITY, f32::max);
                let h = (bottom - top).max(0.0);
                if h > 0.0 {
                    blocks.push(seam("seam.editor_ai", first.0, top, 1.0, h, seam_strong));
                }
            }
        }

        // Editor ↔ Bottom terminal — top edge of the bottom panel (strong).
        if let Some((x, y, w, _)) =
            region_rect(PanelRole::BottomPanel).filter(|&(_, _, w, _)| w > 0.0)
        {
            blocks.push(seam("seam.editor_bottom", x, y, w, 1.0, seam_strong));
        }

        // Tabs ↔ Editor body — bottom edge of the tab strip (subtle).
        if let Some((x, y, w, h)) =
            region_rect(PanelRole::ContentTabStrip).filter(|&(_, _, w, h)| w > 0.0 && h > 1.0)
        {
            blocks.push(seam("seam.tabs_editor", x, y + h - 1.0, w, 1.0, seam_subtle));
        }

        // Title bar ↔ content — bottom edge of the top chrome (subtle).
        if let Some((x, y, w, h)) =
            region_rect(PanelRole::TopBar).filter(|&(_, _, w, h)| w > 0.0 && h > 1.0)
        {
            blocks.push(seam("seam.titlebar", x, y + h - 1.0, w, 1.0, seam_subtle));
        }

        // Status bar ↔ content — top edge of the footer (subtle).
        if let Some((x, y, w, _)) =
            region_rect(PanelRole::StatusBar).filter(|&(_, _, w, _)| w > 0.0)
        {
            blocks.push(seam("seam.status", x, y, w, 1.0, seam_subtle));
        }
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

    (blocks, explorer_cta_rect, explorer_search_rect, sidebar_row_hit_rects)
}
