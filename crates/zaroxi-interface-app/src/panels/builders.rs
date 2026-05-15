use crate::panels::panel_entry::PanelEntry;
use zaroxi_config::AppConfig;
use zaroxi_editor_buffer::Document;

/// Build the default set of application panels used at startup.
///
/// This function is intentionally simple — it returns a Vec<PanelEntry> that
/// the app initialization will attach to AppState.
pub fn default_panels(config: &AppConfig, welcome: &Document) -> Vec<PanelEntry> {
    let mut v = Vec::new();

    v.push(PanelEntry::new("titlebar", "Zaroxi Studio", config.title.clone(), true));
    v.push(PanelEntry::new("sidebar", "Explorer", "", true));
    v.push(PanelEntry::new("editor", "Editor", welcome.display_name.clone(), true));
    v.push(PanelEntry::new("right_panel", "Assistant", "", true));
    v.push(PanelEntry::new("bottom_panel", "Terminal", "", true));
    v.push(PanelEntry::new("status_bar", "Status", "", true));

    v
}
