/*!
frame.rs coordinator (pre-GUI-8 refactor)

This file is now a thin coordinator: it computes nothing itself beyond
delegating each ShellRegion to the appropriate per-panel module. All panel
placeholder drawing logic has been moved into dedicated modules so each panel
owns its own draw behavior.

Behavior is preserved exactly by delegating the same region ids to the
corresponding module draw functions and concatenating their returned rects.
*/

/// Build the small set of overlay rects used for the one-shot clear+present.
/// Uses PanelRole-based dispatch instead of string-matching on region IDs.
pub fn build_overlay_rects(
    shell: &crate::gui::ShellFrame,
) -> Vec<zaroxi_core_engine_render_backend::DrawRect> {
    let mut rects: Vec<zaroxi_core_engine_render_backend::DrawRect> = Vec::new();

    for r in &shell.regions {
        let role = crate::gui::region_dispatch::region_role(r.id);
        let mut produced: Vec<zaroxi_core_engine_render_backend::DrawRect> = match role {
            zaroxi_core_engine_style::PanelRole::TopBar => super::toolbar::draw(r, &shell.theme),
            zaroxi_core_engine_style::PanelRole::NavigationRail
            | zaroxi_core_engine_style::PanelRole::SidePanel => {
                super::rail::draw(r, &shell.theme, shell.work_content.as_ref())
            }
            zaroxi_core_engine_style::PanelRole::ContentTabStrip
            | zaroxi_core_engine_style::PanelRole::ContentBreadcrumb
            | zaroxi_core_engine_style::PanelRole::ContentArea
            | zaroxi_core_engine_style::PanelRole::MinimapLane
            | zaroxi_core_engine_style::PanelRole::BottomPanel => {
                super::editor::draw(r, &shell.theme, shell.work_content.as_ref())
            }
            zaroxi_core_engine_style::PanelRole::AuxiliaryPanelContent => {
                super::ai_pane::draw(r, &shell.theme, shell.work_content.as_ref())
            }
            zaroxi_core_engine_style::PanelRole::BottomDock => {
                super::bottom_panel::draw(r, &shell.theme)
            }
            zaroxi_core_engine_style::PanelRole::StatusBar => {
                super::status_bar::draw(r, &shell.theme)
            }
            _ => Vec::new(),
        };

        rects.append(&mut produced);
    }

    rects
}
