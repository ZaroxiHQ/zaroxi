/*!
AI chat operations for the assistant panel.

Owns the user-facing conversation flows: sending a prompt from the composer,
starting a new chat, and clearing the current conversation. The conversation
itself lives in the application-layer `SessionManager`; this module only
bridges GUI events into it and into the existing AI request pipeline.
*/

use super::GuiApp;
use super::InvalidationFlags;
use crate::gui::window::ai_pane::ProviderUiStatus;
use crate::gui::window::presenters::ai_presenter;

impl GuiApp {
    /// Whether the composer can currently send a prompt: a provider is
    /// available and no request is in flight.
    pub fn ai_composer_ready(&self) -> bool {
        let status = self.ai_provider_status.clone().unwrap_or_else(|| {
            ai_presenter::derive_provider_status(
                &self.settings.ai,
                self.workspace_service.is_some(),
            )
        });
        matches!(status, ProviderUiStatus::Connected { .. })
            && !ai_presenter::session_is_loading(&self.ai_session)
            && !ai_presenter::conversation_is_busy(&self.ai_chat.active_conversation())
    }

    /// Send the current composer text as a user message and dispatch it to
    /// the AI backend through the existing request pipeline.
    ///
    /// The user message is always recorded; when no backend is wired the
    /// conversation surfaces a truthful error instead of pretending to work.
    pub fn ai_send_prompt(&mut self) {
        let text = self.ai_composer_text.trim().to_string();
        if text.is_empty() || !self.ai_composer_ready() {
            return;
        }
        self.ai_chat.send_message(&text);
        self.ai_composer_text.clear();

        let backend = match (
            self.composition.as_mut(),
            self.workspace_view.as_ref(),
            self.workspace_service.as_ref(),
            self.session_id.as_ref(),
        ) {
            (Some(comp), Some(view), Some(service), Some(session)) => {
                Some((comp, view.clone(), service.clone(), session.clone()))
            }
            _ => None,
        };

        match backend {
            Some((comp, view, service, session)) => {
                let result = pollster::block_on(crate::desktop::request_ai_edit_active(
                    comp,
                    view,
                    session,
                    Some(service),
                ));
                match result {
                    Ok(()) => {
                        let reply = comp
                            .latest_ai_projection()
                            .and_then(|p| p.result.clone().or(p.proposal_text.clone()))
                            .unwrap_or_default();
                        if reply.is_empty() {
                            self.ai_chat.set_error("The assistant returned an empty response.");
                        } else {
                            self.ai_chat.append_streaming_chunk(&reply);
                            self.ai_chat.finish_streaming();
                        }
                        self.work_content = Some(comp.build_work_content());
                    }
                    Err(e) => {
                        self.ai_chat.set_error(&e);
                    }
                }
            }
            None => {
                self.ai_chat.set_error("Assistant backend is not available in this session.");
            }
        }
        self.invalidate(InvalidationFlags::content());
    }

    /// Start a new chat: archive the current conversation, reset the live
    /// session state, and clear any pending AI projection.
    pub fn ai_new_chat(&mut self) {
        self.ai_chat.new_chat();
        self.ai_reset_panel_state();
    }

    /// Clear the current conversation in place (with the same projection and
    /// session reset as a new chat, but without archiving to history).
    pub fn ai_clear_conversation(&mut self) {
        self.ai_chat.reset();
        self.ai_reset_panel_state();
    }

    fn ai_reset_panel_state(&mut self) {
        self.ai_session = zaroxi_application_ai::view_model::AiSessionState::default();
        self.ai_composer_text.clear();
        let service = self.workspace_service.clone();
        let session = self.session_id.clone();
        if let Some(comp) = self.composition.as_mut() {
            crate::desktop::cancel_ai_edit_active(comp, service, session);
            self.work_content = Some(comp.build_work_content());
        }
        self.invalidate(InvalidationFlags::content());
    }

    /// Test/automation seam: route a logical key press through the same
    /// keyboard pipeline as the window event loop.
    pub fn press_key(&mut self, logical_key: &winit::keyboard::Key) {
        let actions = super::input::handle_keyboard_press(self, logical_key);
        self.handle_actions(actions);
    }
}
