use zaroxi_application_ai::view_model::{AiPhase, AiSessionState};
use zaroxi_core_engine_ui::ShellWorkContent;
use zaroxi_domain_settings::AiSettings;

use super::super::ai_pane::{AiPanelData, ChatMessageUi, ContextChip, ProviderUiStatus};

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
    /// Conversation messages to display (wired in the chat phase).
    pub messages: Vec<ChatMessageUi>,
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
pub fn derive_context_chips(ai: &AiSettings, active_file: Option<&str>) -> Vec<ContextChip> {
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
    chips
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
    let is_loading = session_is_loading(src.session);
    let composer_placeholder = composer_placeholder_for(&provider_status, is_loading);
    let show_setup_cta = !matches!(
        provider_status,
        ProviderUiStatus::Connected { .. } | ProviderUiStatus::Connecting
    );
    let context_chips = derive_context_chips(src.ai_settings, src.active_file);

    let mut data = AiPanelData {
        ai_content,
        ai_title,
        ai_subtitle,
        provider_status: Some(provider_status),
        messages: src.messages,
        context_chips,
        is_loading,
        composer_placeholder,
        show_setup_cta,
        show_session_controls: !show_setup_cta,
    };

    // Surface the truthful AI session status in the subtitle when the panel
    // content does not provide one of its own.
    if data.ai_subtitle.as_deref().map(str::trim).unwrap_or("").is_empty()
        && let Some(status) = src.session.status_label()
    {
        data.ai_subtitle = Some(status);
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
        let chips = derive_context_chips(&ai, Some("buf:src/main.rs"));
        assert_eq!(chips.len(), 1);
        assert_eq!(chips[0].label, "File");
        assert_eq!(chips[0].detail, "main.rs");
    }

    #[test]
    fn auto_attach_disabled_yields_no_chips() {
        let mut ai = settings();
        ai.auto_attach_file = false;
        assert!(derive_context_chips(&ai, Some("src/main.rs")).is_empty());
    }

    #[test]
    fn no_active_file_yields_no_chips() {
        let ai = settings();
        assert!(derive_context_chips(&ai, None).is_empty());
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
            messages: Vec::new(),
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
            messages: Vec::new(),
        });
        assert!(data.show_setup_cta);
        assert!(!data.show_session_controls);
    }
}
