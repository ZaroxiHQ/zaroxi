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
}

/// Compose all shell regions into UiBlocks by delegating to panel modules.
pub fn compose_blocks(
    regions: &[ShellRegion],
    tokens: &StyleTokens,
    ctx: &ShellBlockContext,
) -> Vec<UiBlock> {
    regions
        .iter()
        .map(|r| {
            let role = region_role(r.id);
            match role {
                PanelRole::TopBar => TopBarPanel::build_block(r, tokens),
                PanelRole::NavigationRail => RailPanel::build_rail_block(r, tokens),
                PanelRole::SidePanel => {
                    RailPanel::build_sidebar_block(r, tokens, &ctx.explorer_data)
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
                PanelRole::BottomPanel => EditorPanel::build_bottom_panel_block(r, tokens),
                PanelRole::BottomDock => BottomDockPanel::build_block(r, tokens),
                PanelRole::AuxiliaryPanelHeader => AiPanel::build_header_block(r, tokens),
                PanelRole::AuxiliaryPanelContent => {
                    AiPanel::build_content_block(r, tokens, &ctx.ai_data)
                }
                PanelRole::StatusBar => {
                    StatusBarPanel::build_block(r, tokens, &ctx.status_bar_data)
                }
            }
        })
        .collect()
}
