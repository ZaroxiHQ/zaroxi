/*!
AI assistant panel — phase-1 foundation.

Renders a real AI IDE panel with:
- Provider/account status indicator
- Model picker info
- Conversation area (messages with roles)
- Input composer
- Context chips
- Proper empty/error/not-connected states

Uses existing `UiBlock` primitives from the rendering pipeline.
*/

use crate::gui::ShellRegion;
use zaroxi_core_engine_render::{Rect, UiBlock};
use zaroxi_core_engine_style::StyleTokens;

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

/// Rich AI panel data for rendering.
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
}

pub struct AiPanel;

impl AiPanel {
    /// Build the assistant panel header area.
    ///
    /// Returns the header label block and, when provider status is available,
    /// a small status badge block positioned within the header region.
    pub fn build_header_block(
        r: &ShellRegion,
        tokens: &StyleTokens,
        data: &AiPanelData,
    ) -> Vec<UiBlock> {
        let mut blocks = Vec::new();

        let header_bg = tokens.assistant_panel_header_background.to_array();
        let text_color = tokens.text_primary.to_array();
        let _rx = r.rect.x as f32;
        let _ry = r.rect.y as f32;

        // Main header label
        blocks.push(UiBlock {
            id: r.id.to_string(),
            title: "AI Assistant".to_string(),
            rect: r.into(),
            header_color: Some(header_bg),
            header_only: true,
            text_color: Some(text_color),
            ..Default::default()
        });

        // Provider badge if status is known
        if let Some(status) = &data.provider_status {
            if let Some(badge) = Self::build_provider_badge(r, tokens, status) {
                blocks.push(badge);
            }
        }

        blocks
    }

    /// Build a small provider status badge block positioned next to the header.
    pub fn build_provider_badge(
        r: &ShellRegion,
        tokens: &StyleTokens,
        status: &ProviderUiStatus,
    ) -> Option<UiBlock> {
        let rx = r.rect.x as f32;
        let ry = r.rect.y as f32 + 4.0;
        let rw = r.rect.width as f32;

        let connected_label: Option<String> = match status {
            ProviderUiStatus::Connected { provider, model } => {
                Some(format!("{provider} \u{00b7} {model}"))
            }
            _ => None,
        };

        let (label, color, accent_color) = match status {
            ProviderUiStatus::NotConnected => {
                ("No provider", tokens.text_muted.to_array(), tokens.divider_subtle.to_array())
            }
            ProviderUiStatus::Connecting => (
                "Connecting\u{2026}",
                tokens.text_muted.to_array(),
                tokens.divider_subtle.to_array(),
            ),
            ProviderUiStatus::Connected { .. } => {
                let l = connected_label.as_deref().unwrap_or("Connected");
                (l, tokens.text_primary.to_array(), tokens.accent.to_array())
            }
            ProviderUiStatus::Error { .. } => {
                ("Connection error", tokens.status_error.to_array(), tokens.status_error.to_array())
            }
        };

        let badge_w = 160.0;
        let badge_h = 20.0;
        let x = (rx + rw - badge_w - 8.0).max(rx + 4.0);

        Some(UiBlock {
            id: format!("{}.provider_badge", r.id),
            title: label.to_string(),
            rect: Rect { x, y: ry, w: badge_w.min(rw - 8.0), h: badge_h },
            header_only: true,
            header_color: Some(accent_color),
            text_color: Some(color),
            corner_radius: 4.0,
            border_color: Some(accent_color),
            border_width: 1.0,
            ..Default::default()
        })
    }

    /// Build the AI content region: conversation messages, context chips,
    /// and a prompt composer.
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

        // ── Empty states ──
        if data.ai_content.is_none() && data.messages.is_empty() {
            // Show a useful empty state based on provider status.
            let empty_title: &str;
            let empty_body: String;
            match &data.provider_status {
                Some(ProviderUiStatus::NotConnected) | None => {
                    empty_title = "Set up an AI provider";
                    empty_body = "Configure an AI provider (OpenAI, Anthropic, etc.) in Settings to start using the assistant.".to_string();
                }
                Some(ProviderUiStatus::Connecting) => {
                    empty_title = "Connecting to provider\u{2026}";
                    empty_body =
                        "Validating connection. This should only take a moment.".to_string();
                }
                Some(ProviderUiStatus::Error { message }) => {
                    empty_title = "Connection failed";
                    empty_body = message.clone();
                }
                Some(ProviderUiStatus::Connected { provider, model }) => {
                    empty_title = "Ready";
                    empty_body = format!(
                        "Connected to {provider} ({model}). Ask a question or request an edit."
                    );
                }
            };

            let body_bg = tokens.assistant_panel_background.to_array();
            let text_primary = tokens.text_primary.to_array();
            let text_muted = tokens.text_muted.to_array();

            // Title block
            blocks.push(UiBlock {
                id: format!("{}.empty_title", r.id),
                title: empty_title.to_string(),
                rect: Rect { x: rx + 12.0, y: ry + 12.0, w: rw - 24.0, h: 24.0 },
                header_only: true,
                header_color: Some(body_bg),
                text_color: Some(text_primary),
                ..Default::default()
            });

            // Body text
            blocks.push(UiBlock {
                id: format!("{}.empty_body", r.id),
                title: empty_body.to_string(),
                content: empty_body.to_string(),
                rect: Rect { x: rx + 12.0, y: ry + 40.0, w: rw - 24.0, h: 48.0 },
                header_only: true,
                header_color: Some(body_bg),
                text_color: Some(text_muted),
                ..Default::default()
            });
        }

        // ── Conversation messages ──
        let mut msg_y = ry + 8.0;
        let body_bg = tokens.assistant_panel_background.to_array();
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

            // Role label block
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

            // Message content block (multi-line)
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

        // ── Context chip strip ──
        if !data.context_chips.is_empty() {
            let chip_y = (ry + rh - 72.0).max(msg_y + 8.0);
            let chip_h = 18.0;
            let mut chip_x = rx + 8.0;
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

        // ── Prompt composer (bottom-anchored) ──
        let pad = 12.0;
        let composer_h = 52.0;
        let send = 30.0;
        let composer_x = rx + pad;
        let composer_y = ry + rh - composer_h - pad;
        let composer_w = (rw - pad * 2.0).max(0.0);

        if composer_w > send + 40.0 && composer_y > ry + pad {
            // Composer container
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

            // Send affordance
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
