/*!
frame.rs — shell composition coordinator.

Phase 50: dispatches each ShellRegion to the owning panel module for
UiBlock construction. Each panel module is the source of its own content.
app.rs only calls compose_blocks() with tokens + live state.
*/

use crate::gui::ShellRegion;
use crate::gui::region_dispatch::region_role;
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

/// Extract scrollbar widgets from the ShellWidgetTree and convert to UiBlocks
/// so they are rendered in the GUI path (scrollbars were previously interaction-only).
pub fn extract_scrollbar_blocks(
    widget_tree: &zaroxi_core_engine_ui::ShellWidgetTree,
) -> Vec<UiBlock> {
    let mut blocks = Vec::new();
    for widget in &widget_tree.widgets {
        if let zaroxi_core_engine_ui::ShellWidget::ScrollBar {
            id: _,
            track_rect,
            thumb_rect,
            track_fill,
            thumb_fill,
            state: _,
        } = widget
        {
            // Track
            blocks.push(UiBlock {
                id: "scrollbar_track".to_string(),
                title: String::new(),
                content: String::new(),
                visible: true,
                rect: zaroxi_core_engine_render::Rect {
                    x: track_rect.x,
                    y: track_rect.y,
                    w: track_rect.width,
                    h: track_rect.height,
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
            });
            // Thumb
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
) -> Vec<UiBlock> {
    let blocks: Vec<UiBlock> = regions
        .iter()
        .map(|r| {
            let role = region_role(r.id);
            let block = match role {
                PanelRole::TopBar => TopBarPanel::build_block(r, tokens),
                PanelRole::NavigationRail => RailPanel::build_rail_block(r, tokens),
                PanelRole::SidePanel => {
                    RailPanel::build_sidebar_block(r, tokens, &ctx.explorer_data)
                }
                PanelRole::GutterLane => {
                    let line_count = ctx.editor_data.cursor_line.max(1);
                    EditorPanel::build_gutter_block(r, tokens, line_count)
                }
                PanelRole::ContentTabStrip => {
                    EditorPanel::build_tab_strip_block(r, tokens, &ctx.editor_data)
                }
                PanelRole::ContentBreadcrumb => {
                    EditorPanel::build_breadcrumb_block(r, tokens, &ctx.editor_data)
                }
                PanelRole::ContentArea => {
                    EditorPanel::build_content_area_block(r, tokens, &ctx.editor_data)
                }
                PanelRole::MinimapLane => EditorPanel::build_minimap_block(r, tokens),
                PanelRole::BottomPanel => {
                    EditorPanel::build_bottom_panel_block(r, tokens, ctx.terminal_tabs.as_deref())
                }
                PanelRole::BottomDock => BottomDockPanel::build_block(r, tokens),
                PanelRole::AuxiliaryPanelHeader => AiPanel::build_header_block(r, tokens),
                PanelRole::AuxiliaryPanelContent => {
                    AiPanel::build_content_block(r, tokens, &ctx.ai_data)
                }
                PanelRole::StatusBar => {
                    StatusBarPanel::build_block(r, tokens, &ctx.status_bar_data)
                }
            };
            block
        })
        .collect();

    log::debug!("ZAROXI_PANEL_COMPOSE: built {} blocks from panel modules", blocks.len());
    blocks
}
