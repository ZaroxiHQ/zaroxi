use zaroxi_core_engine_ui::ShellWorkContent;
use zaroxi_core_engine_ui::chrome::PanelSection;

use super::super::rail::ExplorerData;

/// Shape explorer sidebar content from work_content into `ExplorerData`.
/// Uses engine-owned chrome formatters to produce per-span colored content
/// with section headers (muted) and indented items (secondary text).
pub fn shape_explorer_content(work_content: &Option<ShellWorkContent>) -> ExplorerData {
    let wc = match work_content {
        Some(w) => w,
        None => return ExplorerData::default(),
    };

    let sidebar_items = wc
        .explorer_items
        .clone()
        .filter(|items| !items.is_empty())
        .map(|items| {
            let section = PanelSection { header: "PROJECT".to_string(), items: items.clone() };
            let sections = vec![section];
            (sections, false)
        })
        .unwrap_or_else(|| (Vec::new(), true));

    let empty_button_label = wc.explorer_empty_button.clone();

    ExplorerData {
        sidebar_sections: sidebar_items.0.clone(),
        sidebar_empty: sidebar_items.1,
        empty_button_label,
    }
}
