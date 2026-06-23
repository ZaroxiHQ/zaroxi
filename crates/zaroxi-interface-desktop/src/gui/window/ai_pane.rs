/*!
AI assistant panel.

Phase 61: fixed header_only to render body text from ai_panel_content.
Phase 73: uses chrome formatters for structured header/body/empty-state.
*/

use crate::gui::ShellRegion;
use zaroxi_core_engine_render::UiBlock;
use zaroxi_core_engine_style::StyleTokens;

pub struct AiPanelData {
    pub ai_content: Option<String>,
    pub ai_title: Option<String>,
    pub ai_subtitle: Option<String>,
}

impl Default for AiPanelData {
    fn default() -> Self {
        Self { ai_content: None, ai_title: None, ai_subtitle: None }
    }
}

pub struct AiPanel;

impl AiPanel {
    pub fn build_header_block(r: &ShellRegion, tokens: &StyleTokens) -> UiBlock {
        UiBlock {
            id: r.id.to_string(),
            title: "AI Assistant".to_string(),
            rect: r.into(),
            header_color: Some(tokens.assistant_panel_header_background.to_array()),
            header_only: true,
            text_color: Some(tokens.text_primary.to_array()),
            ..Default::default()
        }
    }

    pub fn build_content_block(
        r: &ShellRegion,
        tokens: &StyleTokens,
        data: &AiPanelData,
    ) -> UiBlock {
        let ai_title_opt = data.ai_title.as_deref();
        let ai_subtitle_opt = data.ai_subtitle.as_deref();
        let ai_body_opt = data.ai_content.as_deref();

        let spans = zaroxi_core_engine_ui::chrome::format_ai_panel_spans(
            ai_title_opt,
            ai_subtitle_opt,
            ai_body_opt,
            tokens,
        );
        let content = spans.iter().map(|(t, _)| t.clone()).collect::<Vec<_>>().join("");

        let title = data
            .ai_title
            .clone()
            .or_else(|| data.ai_subtitle.clone())
            .unwrap_or_else(|| "Assistant".to_string());

        UiBlock {
            id: r.id.to_string(),
            title,
            content,
            rect: r.into(),
            header_color: Some(tokens.assistant_panel_background.to_array()),
            content_color: Some(tokens.assistant_panel_background.to_array()),
            content_spans: Some(spans),
            ..Default::default()
        }
    }
}
