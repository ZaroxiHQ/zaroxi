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

/// Approval banner state for a pending edit proposal.
///
/// Only present while a proposal awaits explicit review — edits are never
/// applied without the user pressing Apply.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ApprovalBannerUi {
    /// Target buffer the proposal would modify.
    pub target: String,
    /// Compact diff summary (e.g. `"12 → 15 lines · 1 changed region"`).
    pub summary: String,
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
    /// Current text typed into the composer (empty shows the placeholder).
    pub composer_text: String,
    /// Whether the composer holds keyboard focus (shows the caret).
    pub composer_focused: bool,
    /// Show the "set up provider" CTA (no provider available).
    pub show_setup_cta: bool,
    /// Show the session controls row (New chat / Clear).
    pub show_session_controls: bool,
    /// Show the quick-action buttons (Explain / Refactor / Tests / Fix).
    pub show_quick_actions: bool,
    /// Pending edit proposal awaiting explicit review.
    pub approval: Option<ApprovalBannerUi>,
    /// Compact MCP status line (only when MCP is enabled in settings).
    pub mcp_status: Option<String>,
    /// Truthful operational activity line (request phase / last-run stats),
    /// shown in the compact status footer.
    pub activity_line: Option<String>,
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

        // Append the compact MCP summary when MCP is enabled — one status
        // row, no duplicated surfaces.
        let label = match &data.mcp_status {
            Some(mcp) => format!("{label} \u{00b7} {mcp}"),
            None => label,
        };
        // Long provider/model names must not overflow narrow panels.
        let label = truncate_to_cols(&label, ((sw / 7.0) as usize).max(8));

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
        let (rx, ry, rw, rh) = lc::ai_controls_row_rect(content);
        ["New chat", "Clear"]
            .iter()
            .enumerate()
            .filter(|(i, _)| {
                let bx = rx + *i as f32 * (lc::AI_SESSION_BTN_W + 8.0);
                bx + lc::AI_SESSION_BTN_W <= rx + rw
            })
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
        let mut body_top = controls_y + controls_h + lc::AI_ROW_GAP * 2.0;

        // ── Actions row: approval takes priority over quick actions ──
        let (actions_x, actions_y, actions_w, actions_h) = lc::ai_actions_row_rect(content);
        if data.show_session_controls && data.approval.is_some() {
            // Apply / Reject buttons aligned with the widget-tree hit targets.
            blocks.push(UiBlock {
                id: format!("{}.approval_apply", r.id),
                title: "Apply".to_string(),
                rect: Rect { x: actions_x, y: actions_y, w: lc::AI_APPROVAL_BTN_W, h: actions_h },
                header_only: true,
                header_color: Some(tokens.accent.to_array()),
                text_color: Some(tokens.text_primary.to_array()),
                corner_radius: 4.0,
                ..Default::default()
            });
            blocks.push(UiBlock {
                id: format!("{}.approval_reject", r.id),
                title: "Reject".to_string(),
                rect: Rect {
                    x: actions_x + lc::AI_APPROVAL_BTN_W + 8.0,
                    y: actions_y,
                    w: lc::AI_APPROVAL_BTN_W,
                    h: actions_h,
                },
                header_only: true,
                header_color: Some(tokens.sidebar_input.to_array()),
                text_color: Some(tokens.text_secondary.to_array()),
                corner_radius: 4.0,
                border_color: Some(tokens.divider_subtle.to_array()),
                border_width: 1.0,
                ..Default::default()
            });
            body_top = actions_y + actions_h + lc::AI_ROW_GAP;

            // Approval banner: explicit review state, target, diff summary.
            if let Some(approval) = &data.approval {
                let banner_h = 40.0;
                blocks.push(UiBlock {
                    id: format!("{}.approval_banner", r.id),
                    title: format!("Review proposed edit \u{2192} {}", approval.target),
                    content: approval.summary.clone(),
                    rect: Rect {
                        x: rx + lc::AI_PANEL_PAD,
                        y: body_top,
                        w: rw - lc::AI_PANEL_PAD * 2.0,
                        h: banner_h,
                    },
                    header_color: Some(body_bg),
                    content_color: Some(body_bg),
                    text_color: Some(tokens.text_primary.to_array()),
                    border_color: Some(tokens.accent.to_array()),
                    border_width: 1.0,
                    corner_radius: 4.0,
                    ..Default::default()
                });
                body_top += banner_h + lc::AI_ROW_GAP;
            }
        } else if data.show_session_controls && data.show_quick_actions {
            let mut bx = actions_x;
            let actions_right = actions_x + actions_w;
            for label in ["Explain", "Refactor", "Tests", "Fix"] {
                if bx + lc::AI_QUICK_BTN_W > actions_right {
                    break;
                }
                blocks.push(UiBlock {
                    id: format!("{}.quick_{}", r.id, label.to_lowercase()),
                    title: label.to_string(),
                    rect: Rect { x: bx, y: actions_y, w: lc::AI_QUICK_BTN_W, h: actions_h },
                    header_only: true,
                    header_color: Some(tokens.sidebar_input.to_array()),
                    text_color: Some(tokens.text_secondary.to_array()),
                    corner_radius: 4.0,
                    border_color: Some(tokens.divider_subtle.to_array()),
                    border_width: 1.0,
                    ..Default::default()
                });
                bx += lc::AI_QUICK_BTN_W + lc::AI_QUICK_BTN_GAP;
            }
            body_top = actions_y + actions_h + lc::AI_ROW_GAP;
        }

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
        // Approximate monospace character budget for wrapping message text.
        let wrap_cols = (((rw - 32.0) / 7.0) as usize).max(16);

        // ── Result / proposal preview card ──
        // Shows the projection content (proposal preview, analysis output,
        // diagnostics summary) when it is not already covered by the chat:
        // always during proposal review, otherwise only without messages.
        if let Some(ai_content) = &data.ai_content
            && (data.approval.is_some() || data.messages.is_empty())
            && !ai_content.trim().is_empty()
        {
            const MAX_PREVIEW_LINES: usize = 10;
            let wrapped = wrap_message(ai_content, wrap_cols);
            let all_lines: Vec<&str> = wrapped.lines().collect();
            let shown = all_lines.len().min(MAX_PREVIEW_LINES);
            let mut preview = all_lines[..shown].join("\n");
            if all_lines.len() > shown {
                preview.push_str(&format!("\n\u{2026} (+{} more lines)", all_lines.len() - shown));
            }
            let preview_lines = preview.lines().count() as f32;
            let card_h = preview_lines * 16.0 + 24.0;
            let card_title = data.ai_title.clone().unwrap_or_else(|| "Assistant".to_string());

            blocks.push(UiBlock {
                id: format!("{}.result_card", r.id),
                title: card_title,
                content: preview,
                rect: Rect {
                    x: rx + lc::AI_PANEL_PAD,
                    y: msg_y,
                    w: rw - lc::AI_PANEL_PAD * 2.0,
                    h: card_h,
                },
                header_color: Some(tokens.sidebar_input.to_array()),
                content_color: Some(tokens.sidebar_input.to_array()),
                text_color: Some(tokens.text_secondary.to_array()),
                border_color: Some(tokens.divider_subtle.to_array()),
                border_width: 1.0,
                corner_radius: 4.0,
                ..Default::default()
            });
            msg_y += card_h + lc::AI_ROW_GAP * 2.0;
        }

        // ── Conversation messages (tail-anchored, like a real chat) ──
        // Precompute per-message display text + heights, then render the
        // most recent messages that fit between the body top and the footer
        // reservations (activity line, context strip, composer).
        let (composer_x, composer_y, composer_w, composer_h) = lc::ai_composer_rect(content);
        let chip_h = 18.0;
        let mut footer_reserved = 0.0;
        if !data.context_chips.is_empty() {
            footer_reserved += chip_h + lc::AI_ROW_GAP;
        }
        if data.activity_line.is_some() {
            footer_reserved += 16.0 + lc::AI_ROW_GAP;
        }
        let messages_bottom = composer_y - lc::AI_ROW_GAP - footer_reserved;

        struct MsgBlock {
            label: String,
            display: String,
            role_color: [f32; 4],
            height: f32,
        }
        let prepared: Vec<MsgBlock> = data
            .messages
            .iter()
            .map(|msg| {
                let role_label = match msg.role {
                    MessageRole::User => "You",
                    MessageRole::Assistant => "Assistant",
                    MessageRole::System => "System",
                };
                let content = wrap_message(&msg.content, wrap_cols);
                let role_color = match msg.role {
                    MessageRole::User => tokens.accent.to_array(),
                    MessageRole::Assistant => tokens.text_primary.to_array(),
                    MessageRole::System => tokens.status_error.to_array(),
                };
                let streaming_indicator = if msg.is_streaming { " \u{25CF}" } else { "" };
                let label = format!("{role_label}{streaming_indicator}");
                let display = if content.is_empty() && msg.is_streaming {
                    "\u{2026}".to_string()
                } else {
                    content
                };
                let content_h = (display.lines().count() as f32 * 16.0).max(16.0);
                MsgBlock { label, display, role_color, height: 18.0 + content_h + 12.0 }
            })
            .collect();

        let heights: Vec<f32> = prepared.iter().map(|m| m.height).collect();
        let (start_idx, hidden) =
            visible_message_window(&heights, (messages_bottom - msg_y).max(0.0));

        if hidden > 0 {
            let noun = if hidden == 1 { "earlier message" } else { "earlier messages" };
            blocks.push(UiBlock {
                id: format!("{}.hidden_msgs", r.id),
                title: format!("\u{2026} {hidden} {noun} hidden"),
                rect: Rect { x: rx + 8.0, y: msg_y, w: rw - 16.0, h: 14.0 },
                header_only: true,
                header_color: Some(body_bg),
                text_color: Some(tokens.text_faint.to_array()),
                corner_radius: 0.0,
                ..Default::default()
            });
            msg_y += 16.0;
        }

        for msg in prepared.iter().skip(start_idx) {
            blocks.push(UiBlock {
                id: format!("{}.msg_role_{}", r.id, msg_y as u32),
                title: msg.label.clone(),
                rect: Rect { x: rx + 8.0, y: msg_y, w: rw - 16.0, h: 16.0 },
                header_only: true,
                header_color: Some(body_bg),
                text_color: Some(msg.role_color),
                corner_radius: 0.0,
                ..Default::default()
            });
            msg_y += 18.0;

            let content_h = msg.height - 18.0 - 12.0;
            blocks.push(UiBlock {
                id: format!("{}.msg_body_{}", r.id, msg_y as u32),
                title: msg.display.clone(),
                content: msg.display.clone(),
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
        // Only shown when no streaming message already communicates progress.
        let has_streaming_message = data.messages.iter().any(|m| m.is_streaming);
        if data.is_loading && !has_streaming_message {
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

        // ── Context chip strip (fixed position above the composer) ──
        let chips_y = composer_y - chip_h - lc::AI_ROW_GAP;
        if !data.context_chips.is_empty() && chips_y > msg_y {
            let chip_y = chips_y;
            let mut chip_x = rx + lc::AI_PANEL_PAD;
            let chip_gap = 6.0;
            let strip_right = rx + rw - lc::AI_PANEL_PAD;
            let mut shown = 0usize;

            for chip in &data.context_chips {
                let chip_label = format!("{}: {}", chip.label, chip.detail);
                let chip_w = (chip_label.len() as f32 * 7.0 + 16.0).min(rw - 16.0);
                // Reserve room for a possible "+N" overflow marker.
                if chip_x + chip_w > strip_right - 36.0 && shown + 1 < data.context_chips.len() {
                    break;
                }
                if chip_x + chip_w > strip_right {
                    break;
                }

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
                shown += 1;
            }

            // Overflow marker: context is still attached, just not all shown.
            let overflow = data.context_chips.len() - shown;
            if overflow > 0 && chip_x + 32.0 <= strip_right {
                blocks.push(UiBlock {
                    id: format!("{}.chip_overflow", r.id),
                    title: format!("+{overflow}"),
                    rect: Rect { x: chip_x, y: chip_y, w: 32.0, h: chip_h },
                    header_only: true,
                    header_color: Some(tokens.sidebar_input.to_array()),
                    text_color: Some(tokens.text_muted.to_array()),
                    corner_radius: 3.0,
                    border_color: Some(tokens.divider_subtle.to_array()),
                    border_width: 1.0,
                    ..Default::default()
                });
            }
        }

        // ── Compact activity footer (above the context strip) ──
        // Truthful operational state only: request phase or last-run stats.
        if let Some(activity) = &data.activity_line {
            let mut activity_y = chips_y - 16.0 - lc::AI_ROW_GAP;
            if data.context_chips.is_empty() {
                activity_y = composer_y - 16.0 - lc::AI_ROW_GAP;
            }
            if activity_y > msg_y {
                blocks.push(UiBlock {
                    id: format!("{}.activity", r.id),
                    title: activity.clone(),
                    rect: Rect {
                        x: rx + lc::AI_PANEL_PAD,
                        y: activity_y,
                        w: rw - lc::AI_PANEL_PAD * 2.0,
                        h: 16.0,
                    },
                    header_only: true,
                    header_color: Some(body_bg),
                    text_color: Some(tokens.text_faint.to_array()),
                    corner_radius: 0.0,
                    ..Default::default()
                });
            }
        }

        // ── Prompt composer (bottom-anchored, shared geometry) ──
        let send = lc::AI_SEND_SIZE;
        if composer_w > send + 40.0 && composer_y > ry + lc::AI_PANEL_PAD {
            let composer_ready =
                matches!(data.provider_status, Some(ProviderUiStatus::Connected { .. }))
                    && !data.is_loading;

            // Typed text takes priority over the placeholder; a caret marker
            // communicates keyboard focus.
            let has_text = !data.composer_text.is_empty();
            let composer_label = if has_text {
                let mut t = data.composer_text.clone();
                if data.composer_focused {
                    t.push('\u{258F}');
                }
                t
            } else if data.composer_focused {
                "\u{258F}".to_string()
            } else {
                data.composer_placeholder.clone()
            };
            let composer_text_color = if has_text {
                tokens.text_primary.to_array()
            } else {
                tokens.text_muted.to_array()
            };
            let composer_border = if data.composer_focused {
                tokens.accent.to_array()
            } else {
                tokens.border_strong.to_array()
            };

            blocks.push(UiBlock {
                id: format!("{}.composer", r.id),
                title: composer_label,
                rect: Rect { x: composer_x, y: composer_y, w: composer_w, h: composer_h },
                header_only: true,
                header_color: Some(tokens.sidebar_input.to_array()),
                border_color: Some(composer_border),
                border_width: 1.0,
                corner_radius: 8.0,
                text_color: Some(composer_text_color),
                ..Default::default()
            });

            // Send affordance — dimmed when the composer cannot send.
            let send_color = if composer_ready && has_text {
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

/// Word-aware wrap of message text to a column budget so long responses stay
/// inside the panel. Falls back to hard breaks for unbroken runs.
fn wrap_message(text: &str, max_cols: usize) -> String {
    let mut out = String::with_capacity(text.len() + text.len() / max_cols.max(1));
    for (i, line) in text.lines().enumerate() {
        if i > 0 {
            out.push('\n');
        }
        let mut col = 0usize;
        for word in line.split(' ') {
            let wlen = word.chars().count();
            if col > 0 && col + 1 + wlen > max_cols {
                out.push('\n');
                col = 0;
            } else if col > 0 {
                out.push(' ');
                col += 1;
            }
            if wlen > max_cols {
                // Hard-break an unbroken run.
                for (j, ch) in word.chars().enumerate() {
                    if j > 0 && j % max_cols == 0 {
                        out.push('\n');
                    }
                    out.push(ch);
                }
                col = wlen % max_cols;
            } else {
                out.push_str(word);
                col += wlen;
            }
        }
    }
    out
}

/// Truncate `text` to `max_cols` characters, appending an ellipsis when cut.
fn truncate_to_cols(text: &str, max_cols: usize) -> String {
    let count = text.chars().count();
    if count <= max_cols {
        return text.to_string();
    }
    let keep = max_cols.saturating_sub(1).max(1);
    let mut out: String = text.chars().take(keep).collect();
    out.push('\u{2026}');
    out
}

/// Select the suffix of messages that fits into `available` vertical space.
///
/// Returns `(start_index, hidden_count)`. When some messages are hidden,
/// 16px is reserved for the "earlier messages hidden" marker.
fn visible_message_window(heights: &[f32], available: f32) -> (usize, usize) {
    if heights.is_empty() {
        return (0, 0);
    }
    let total: f32 = heights.iter().sum();
    if total <= available {
        return (0, 0);
    }
    let marker_h = 16.0;
    let budget = (available - marker_h).max(0.0);
    let mut used = 0.0;
    let mut start = heights.len();
    for (i, h) in heights.iter().enumerate().rev() {
        if used + h > budget {
            break;
        }
        used += h;
        start = i;
    }
    // Always show at least the newest message, even if it must clip.
    if start == heights.len() {
        start = heights.len() - 1;
    }
    (start, start)
}

#[cfg(test)]
mod tests {
    use super::{truncate_to_cols, visible_message_window, wrap_message};

    #[test]
    fn truncate_to_cols_preserves_short_text_and_ellipsizes_long() {
        assert_eq!(truncate_to_cols("short", 10), "short");
        assert_eq!(truncate_to_cols("a very long provider name", 10), "a very lo\u{2026}");
    }

    #[test]
    fn visible_message_window_shows_all_when_fitting() {
        assert_eq!(visible_message_window(&[40.0, 40.0], 200.0), (0, 0));
    }

    #[test]
    fn visible_message_window_hides_oldest_when_overflowing() {
        // 5 messages of 50px into 120px available (minus 16px marker):
        // only the last two fit.
        let heights = [50.0; 5];
        let (start, hidden) = visible_message_window(&heights, 120.0);
        assert_eq!(start, 3);
        assert_eq!(hidden, 3);
    }

    #[test]
    fn visible_message_window_always_keeps_newest_message() {
        let heights = [500.0, 500.0];
        let (start, hidden) = visible_message_window(&heights, 100.0);
        assert_eq!(start, 1, "newest message must remain visible");
        assert_eq!(hidden, 1);
    }

    #[test]
    fn wrap_message_respects_column_budget() {
        let wrapped = wrap_message("alpha beta gamma delta", 11);
        for line in wrapped.lines() {
            assert!(line.chars().count() <= 11, "line too long: {line:?}");
        }
        assert_eq!(wrapped.replace('\n', " "), "alpha beta gamma delta");
    }

    #[test]
    fn wrap_message_hard_breaks_long_runs() {
        let wrapped = wrap_message(&"x".repeat(25), 10);
        let lines: Vec<&str> = wrapped.lines().collect();
        assert_eq!(lines.len(), 3);
        assert!(lines.iter().all(|l| l.chars().count() <= 10));
    }

    #[test]
    fn wrap_message_preserves_existing_newlines() {
        let wrapped = wrap_message("line one\nline two", 40);
        assert_eq!(wrapped, "line one\nline two");
    }
}
