use zaroxi_application_ai::mcp_service::McpStatusSummary;
use zaroxi_application_ai::view_model::{AiPhase, AiSessionState};
use zaroxi_core_engine_ui::ShellWorkContent;
use zaroxi_domain_ai::conversation::{ChatRole, Conversation, ConversationStatus};
use zaroxi_domain_settings::AiSettings;

use super::super::ai_pane::{
    AiPanelData, ApprovalBannerUi, ChatMessageUi, ContextChip, MessageRole, ProviderUiStatus,
};

/// A pending edit proposal, as sourced from the live AI projection.
pub struct PendingProposalSource<'a> {
    /// Target buffer identity the proposal would modify.
    pub target: &'a str,
    /// Full proposed text.
    pub proposal_text: &'a str,
    /// Current text of the target buffer (for the diff summary), when known.
    pub original_text: Option<&'a str>,
}

/// All real application state the AI panel is shaped from.
///
/// This is the single normalization point for AI panel UI state: the GUI app
/// hands its live state in, and rendering consumes only the resulting
/// [`AiPanelData`]. No AI business decisions are made in block-shaping code.
pub struct AiPanelSources<'a> {
    /// Per-frame workspace snapshot (carries the AI projection content view).
    pub work_content: &'a Option<ShellWorkContent>,
    /// User AI settings (provider preference, auto-attach flags).
    pub ai_settings: &'a AiSettings,
    /// Whether an AI-capable backend service is wired into this app instance.
    pub backend_available: bool,
    /// Truthful live session state (phase, streamed tokens, latency).
    pub session: &'a AiSessionState,
    /// Optional live connection status override (e.g. from a connection test).
    /// When `None`, status is derived from settings + backend availability.
    pub provider_override: Option<ProviderUiStatus>,
    /// Path/identity of the active editor file, if any.
    pub active_file: Option<&'a str>,
    /// Current conversation snapshot from the session manager.
    pub conversation: Option<&'a Conversation>,
    /// Active editor selection as a 1-based inclusive line range.
    pub selection_lines: Option<(usize, usize)>,
    /// Name of the open workspace root, if any.
    pub workspace_name: Option<&'a str>,
    /// Current text typed into the composer.
    pub composer_text: &'a str,
    /// Whether the composer holds keyboard focus.
    pub composer_focused: bool,
    /// Pending edit proposal awaiting explicit review, if any.
    pub pending_proposal: Option<PendingProposalSource<'a>>,
    /// MCP status summary — only provided when MCP is enabled in settings,
    /// so the default panel surface stays free of MCP noise.
    pub mcp: Option<McpStatusSummary>,
}

/// Derive the provider status shown in the panel status row.
///
/// This is deliberately conservative: it never claims a live connection that
/// does not exist. "Connected" here means an AI client is wired and the
/// assistant is enabled for the configured provider.
pub fn derive_provider_status(ai: &AiSettings, backend_available: bool) -> ProviderUiStatus {
    if !ai.assistant_enabled || ai.active_provider.trim().is_empty() {
        return ProviderUiStatus::NotConnected;
    }
    if !backend_available {
        return ProviderUiStatus::Error { message: "AI backend not connected".to_string() };
    }
    ProviderUiStatus::Connected { provider: ai.active_provider.clone(), model: String::new() }
}

/// Derive the auto-attached context chips shown above the composer.
pub fn derive_context_chips(
    ai: &AiSettings,
    active_file: Option<&str>,
    selection_lines: Option<(usize, usize)>,
    workspace_name: Option<&str>,
) -> Vec<ContextChip> {
    let mut chips = Vec::new();
    if ai.auto_attach_file
        && let Some(file) = active_file
    {
        let name = file
            .strip_prefix("buf:")
            .unwrap_or(file)
            .rsplit(['/', '\\'])
            .next()
            .unwrap_or(file)
            .to_string();
        if !name.is_empty() {
            chips.push(ContextChip { label: "File".to_string(), detail: name });
        }
    }
    if ai.auto_attach_selection
        && let Some((start, end)) = selection_lines
    {
        let detail =
            if start == end { format!("Ln {start}") } else { format!("Ln {start}\u{2013}{end}") };
        chips.push(ContextChip { label: "Selection".to_string(), detail });
    }
    if let Some(name) = workspace_name
        && !name.trim().is_empty()
    {
        chips.push(ContextChip { label: "Workspace".to_string(), detail: name.to_string() });
    }
    chips
}

/// Map a domain conversation into displayable panel messages.
///
/// The empty assistant placeholder appended by `SessionManager::send_message`
/// is shown as a streaming message while a request is in flight and dropped
/// otherwise; a conversation-level error surfaces as a system message.
pub fn messages_from_conversation(conv: &Conversation) -> Vec<ChatMessageUi> {
    let busy = conversation_is_busy(conv);
    let mut out: Vec<ChatMessageUi> = Vec::with_capacity(conv.messages.len() + 1);
    for msg in &conv.messages {
        let role = match msg.role {
            ChatRole::User => MessageRole::User,
            ChatRole::Assistant => MessageRole::Assistant,
            ChatRole::System => MessageRole::System,
        };
        let is_empty_assistant = role == MessageRole::Assistant && msg.content.is_empty();
        if is_empty_assistant && !busy {
            continue;
        }
        out.push(ChatMessageUi {
            role,
            content: msg.content.clone(),
            is_streaming: msg.is_streaming || (is_empty_assistant && busy),
        });
    }
    if conv.status == ConversationStatus::Error
        && let Some(err) = conv.last_error.as_deref()
    {
        out.push(ChatMessageUi {
            role: MessageRole::System,
            content: err.to_string(),
            is_streaming: false,
        });
    }
    out
}

/// Whether the conversation has an in-flight request.
pub fn conversation_is_busy(conv: &Conversation) -> bool {
    matches!(conv.status, ConversationStatus::Sending | ConversationStatus::Streaming)
}

/// Build the approval banner for a pending edit proposal, including a compact
/// truthful diff summary derived from the domain diff engine.
pub fn approval_from_proposal(src: &PendingProposalSource<'_>) -> ApprovalBannerUi {
    let new_lines = src.proposal_text.lines().count();
    let summary = match src.original_text {
        Some(original) => {
            let old_lines = original.lines().count();
            let changes = zaroxi_domain_ai::diff::compute_diff(original, src.proposal_text);
            let regions = changes.len();
            let noun = if regions == 1 { "changed region" } else { "changed regions" };
            format!("{old_lines} \u{2192} {new_lines} lines \u{00b7} {regions} {noun}")
        }
        None => {
            let noun = if new_lines == 1 { "line" } else { "lines" };
            format!("{new_lines} {noun} proposed")
        }
    };
    ApprovalBannerUi { target: src.target.to_string(), summary }
}

/// Compact MCP status line for the panel status row
/// (e.g. `"MCP 1/2 \u{00b7} 5 tools"`).
pub fn format_mcp_status(summary: &McpStatusSummary) -> String {
    let noun = if summary.tool_count == 1 { "tool" } else { "tools" };
    format!("MCP {}/{} \u{00b7} {} {}", summary.connected, summary.total, summary.tool_count, noun)
}

/// Compact conversation summary for the panel subtitle
/// (e.g. `"New Chat \u{00b7} 3 messages"`). `None` when the chat is empty.
pub fn conversation_subtitle(conv: &Conversation) -> Option<String> {
    let visible = conv.messages.iter().filter(|m| !m.content.is_empty()).count();
    if visible == 0 {
        return None;
    }
    let noun = if visible == 1 { "message" } else { "messages" };
    Some(format!("{} \u{00b7} {} {}", conv.title, visible, noun))
}

/// State-aware placeholder for the prompt composer.
pub fn composer_placeholder_for(status: &ProviderUiStatus, is_loading: bool) -> String {
    if is_loading {
        return "Waiting for response\u{2026}".to_string();
    }
    match status {
        ProviderUiStatus::Connected { .. } => "Ask about your code\u{2026}".to_string(),
        ProviderUiStatus::Connecting => "Connecting\u{2026}".to_string(),
        ProviderUiStatus::NotConnected | ProviderUiStatus::Error { .. } => {
            "Configure a provider to start".to_string()
        }
    }
}

/// Whether the session is actively working (drives the loading indicator).
/// `Complete` is intentionally not a loading state.
pub fn session_is_loading(session: &AiSessionState) -> bool {
    matches!(session.phase, AiPhase::PromptBuilding | AiPhase::Requesting | AiPhase::Streaming)
}

/// Shape the full AI panel view state from live application sources.
pub fn shape_ai_panel(src: AiPanelSources<'_>) -> AiPanelData {
    let (ai_content, ai_title, ai_subtitle) = match src.work_content {
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

    let provider_status = src
        .provider_override
        .unwrap_or_else(|| derive_provider_status(src.ai_settings, src.backend_available));
    let conversation_busy = src.conversation.map(conversation_is_busy).unwrap_or(false);
    let is_loading = session_is_loading(src.session) || conversation_busy;
    let composer_placeholder = composer_placeholder_for(&provider_status, is_loading);
    let show_setup_cta = !matches!(
        provider_status,
        ProviderUiStatus::Connected { .. } | ProviderUiStatus::Connecting
    );
    let context_chips = derive_context_chips(
        src.ai_settings,
        src.active_file,
        src.selection_lines,
        src.workspace_name,
    );
    let messages = src.conversation.map(messages_from_conversation).unwrap_or_default();
    let approval = src.pending_proposal.as_ref().map(approval_from_proposal);
    let show_quick_actions =
        !show_setup_cta && src.active_file.is_some() && approval.is_none() && !is_loading;
    let mcp_status = src.mcp.as_ref().map(format_mcp_status);
    // Truthful operational activity: request phase or completed-run stats.
    let activity_line = src.session.status_label();

    let mut data = AiPanelData {
        ai_content,
        ai_title,
        ai_subtitle,
        provider_status: Some(provider_status),
        messages,
        context_chips,
        is_loading,
        composer_placeholder,
        composer_text: src.composer_text.to_string(),
        composer_focused: src.composer_focused,
        show_setup_cta,
        show_session_controls: !show_setup_cta,
        show_quick_actions,
        approval,
        mcp_status,
        activity_line,
    };

    // Subtitle priority: panel content subtitle → conversation summary →
    // truthful session status.
    if data.ai_subtitle.as_deref().map(str::trim).unwrap_or("").is_empty() {
        data.ai_subtitle =
            src.conversation.and_then(conversation_subtitle).or_else(|| src.session.status_label());
    }

    data
}

#[cfg(test)]
mod tests {
    use super::*;

    fn settings() -> AiSettings {
        AiSettings::default()
    }

    #[test]
    fn disabled_assistant_reports_not_connected() {
        let mut ai = settings();
        ai.assistant_enabled = false;
        assert_eq!(derive_provider_status(&ai, true), ProviderUiStatus::NotConnected);
    }

    #[test]
    fn enabled_with_backend_reports_connected_with_provider_name() {
        let ai = settings();
        match derive_provider_status(&ai, true) {
            ProviderUiStatus::Connected { provider, .. } => assert_eq!(provider, "OpenAI"),
            other => panic!("expected Connected, got {other:?}"),
        }
    }

    #[test]
    fn enabled_without_backend_reports_error() {
        let ai = settings();
        assert!(matches!(derive_provider_status(&ai, false), ProviderUiStatus::Error { .. }));
    }

    #[test]
    fn empty_provider_name_reports_not_connected() {
        let mut ai = settings();
        ai.active_provider = "  ".into();
        assert_eq!(derive_provider_status(&ai, true), ProviderUiStatus::NotConnected);
    }

    #[test]
    fn auto_attach_file_yields_file_chip_with_basename() {
        let ai = settings();
        let chips = derive_context_chips(&ai, Some("buf:src/main.rs"), None, None);
        assert_eq!(chips.len(), 1);
        assert_eq!(chips[0].label, "File");
        assert_eq!(chips[0].detail, "main.rs");
    }

    #[test]
    fn auto_attach_disabled_yields_no_chips() {
        let mut ai = settings();
        ai.auto_attach_file = false;
        assert!(derive_context_chips(&ai, Some("src/main.rs"), None, None).is_empty());
    }

    #[test]
    fn no_active_file_yields_no_chips() {
        let ai = settings();
        assert!(derive_context_chips(&ai, None, None, None).is_empty());
    }

    #[test]
    fn selection_chip_shows_line_range() {
        let ai = settings();
        let chips = derive_context_chips(&ai, None, Some((3, 9)), None);
        assert_eq!(chips.len(), 1);
        assert_eq!(chips[0].label, "Selection");
        assert_eq!(chips[0].detail, "Ln 3\u{2013}9");

        let single = derive_context_chips(&ai, None, Some((5, 5)), None);
        assert_eq!(single[0].detail, "Ln 5");
    }

    #[test]
    fn selection_chip_respects_auto_attach_setting() {
        let mut ai = settings();
        ai.auto_attach_selection = false;
        assert!(derive_context_chips(&ai, None, Some((1, 4)), None).is_empty());
    }

    #[test]
    fn workspace_chip_shows_root_name() {
        let ai = settings();
        let chips = derive_context_chips(&ai, None, None, Some("zaroxi"));
        assert_eq!(chips.len(), 1);
        assert_eq!(chips[0].label, "Workspace");
        assert_eq!(chips[0].detail, "zaroxi");
    }

    #[test]
    fn complete_phase_is_not_loading() {
        let mut session = AiSessionState::default();
        session.phase = AiPhase::Complete;
        assert!(!session_is_loading(&session));
        session.phase = AiPhase::Streaming;
        assert!(session_is_loading(&session));
    }

    #[test]
    fn composer_placeholder_tracks_state() {
        let connected =
            ProviderUiStatus::Connected { provider: "OpenAI".into(), model: String::new() };
        assert_eq!(composer_placeholder_for(&connected, false), "Ask about your code\u{2026}");
        assert_eq!(composer_placeholder_for(&connected, true), "Waiting for response\u{2026}");
        assert_eq!(
            composer_placeholder_for(&ProviderUiStatus::NotConnected, false),
            "Configure a provider to start"
        );
    }

    #[test]
    fn shape_ai_panel_normalizes_setup_cta_and_controls() {
        let session = AiSessionState::default();
        let ai = settings();
        let data = shape_ai_panel(AiPanelSources {
            work_content: &None,
            ai_settings: &ai,
            backend_available: true,
            session: &session,
            provider_override: None,
            active_file: None,
            conversation: None,
            selection_lines: None,
            workspace_name: None,
            composer_text: "",
            composer_focused: false,
            pending_proposal: None,
            mcp: None,
        });
        assert!(!data.show_setup_cta);
        assert!(data.show_session_controls);

        let mut disabled = settings();
        disabled.assistant_enabled = false;
        let data = shape_ai_panel(AiPanelSources {
            work_content: &None,
            ai_settings: &disabled,
            backend_available: true,
            session: &session,
            provider_override: None,
            active_file: None,
            conversation: None,
            selection_lines: None,
            workspace_name: None,
            composer_text: "",
            composer_focused: false,
            pending_proposal: None,
            mcp: None,
        });
        assert!(data.show_setup_cta);
        assert!(!data.show_session_controls);
    }

    #[test]
    fn messages_map_roles_and_surface_errors() {
        use zaroxi_domain_ai::conversation::ChatMessage;
        let mut conv = Conversation::new();
        conv.add_message(ChatMessage::user("explain this"));
        conv.add_message(ChatMessage::assistant("It does X."));
        conv.status = ConversationStatus::Error;
        conv.last_error = Some("timeout".into());

        let msgs = messages_from_conversation(&conv);
        assert_eq!(msgs.len(), 3);
        assert_eq!(msgs[0].role, MessageRole::User);
        assert_eq!(msgs[1].role, MessageRole::Assistant);
        assert_eq!(msgs[2].role, MessageRole::System);
        assert_eq!(msgs[2].content, "timeout");
    }

    #[test]
    fn empty_assistant_placeholder_streams_while_busy_and_hides_when_idle() {
        use zaroxi_domain_ai::conversation::ChatMessage;
        let mut conv = Conversation::new();
        conv.add_message(ChatMessage::user("hello"));
        conv.add_message(ChatMessage::assistant(""));

        conv.status = ConversationStatus::Sending;
        let busy = messages_from_conversation(&conv);
        assert_eq!(busy.len(), 2);
        assert!(busy[1].is_streaming, "pending assistant reply must show as streaming");

        conv.status = ConversationStatus::Idle;
        let idle = messages_from_conversation(&conv);
        assert_eq!(idle.len(), 1, "empty assistant placeholder must be hidden when idle");
    }

    #[test]
    fn conversation_subtitle_counts_visible_messages() {
        use zaroxi_domain_ai::conversation::ChatMessage;
        let mut conv = Conversation::new();
        assert!(conversation_subtitle(&conv).is_none());

        conv.add_message(ChatMessage::user("q"));
        conv.add_message(ChatMessage::assistant("a"));
        conv.add_message(ChatMessage::assistant(""));
        let subtitle = conversation_subtitle(&conv).unwrap();
        assert!(subtitle.contains("2 messages"), "empty messages must not be counted: {subtitle}");
    }

    #[test]
    fn conversation_busy_drives_loading_state() {
        use zaroxi_domain_ai::conversation::ChatMessage;
        let session = AiSessionState::default();
        let ai = settings();
        let mut conv = Conversation::new();
        conv.add_message(ChatMessage::user("q"));
        conv.status = ConversationStatus::Sending;

        let data = shape_ai_panel(AiPanelSources {
            work_content: &None,
            ai_settings: &ai,
            backend_available: true,
            session: &session,
            provider_override: None,
            active_file: None,
            conversation: Some(&conv),
            selection_lines: None,
            workspace_name: None,
            composer_text: "",
            composer_focused: false,
            pending_proposal: None,
            mcp: None,
        });
        assert!(data.is_loading, "sending conversation must show loading state");
    }

    #[test]
    fn approval_summary_reports_line_delta_and_regions() {
        let src = PendingProposalSource {
            target: "main.rs",
            proposal_text: "a\nb\nc\nd",
            original_text: Some("a\nb\nc"),
        };
        let banner = approval_from_proposal(&src);
        assert_eq!(banner.target, "main.rs");
        assert!(banner.summary.starts_with("3 \u{2192} 4 lines"), "got: {}", banner.summary);
        assert!(banner.summary.contains("changed region"), "got: {}", banner.summary);
    }

    #[test]
    fn approval_summary_without_original_reports_proposed_lines() {
        let src = PendingProposalSource {
            target: "lib.rs",
            proposal_text: "one\ntwo",
            original_text: None,
        };
        let banner = approval_from_proposal(&src);
        assert_eq!(banner.summary, "2 lines proposed");
    }

    #[test]
    fn quick_actions_require_active_file_and_no_pending_proposal() {
        let session = AiSessionState::default();
        let ai = settings();

        let base = |active_file: Option<&'static str>,
                    proposal: Option<PendingProposalSource<'static>>| {
            shape_ai_panel(AiPanelSources {
                work_content: &None,
                ai_settings: &ai,
                backend_available: true,
                session: &session,
                provider_override: None,
                active_file,
                conversation: None,
                selection_lines: None,
                workspace_name: None,
                composer_text: "",
                composer_focused: false,
                pending_proposal: proposal,
                mcp: None,
            })
        };

        assert!(!base(None, None).show_quick_actions, "no active file → no quick actions");
        assert!(base(Some("buf:a.rs"), None).show_quick_actions);

        let with_proposal = base(
            Some("buf:a.rs"),
            Some(PendingProposalSource { target: "a.rs", proposal_text: "x", original_text: None }),
        );
        assert!(
            !with_proposal.show_quick_actions,
            "pending proposal must replace quick actions with approval"
        );
        assert!(with_proposal.approval.is_some(), "approval banner must be present");
    }

    #[test]
    fn mcp_status_line_formats_counts() {
        let summary = McpStatusSummary { connected: 1, total: 2, tool_count: 5 };
        assert_eq!(format_mcp_status(&summary), "MCP 1/2 \u{00b7} 5 tools");
        let single = McpStatusSummary { connected: 0, total: 1, tool_count: 1 };
        assert_eq!(format_mcp_status(&single), "MCP 0/1 \u{00b7} 1 tool");
    }

    #[test]
    fn mcp_status_only_surfaces_when_provided() {
        let session = AiSessionState::default();
        let ai = settings();
        let shape = |mcp: Option<McpStatusSummary>| {
            shape_ai_panel(AiPanelSources {
                work_content: &None,
                ai_settings: &ai,
                backend_available: true,
                session: &session,
                provider_override: None,
                active_file: None,
                conversation: None,
                selection_lines: None,
                workspace_name: None,
                composer_text: "",
                composer_focused: false,
                pending_proposal: None,
                mcp,
            })
        };
        assert!(shape(None).mcp_status.is_none(), "MCP off → no MCP status in the panel");
        let on = shape(Some(McpStatusSummary { connected: 2, total: 2, tool_count: 7 }));
        assert_eq!(on.mcp_status.as_deref(), Some("MCP 2/2 \u{00b7} 7 tools"));
    }

    #[test]
    fn activity_line_surfaces_session_phase_and_run_stats() {
        let ai = settings();
        let mut session = AiSessionState::default();
        session.phase = AiPhase::Streaming;
        let shape = |session: &AiSessionState| {
            shape_ai_panel(AiPanelSources {
                work_content: &None,
                ai_settings: &ai,
                backend_available: true,
                session,
                provider_override: None,
                active_file: None,
                conversation: None,
                selection_lines: None,
                workspace_name: None,
                composer_text: "",
                composer_focused: false,
                pending_proposal: None,
                mcp: None,
            })
        };
        assert_eq!(shape(&session).activity_line.as_deref(), Some("Streaming\u{2026}"));

        session.phase = AiPhase::Idle;
        assert!(shape(&session).activity_line.is_none(), "idle session → no activity noise");
    }
}
