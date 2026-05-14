use log::info;
use crate::state::AppState;

/// A renderer-facing, minimal panel descriptor used to prove handoff.
///
/// This type intentionally mirrors App's PanelEntry but uses owned Strings
/// so it is easy to transport across crate boundaries.
#[derive(Debug, Clone)]
pub struct RenderPanel {
    pub id: String,
    pub title: String,
    pub content: String,
    pub visible: bool,
}

impl From<&crate::panels::panel_entry::PanelEntry> for RenderPanel {
    fn from(e: &crate::panels::panel_entry::PanelEntry) -> Self {
        Self {
            id: e.id.to_string(),
            title: e.title.clone(),
            content: e.content.clone(),
            visible: e.visible,
        }
    }
}

/// Convert AppState-owned panel entries into renderer-facing descriptors.
///
/// Logs the panel ids/titles during conversion for traceability.
pub fn to_render_panels(state: &AppState) -> Vec<RenderPanel> {
    let mut out = Vec::new();
    for p in &state.app_panels {
        info!("converting panel -> render_panel: id='{}' title='{}' visible={}", p.id, p.title, p.visible);
        out.push(RenderPanel::from(p));
    }
    info!("converted {} panels to render_panels", out.len());
    out
}
