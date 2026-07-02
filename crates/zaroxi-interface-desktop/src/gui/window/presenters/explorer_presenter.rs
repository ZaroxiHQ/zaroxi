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

    let panel_items = wc.explorer_panel_items.clone();
    let has_structured_items = panel_items.as_ref().is_some_and(|v| !v.is_empty());

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

    let sidebar_empty = if has_structured_items { false } else { sidebar_items.1 };

    ExplorerData {
        sidebar_sections: sidebar_items.0.clone(),
        sidebar_empty,
        empty_button_label,
        panel_items,
        scroll_top: wc.explorer_scroll_top,
        search_query: wc.explorer_search_query.clone(),
        search_active: wc.explorer_search_active,
        has_workspace: wc.explorer_has_workspace,
        // Advance, keyboard selection, and blink phase are overridden by the
        // GuiApp render path (which has renderer + UI state); these are defaults.
        char_advance: 8.0,
        selected_row: None,
        search_caret_visible: wc.explorer_search_active,
    }
}
