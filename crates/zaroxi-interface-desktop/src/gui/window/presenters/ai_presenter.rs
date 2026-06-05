use zaroxi_core_engine_ui::ShellWorkContent;

use super::super::ai_pane::AiPanelData;

/// Shape AI assistant panel text from work_content into `AiPanelData`.
/// Extracts title, subtitle, and body lines from the ContentView.
pub fn shape_ai_content(work_content: &Option<ShellWorkContent>) -> AiPanelData {
    let wc = match work_content {
        Some(w) => w,
        None => return AiPanelData::default(),
    };

    let cv = wc.ai_panel_content.as_ref();
    let ai_content = cv.map(|cv| cv.lines.join("\n"));
    let ai_title = cv.map(|cv| cv.title.clone());
    let ai_subtitle = cv.map(|cv| cv.subtitle.clone());

    AiPanelData { ai_content, ai_title, ai_subtitle }
}
