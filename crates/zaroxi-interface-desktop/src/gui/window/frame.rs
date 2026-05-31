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
/// This function delegates region-specific drawing to per-panel modules.
pub fn build_overlay_rects(
    shell: &crate::gui::ShellFrame,
) -> Vec<zaroxi_core_engine_render_backend::DrawRect> {
    let mut rects: Vec<zaroxi_core_engine_render_backend::DrawRect> = Vec::new();

    for r in &shell.regions {
        let mut produced: Vec<zaroxi_core_engine_render_backend::DrawRect> = match r.id {
            "toolbar" => super::toolbar::draw(r, &shell.theme),
            "app_rail" | "sidebar" => super::rail::draw(r, &shell.theme),
            "editor_tabs"
            | "breadcrumb"
            | "center_editor"
            | "minimap_lane"
            | "center_bottom_panel" => super::editor::draw(r, &shell.theme),
            "ai_panel_content" => super::ai_pane::draw(r, &shell.theme),
            "bottom_dock" => super::bottom_panel::draw(r, &shell.theme),
            "status_bar" => super::status_bar::draw(r, &shell.theme),
            _ => Vec::new(),
        };

        rects.append(&mut produced);
    }

    rects
}
