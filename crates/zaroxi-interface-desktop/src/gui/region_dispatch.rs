//! Panel-role dispatch: maps ShellRegion string IDs to the app-neutral
//! `PanelRole` enum from the engine theme contract.
//!
//! Phase 41: Replaces string-based `match r.id { "toolbar" => ... }` patterns
//! with role-based dispatch. The mapping is intentionally centralized here so
//! all future refactors (e.g. storing `panel_role` directly in `ShellRegion`)
//! only need to change this one function.
//!
//! Design: each ShellRegion string `id` is mapped to the corresponding
//! `PanelRole` from `zaroxi-core-engine-style`. Calling code uses
//! `region_role(r.id)` instead of string-matching.

use zaroxi_core_engine_style::PanelRole;

/// Convert a legacy ShellRegion string ID to a PanelRole.
///
/// Unknown IDs produce `PanelRole::ContentArea` so callers can handle them
/// gracefully. This is the single source of truth for the string-to-role mapping.
pub fn region_role(id: &str) -> PanelRole {
    match id {
        "toolbar" => PanelRole::TopBar,
        "app_rail" => PanelRole::NavigationRail,
        "sidebar" => PanelRole::SidePanel,
        "editor_tabs" => PanelRole::ContentTabStrip,
        "breadcrumb" => PanelRole::ContentBreadcrumb,
        "editor_content" | "center_editor" => PanelRole::ContentArea,
        "minimap_lane" => PanelRole::MinimapLane,
        "center_bottom_panel" => PanelRole::BottomPanel,
        "bottom_dock" => PanelRole::BottomDock,
        "ai_panel_header" => PanelRole::AuxiliaryPanelHeader,
        "ai_panel_content" => PanelRole::AuxiliaryPanelContent,
        "status_bar" => PanelRole::StatusBar,
        _ => {
            log::warn!("unknown ShellRegion id '{}', falling back to ContentArea", id);
            PanelRole::ContentArea
        }
    }
}

/// Find a `ShellRegion` by its `PanelRole`. Returns the first region whose
/// string `id` maps to the requested role, or `None` if no such region exists.
///
/// Uses `region_role()` for the string-to-role conversion so the mapping
/// stays centralized in one place.
pub fn find_region_by_role<'a>(
    regions: &'a [crate::gui::ShellRegion],
    role: PanelRole,
) -> Option<&'a crate::gui::ShellRegion> {
    regions.iter().find(|r| region_role(r.id) == role)
}
