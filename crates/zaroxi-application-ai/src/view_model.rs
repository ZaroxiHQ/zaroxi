// Minimal AI panel view models owned by application-ai

use crate::trace::{AiTraceEvent, AiTraceReceiver, ai_trace_enabled};

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct AiCard {
    pub title: String,
    pub body: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct AiPanelState {
    pub header: String,
    pub cards: Vec<AiCard>,
    pub composer_text: String,
}

/// Lifecycle phase of the current AI request.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum AiPhase {
    /// No request in flight (initial state / fallback).
    #[default]
    Idle,
    /// Assembling the prompt / context window.
    PromptBuilding,
    /// Request dispatched; awaiting the first token.
    Requesting,
    /// Tokens are streaming back from the backend.
    Streaming,
    /// The stream has finished.
    Complete,
}

/// A UI-facing snapshot of the live AI session, folded from [`AiTraceEvent`]s.
///
/// **Truthfulness contract:** every field reflects state the current pipeline
/// actually reports. There is deliberately *no* model context-window total and
/// *no* edit-prediction metadata here, because the request/response pipeline
/// does not produce them — consumers treat their absence as "unavailable" and
/// render a fallback rather than an invented number.
///
/// The state is updated incrementally as trace events arrive (see
/// [`AiSessionState::drain_from`]), so it is ready for richer streaming and
/// partial-result feedback as the backend grows.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct AiSessionState {
    /// Current request lifecycle phase.
    pub phase: AiPhase,
    /// Tokens streamed in the most recent request (final count, known at
    /// completion; `0` while idle or before the stream completes).
    pub tokens_streamed: usize,
    /// Latency from request-sent to the first token, if observed (ms).
    pub first_token_ms: Option<f32>,
    /// Total stream duration of the last completed request (ms).
    pub last_stream_ms: Option<f32>,
    /// Throughput (tokens/sec) of the last completed stream.
    pub tokens_per_sec: Option<f32>,
    /// Prompt/context-build time of the current request, if observed (ms).
    pub prompt_build_ms: Option<f32>,
}

impl AiSessionState {
    /// Fold a single trace event into the session snapshot.
    pub fn apply(&mut self, event: &AiTraceEvent) {
        match event {
            AiTraceEvent::PromptBuilt { ms } => {
                // First event of a new request: reset per-request metrics.
                self.phase = AiPhase::PromptBuilding;
                self.prompt_build_ms = Some(*ms);
                self.first_token_ms = None;
                self.tokens_streamed = 0;
            }
            AiTraceEvent::RequestSent => {
                self.phase = AiPhase::Requesting;
            }
            AiTraceEvent::FirstToken { ms } => {
                self.phase = AiPhase::Streaming;
                self.first_token_ms = Some(*ms);
            }
            AiTraceEvent::StreamComplete { ms, tokens, tokens_per_sec } => {
                self.phase = AiPhase::Complete;
                self.tokens_streamed = *tokens;
                self.last_stream_ms = Some(*ms);
                self.tokens_per_sec = Some(*tokens_per_sec);
            }
            // Buffer-apply timing is not part of the session surface.
            AiTraceEvent::Applied { .. } => {}
        }
    }

    /// Drain all pending events from `rx` into this session (non-blocking),
    /// preserving the `ZAROXI_AI_TRACE` print behaviour. Returns the count
    /// drained. Intended to be called once per frame from the render loop in
    /// place of [`AiTraceReceiver::drain_to_trace`].
    pub fn drain_from(&mut self, rx: &mut AiTraceReceiver) -> usize {
        let events = rx.drain();
        for event in &events {
            if ai_trace_enabled() {
                eprintln!("{}", event.format_line());
            }
            self.apply(event);
        }
        events.len()
    }

    /// A short, truthful status label for the assistant panel, or `None` when
    /// idle (so the panel falls back to its own/empty state).
    pub fn status_label(&self) -> Option<String> {
        match self.phase {
            AiPhase::Idle => None,
            AiPhase::PromptBuilding => Some("Preparing request\u{2026}".to_string()),
            AiPhase::Requesting => Some("Waiting for response\u{2026}".to_string()),
            AiPhase::Streaming => Some("Streaming\u{2026}".to_string()),
            AiPhase::Complete => {
                let mut label = format!("{} tokens", self.tokens_streamed);
                if let Some(tps) = self.tokens_per_sec {
                    label.push_str(&format!(" \u{00b7} {tps:.0} tok/s"));
                }
                Some(label)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_session_is_idle_with_no_label() {
        let s = AiSessionState::default();
        assert_eq!(s.phase, AiPhase::Idle);
        assert_eq!(s.tokens_streamed, 0);
        assert_eq!(s.status_label(), None);
    }

    #[test]
    fn folds_request_lifecycle_into_truthful_state() {
        let mut s = AiSessionState::default();
        s.apply(&AiTraceEvent::PromptBuilt { ms: 1.0 });
        assert_eq!(s.phase, AiPhase::PromptBuilding);
        s.apply(&AiTraceEvent::RequestSent);
        assert_eq!(s.phase, AiPhase::Requesting);
        s.apply(&AiTraceEvent::FirstToken { ms: 12.0 });
        assert_eq!(s.phase, AiPhase::Streaming);
        assert_eq!(s.first_token_ms, Some(12.0));
        s.apply(&AiTraceEvent::StreamComplete { ms: 200.0, tokens: 40, tokens_per_sec: 200.0 });
        assert_eq!(s.phase, AiPhase::Complete);
        assert_eq!(s.tokens_streamed, 40);
        assert_eq!(s.tokens_per_sec, Some(200.0));
        assert_eq!(s.status_label(), Some("40 tokens \u{00b7} 200 tok/s".to_string()));
    }

    #[test]
    fn new_request_resets_previous_token_count() {
        let mut s = AiSessionState::default();
        s.apply(&AiTraceEvent::StreamComplete { ms: 100.0, tokens: 99, tokens_per_sec: 50.0 });
        assert_eq!(s.tokens_streamed, 99);
        // A fresh request starts by rebuilding the prompt, which must reset.
        s.apply(&AiTraceEvent::PromptBuilt { ms: 0.5 });
        assert_eq!(s.tokens_streamed, 0);
        assert_eq!(s.first_token_ms, None);
        assert_eq!(s.phase, AiPhase::PromptBuilding);
    }

    #[test]
    fn drain_from_folds_channel_events() {
        let (tracer, mut rx) = crate::trace::AiTracer::channel();
        tracer.emit(AiTraceEvent::PromptBuilt { ms: 1.0 });
        tracer.emit(AiTraceEvent::RequestSent);
        tracer.emit(AiTraceEvent::FirstToken { ms: 5.0 });
        tracer.emit(AiTraceEvent::StreamComplete { ms: 50.0, tokens: 7, tokens_per_sec: 140.0 });
        let mut s = AiSessionState::default();
        let n = s.drain_from(&mut rx);
        assert_eq!(n, 4);
        assert_eq!(s.phase, AiPhase::Complete);
        assert_eq!(s.tokens_streamed, 7);
    }
}
