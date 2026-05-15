use crate::panels::panel_entry::PanelEntry;
use crate::panels::panel_id::PanelId;

/// Very small registry helpers for panels.
///
/// This module is intentionally tiny for v1. It provides a way to look up
/// panel entries by their stable identifier.
pub struct PanelRegistry;

impl PanelRegistry {
    /// Find a panel entry by id string.
    pub fn find_by_id<'a>(panels: &'a [PanelEntry], id: PanelId) -> Option<&'a PanelEntry> {
        let s = id.as_str();
        panels.iter().find(|p| p.id == s)
    }

    /// Find a mutable panel entry by id string.
    pub fn find_by_id_mut<'a>(panels: &'a mut [PanelEntry], id: PanelId) -> Option<&'a mut PanelEntry> {
        let s = id.as_str();
        panels.iter_mut().find(|p| p.id == s)
    }
}
