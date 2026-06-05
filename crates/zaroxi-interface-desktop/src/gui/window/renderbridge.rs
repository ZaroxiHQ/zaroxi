use crate::gui::ShellRegion;
use zaroxi_core_engine_render::{PanelColors, Rect, RenderLayout};
use zaroxi_core_engine_style::StyleTokens;

/// Build a `RenderLayout` from shell regions and resolved style tokens.
/// This converts the IDE shell region rects into the engine's render layout
/// plus the `PanelColors` color bag.
pub fn build_render_layout(regions: &[ShellRegion], tokens: &StyleTokens) -> RenderLayout {
    let find_rect = |role: zaroxi_core_engine_style::PanelRole| -> Rect {
        if let Some(r) = crate::gui::region_dispatch::find_region_by_role(regions, role) {
            Rect {
                x: r.rect.x as f32,
                y: r.rect.y as f32,
                w: r.rect.width as f32,
                h: r.rect.height as f32,
            }
        } else {
            Rect { x: 0.0, y: 0.0, w: 0.0, h: 0.0 }
        }
    };

    RenderLayout {
        title_bar: find_rect(zaroxi_core_engine_style::PanelRole::TopBar),
        sidebar: find_rect(zaroxi_core_engine_style::PanelRole::SidePanel),
        editor: find_rect(zaroxi_core_engine_style::PanelRole::ContentArea),
        right_panel: find_rect(zaroxi_core_engine_style::PanelRole::AuxiliaryPanelContent),
        bottom_panel: find_rect(zaroxi_core_engine_style::PanelRole::BottomDock),
        status_bar: find_rect(zaroxi_core_engine_style::PanelRole::StatusBar),
        colors: PanelColors {
            panel_header_background: tokens.panel_header_background.to_array(),
            panel_background: tokens.panel_background.to_array(),
            editor_cursor: tokens.editor_cursor.to_array(),
            editor_selection: tokens.editor_selection.to_array(),
            editor_line_highlight: tokens.editor_line_highlight.to_array(),
            text_default: tokens.text_primary.to_array(),
        },
    }
}
