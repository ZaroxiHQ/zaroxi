use zaroxi_core_engine_ui::ShellWorkContent;

use super::super::ai_pane::AiPanelData;

/// Shape AI assistant panel text from work_content into `AiPanelData`.
pub fn shape_ai_content(work_content: &Option<ShellWorkContent>) -> AiPanelData {
    let wc = match work_content {
        Some(w) => w,
        None => return AiPanelData::default(),
    };

    let ai_text = wc.ai_panel_content.as_ref().map(|cv| cv.lines.join("\n"));

    AiPanelData { ai_content: ai_text }
}
