use zaroxi_core_engine_ui::ShellWorkContent;

use super::super::rail::ExplorerData;

/// Shape explorer sidebar text from work_content into `ExplorerData`.
pub fn shape_explorer_content(work_content: &Option<ShellWorkContent>) -> ExplorerData {
    let wc = match work_content {
        Some(w) => w,
        None => return ExplorerData::default(),
    };

    let sidebar_items = wc
        .explorer_items
        .clone()
        .map(|items| {
            let mut text = String::from("EXPLORER\n");
            for item in &items {
                text.push_str(&format!("  {}\n", item));
            }
            text
        })
        .unwrap_or_else(|| "No workspace loaded".to_string());

    ExplorerData { sidebar_items }
}
