/*!
AI assistant panel — shell wiring phase.

Renders the AI IDE panel frame:
- header with panel title
- provider/model status row (derived from real settings + backend state)
- session controls row (New chat / Clear)
- conversation area with role-labelled messages
- context strip (auto-attached context chips)
- bottom-anchored prompt composer with state-aware placeholder

All row geometry comes from `zaroxi_core_engine_ui::layout_constants` so the
painted blocks always align with the hit targets in the shell widget tree.
Uses existing `UiBlock` primitives from the rendering pipeline.
*/

use crate::gui::ShellRegion;
use zaroxi_core_engine_render::{Rect, UiBlock};
use zaroxi_core_engine_style::StyleTokens;
use zaroxi_core_engine_ui::layout_constants as lc;

/// Provider connection status for display.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProviderUiStatus {
    NotConnected,
    Connected { provider: String, model: String },
    Error { message: String },
    Connecting,
}

/// Message in the conversation display.
#[derive(Debug, Clone)]
pub struct ChatMessageUi {
    pub role: MessageRole,
    pub content: String,
    pub is_streaming: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[allow(dead_code)]
pub enum MessageRole {
    User,
    Assistant,
    System,
}

/// A context chip shown near the input.
#[derive(Debug, Clone)]
pub struct ContextChip {
    pub label: String,
    pub detail: String,
}

/// Normalized AI panel view state (shaped by the AI presenter).
#[derive(Default)]
pub struct AiPanelData {
    pub ai_content: Option<String>,
    #[allow(dead_code)]
    pub ai_title: Option<String>,
    pub ai_subtitle: Option<String>,
    /// Live provider connection status.
    pub provider_status: Option<ProviderUiStatus>,
    /// Recently displayed messages.
    pub messages: Vec<ChatMessageUi>,
    /// Active context chips.
    pub context_chips: Vec<ContextChip>,
    /// Whether the UI is waiting for a response.
    pub is_loading: bool,
    /// State-aware placeholder text for the composer.
    pub composer_placeholder: String,
    /// Show the "set up provider" CTA (no provider available).
    pub show_setup_cta: bool,
    /// Show the session controls row (New chat / Clear).
    pub show_session_controls: bool,
}

pub struct AiPanel;

impl AiPanel {
    /// Build the assistant panel header area (title only — provider status
    /// lives in the dedicated status row at the top of the content area).
    pub fn build_header_block(
        r: &ShellRegion,
        tokens: &StyleTokens,
        _data: &AiPanelData,
    ) -> Vec<UiBlock> {
        vec![UiBlock {
            id: r.id.to_string(),
            title: "AI Assistant".to_string(),
            rect: r.into(),
            header_color: Some(tokens.assistant_panel_header_background.to_array()),
            header_only: true,
            text_color: Some(tokens.text_primary.to_array()),
            ..Default::default()
        }]
    }

    /// Build the provider/model status row shown at the top of the content
    /// area. Always present so the panel state is clear at a glance.
    fn build_status_row(r: &ShellRegion, tokens: &StyleTokens, data: &AiPanelData) -> UiBlock {
        let content = (r.rect.x as f32, r.rect.y as f32, r.rect.width as f32, r.rect.height as f32);
        let (sx, sy, sw, sh) = lc::ai_status_row_rect(content);

        let (label, text_color, accent) = match &data.provider_status {
            Some(ProviderUiStatus::Connected { provider, model }) => {
                let label = if model.is_empty() {
                    format!("\u{25CF} {provider}")
                } else {
                    format!("\u{25CF} {provider} \u{00b7} {model}")
                };
                (label, tokens.text_primary.to_array(), tokens.accent.to_array())
            }
            Some(ProviderUiStatus::Connecting) => (
                "\u{25CB} Connecting\u{2026}".to_string(),
                tokens.text_muted.to_array(),
                tokens.divider_subtle.to_array(),
            ),
            Some(ProviderUiStatus::Error { .. }) => (
                "\u{25CF} Connection error".to_string(),
                tokens.status_error.to_array(),
                tokens.status_error.to_array(),
            ),
            Some(ProviderUiStatus::NotConnected) | None => (
                "\u{25CB} No provider configured".to_string(),
                tokens.text_muted.to_array(),
                tokens.divider_subtle.to_array(),
            ),
        };

        UiBlock {
            id: format!("{}.status_row", r.id),
            title: label,
            rect: Rect { x: sx, y: sy, w: sw, h: sh },
            header_only: true,
            header_color: Some(tokens.assistant_panel_background.to_array()),
            text_color: Some(text_color),
            border_color: Some(accent),
            border_width: 0.0,
            ..Default::default()
        }
    }

    /// Build the session controls row (New chat / Clear) aligned with the
    /// widget-tree hit targets.
    fn build_session_controls(r: &ShellRegion, tokens: &StyleTokens) -> Vec<UiBlock> {
        let content = (r.rect.x as f32, r.rect.y as f32, r.rect.width as f32, r.rect.height as f32);
        let (rx, ry, _rw, rh) = lc::ai_controls_row_rect(content);
        ["New chat", "Clear"]
            .iter()
            .enumerate()
            .map(|(i, label)| UiBlock {
                id: format!("{}.session_{}", r.id, label.to_lowercase().replace(' ', "_")),
                title: label.to_string(),
                rect: Rect {
                    x: rx + i as f32 * (lc::AI_SESSION_BTN_W + 8.0),
                    y: ry,
                    w: lc::AI_SESSION_BTN_W,
                    h: rh,
                },
                header_only: true,
                header_color: Some(tokens.sidebar_input.to_array()),
                text_color: Some(tokens.text_secondary.to_array()),
                corner_radius: 4.0,
                border_color: Some(tokens.divider_subtle.to_array()),
                border_width: 1.0,
                ..Default::default()
            })
            .collect()
    }

    /// Build the AI content region: status row, session controls,
    /// conversation messages, context chips, and the prompt composer.
    pub fn build_content_block(
        r: &ShellRegion,
        tokens: &StyleTokens,
        data: &AiPanelData,
    ) -> Vec<UiBlock> {
        let mut blocks = Vec::new();

        let rx = r.rect.x as f32;
        let ry = r.rect.y as f32;
        let rw = r.rect.width as f32;
        let rh = r.rect.height as f32;
        let content = (rx, ry, rw, rh);
        let body_bg = tokens.assistant_panel_background.to_array();

        // ── Provider/model status row ──
        blocks.push(Self::build_status_row(r, tokens, data));

        // ── Session controls row ──
        if data.show_session_controls {
            blocks.extend(Self::build_session_controls(r, tokens));
        }

        let (_, controls_y, _, controls_h) = lc::ai_controls_row_rect(content);
        let body_top = controls_y + controls_h + lc::AI_ROW_GAP * 2.0;

        // ── Empty states ──
        if data.ai_content.is_none() && data.messages.is_empty() {
            let empty_title: &str;
            let empty_body: String;
            match &data.provider_status {
                Some(ProviderUiStatus::NotConnected) | None => {
                    empty_title = "Set up an AI provider";
                    empty_body =
                        "Connect a provider in Settings to chat, explain code, and request edits."
                            .to_string();
                }
                Some(ProviderUiStatus::Connecting) => {
                    empty_title = "Connecting to provider\u{2026}";
                    empty_body =
                        "Validating connection. This should only take a moment.".to_string();
                }
                Some(ProviderUiStatus::Error { message }) => {
                    empty_title = "Assistant unavailable";
                    empty_body = message.clone();
                }
                Some(ProviderUiStatus::Connected { provider, .. }) => {
                    empty_title = "Start a conversation";
                    empty_body = format!(
                        "{provider} is ready. Ask a question or request an edit to the active file."
                    );
                }
            };

            blocks.push(UiBlock {
                id: format!("{}.empty_title", r.id),
                title: empty_title.to_string(),
                rect: Rect {
                    x: rx + lc::AI_PANEL_PAD,
                    y: body_top,
                    w: rw - lc::AI_PANEL_PAD * 2.0,
                    h: 24.0,
                },
                header_only: true,
                header_color: Some(body_bg),
                text_color: Some(tokens.text_primary.to_array()),
                ..Default::default()
            });

            blocks.push(UiBlock {
                id: format!("{}.empty_body", r.id),
                title: empty_body.clone(),
                content: empty_body,
                rect: Rect {
                    x: rx + lc::AI_PANEL_PAD,
                    y: body_top + 28.0,
                    w: rw - lc::AI_PANEL_PAD * 2.0,
                    h: 48.0,
                },
                header_only: true,
                header_color: Some(body_bg),
                text_color: Some(tokens.text_muted.to_array()),
                ..Default::default()
            });

            // Primary CTA: jump to Settings when no provider is available.
            if data.show_setup_cta {
                let (cx, cy, cw, ch) = lc::ai_setup_cta_rect(content);
                blocks.push(UiBlock {
                    id: format!("{}.setup_cta", r.id),
                    title: "Open Settings".to_string(),
                    rect: Rect { x: cx, y: cy, w: cw, h: ch },
                    header_only: true,
                    header_color: Some(tokens.accent.to_array()),
                    text_color: Some(tokens.text_primary.to_array()),
                    corner_radius: 4.0,
                    ..Default::default()
                });
            }
        }

        // ── Conversation messages ──
        let mut msg_y = body_top;
        for msg in &data.messages {
            let role_label = match msg.role {
                MessageRole::User => "You",
                MessageRole::Assistant => "Assistant",
                MessageRole::System => "System",
            };
            let content = if msg.content.len() > 200 {
                format!("{}...", &msg.content[..200])
            } else {
                msg.content.clone()
            };
            let role_color = match msg.role {
                MessageRole::User => tokens.accent.to_array(),
                MessageRole::Assistant => tokens.text_primary.to_array(),
                MessageRole::System => tokens.text_muted.to_array(),
            };

            let streaming_indicator = if msg.is_streaming { " \u{25CF}" } else { "" };
            let label = format!("{role_label}{streaming_indicator}");

            blocks.push(UiBlock {
                id: format!("{}.msg_role_{}", r.id, msg_y as u32),
                title: label,
                rect: Rect { x: rx + 8.0, y: msg_y, w: rw - 16.0, h: 16.0 },
                header_only: true,
                header_color: Some(body_bg),
                text_color: Some(role_color),
                corner_radius: 0.0,
                ..Default::default()
            });
            msg_y += 18.0;

            let content_lines = content.lines().count() as f32;
            let content_h = (content_lines * 16.0).max(16.0);
            blocks.push(UiBlock {
                id: format!("{}.msg_body_{}", r.id, msg_y as u32),
                title: content.clone(),
                content: content.clone(),
                rect: Rect { x: rx + 8.0, y: msg_y, w: rw - 16.0, h: content_h },
                header_color: Some(body_bg),
                content_color: Some(body_bg),
                text_color: Some(tokens.text_secondary.to_array()),
                corner_radius: 0.0,
                ..Default::default()
            });
            msg_y += content_h + 12.0;
        }

        // ── Loading indicator ──
        if data.is_loading {
            blocks.push(UiBlock {
                id: format!("{}.loading", r.id),
                title: "Thinking\u{2026}".to_string(),
                rect: Rect { x: rx + 8.0, y: msg_y, w: rw - 16.0, h: 20.0 },
                header_only: true,
                header_color: Some(body_bg),
                text_color: Some(tokens.text_muted.to_array()),
                corner_radius: 0.0,
                ..Default::default()
            });
        }

        // ── Context chip strip (above the composer) ──
        let (composer_x, composer_y, composer_w, composer_h) = lc::ai_composer_rect(content);
        if !data.context_chips.is_empty() {
            let chip_h = 18.0;
            let chip_y = (composer_y - chip_h - lc::AI_ROW_GAP).max(msg_y + 8.0);
            let mut chip_x = rx + lc::AI_PANEL_PAD;
            let chip_gap = 6.0;

            for chip in &data.context_chips {
                let chip_label = format!("{}: {}", chip.label, chip.detail);
                let chip_w = (chip_label.len() as f32 * 7.0 + 16.0).min(rw - 16.0);

                blocks.push(UiBlock {
                    id: format!("{}.chip_{}", r.id, chip.label),
                    title: chip_label,
                    rect: Rect { x: chip_x, y: chip_y, w: chip_w, h: chip_h },
                    header_only: true,
                    header_color: Some(tokens.sidebar_input.to_array()),
                    text_color: Some(tokens.text_muted.to_array()),
                    corner_radius: 3.0,
                    border_color: Some(tokens.divider_subtle.to_array()),
                    border_width: 1.0,
                    ..Default::default()
                });
                chip_x += chip_w + chip_gap;
            }
        }

        // ── Prompt composer (bottom-anchored, shared geometry) ──
        let send = lc::AI_SEND_SIZE;
        if composer_w > send + 40.0 && composer_y > ry + lc::AI_PANEL_PAD {
            let composer_ready =
                matches!(data.provider_status, Some(ProviderUiStatus::Connected { .. }))
                    && !data.is_loading;

            blocks.push(UiBlock {
                id: format!("{}.composer", r.id),
                title: data.composer_placeholder.clone(),
                rect: Rect { x: composer_x, y: composer_y, w: composer_w, h: composer_h },
                header_only: true,
                header_color: Some(tokens.sidebar_input.to_array()),
                border_color: Some(tokens.border_strong.to_array()),
                border_width: 1.0,
                corner_radius: 8.0,
                text_color: Some(tokens.text_muted.to_array()),
                ..Default::default()
            });

            // Send affordance — dimmed when the composer cannot send.
            let send_color = if composer_ready {
                tokens.accent.to_array()
            } else {
                tokens.divider_subtle.to_array()
            };
            blocks.push(UiBlock {
                id: format!("{}.composer_send", r.id),
                rect: Rect {
                    x: composer_x + composer_w - send - 8.0,
                    y: composer_y + (composer_h - send) / 2.0,
                    w: send,
                    h: send,
                },
                header_only: true,
                header_color: Some(send_color),
                corner_radius: 6.0,
                ..Default::default()
            });
        }

        blocks
    }
}
