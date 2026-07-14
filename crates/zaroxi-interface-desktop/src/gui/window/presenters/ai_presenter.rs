use zaroxi_core_engine_ui::ShellWorkContent;

use super::super::ai_pane::{AiPanelData, ChatMessageUi, ContextChip, ProviderUiStatus};

/// Shape AI assistant panel data from work_content and live app state.
///
/// Extracts title, subtitle, and body lines from the ContentView,
/// and enriches with provider connection status, conversation messages,
/// and context chips provided by the app layer.
pub fn shape_ai_content(
    work_content: &Option<ShellWorkContent>,
    provider_status: Option<ProviderUiStatus>,
    messages: Vec<ChatMessageUi>,
    context_chips: Vec<ContextChip>,
    is_loading: bool,
) -> AiPanelData {
    let (ai_content, ai_title, ai_subtitle) = match work_content {
        Some(w) => {
            let cv = w.ai_panel_content.as_ref();
            (
                cv.map(|cv| cv.lines.join("\n")),
                cv.map(|cv| cv.title.clone()),
                cv.map(|cv| cv.subtitle.clone()),
            )
        }
        None => (None, None, None),
    };

    AiPanelData {
        ai_content,
        ai_title,
        ai_subtitle,
        provider_status,
        messages,
        context_chips,
        is_loading,
    }
}
