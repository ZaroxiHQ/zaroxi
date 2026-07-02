/*!
AI assistant panel.

Phase 61: fixed header_only to render body text from ai_panel_content.
Phase 73: uses chrome formatters for structured header/body/empty-state.
*/

use crate::gui::ShellRegion;
use zaroxi_core_engine_render::{Rect, UiBlock};
use zaroxi_core_engine_style::StyleTokens;

#[derive(Default)]
pub struct AiPanelData {
    pub ai_content: Option<String>,
    pub ai_title: Option<String>,
    pub ai_subtitle: Option<String>,
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

    /// Build the AI content region: the body module plus a bottom-anchored
    /// prompt composer (a real bordered input container with a send affordance).
    ///
    /// The composer is a sticky footer — it is emitted after the body so it
    /// paints on top, giving the panel visible product geometry instead of a
    /// plain text column. Uses only existing tokens, so both themes match.
    pub fn build_content_block(
        r: &ShellRegion,
        tokens: &StyleTokens,
        data: &AiPanelData,
    ) -> Vec<UiBlock> {
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

        // Avoid repeating the panel's "AI Assistant" header label in the body.
        let title = data.ai_title.clone().or_else(|| data.ai_subtitle.clone()).unwrap_or_default();

        let mut blocks = vec![UiBlock {
            id: r.id.to_string(),
            title,
            content,
            rect: r.into(),
            header_color: Some(tokens.assistant_panel_background.to_array()),
            content_color: Some(tokens.assistant_panel_background.to_array()),
            content_spans: Some(spans),
            ..Default::default()
        }];

        // ── Prompt composer (bottom-anchored container + send button) ──
        let rx = r.rect.x as f32;
        let ry = r.rect.y as f32;
        let rw = r.rect.width as f32;
        let rh = r.rect.height as f32;

        let pad = 12.0;
        let composer_h = 52.0;
        let send = 30.0;
        let composer_x = rx + pad;
        let composer_y = ry + rh - composer_h - pad;
        let composer_w = (rw - pad * 2.0).max(0.0);

        // Only draw when the panel is tall/wide enough to host a real composer.
        if composer_w > send + 40.0 && composer_y > ry + pad {
            // Composer container — inset field surface with a strong 1px edge.
            blocks.push(UiBlock {
                id: format!("{}.composer", r.id),
                title: "Ask Zaroxi\u{2026}".to_string(),
                rect: Rect { x: composer_x, y: composer_y, w: composer_w, h: composer_h },
                header_only: true,
                header_color: Some(tokens.sidebar_input.to_array()),
                border_color: Some(tokens.border_strong.to_array()),
                border_width: 1.0,
                corner_radius: 8.0,
                text_color: Some(tokens.text_muted.to_array()),
                ..Default::default()
            });

            // Send affordance — accent square, right-aligned inside the composer.
            blocks.push(UiBlock {
                id: format!("{}.composer_send", r.id),
                rect: Rect {
                    x: composer_x + composer_w - send - 8.0,
                    y: composer_y + (composer_h - send) / 2.0,
                    w: send,
                    h: send,
                },
                header_only: true,
                header_color: Some(tokens.accent.to_array()),
                corner_radius: 6.0,
                ..Default::default()
            });
        }

        blocks
    }
}
