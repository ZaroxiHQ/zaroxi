#![doc = "Application-level AI service used in Phase 11 to orchestrate AI edit proposals.\n\nThis service holds a small in-memory pending proposal list (presentation only)\nand exposes a simple request/accept/reject API that application orchestrators\nand interface adapters can call during Phase 11. The concrete AI provider is\nplumbed as an `AiClient` trait object provided by application composition.\n"]

use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};
use tracing::info;

use zaroxi_domain_ai::types::{AiEditApplyResult, AiEditProposal, AiEditRequest, AiProposalState};
use zaroxi_kernel_types::Id;

/// Re-export port traits used by infra adapters (AiClient trait lives in crate::ports).
use crate::ports::{AiClient, AiRequest};
use crate::trace::AiTracer;

/// AI service for handling AI-related operations and holding pending proposals.
pub struct AiService {
    /// Internal state.
    state: Arc<Mutex<AiServiceState>>,
}

struct AiServiceState {
    /// Whether the service is running.
    running: bool,
    /// In-memory pending proposals (presentation-oriented).
    pending_proposals: Vec<AiEditProposal>,
}

impl Default for AiService {
    fn default() -> Self {
        Self::new()
    }
}

impl AiService {
    /// Create a new AI service.
    pub fn new() -> Self {
        Self {
            state: Arc::new(Mutex::new(AiServiceState {
                running: false,
                pending_proposals: Vec::new(),
            })),
        }
    }

    /// Start the AI service.
    pub async fn start(&self) -> Result<(), anyhow::Error> {
        let mut state = self.state.lock().unwrap();
        if state.running {
            return Err(anyhow::anyhow!("AI service is already running"));
        }
        state.running = true;
        info!("AI service started");
        Ok(())
    }

    /// Stop the AI service.
    pub async fn stop(&self) -> Result<(), anyhow::Error> {
        let mut state = self.state.lock().unwrap();
        if !state.running {
            return Err(anyhow::anyhow!("AI service is not running"));
        }
        state.running = false;
        info!("AI service stopped");
        Ok(())
    }

    /// Check if the service is running.
    pub async fn is_running(&self) -> bool {
        let state = self.state.lock().unwrap();
        state.running
    }

    /// Request an AI edit proposal for the given request using the provided AiClient.
    ///
    /// - Calls the AiClient adapter to obtain text output.
    /// - Normalizes the provider output into an AiEditProposal (presentation DTO).
    /// - Stores the proposal in the in-memory pending_proposals list and returns it.
    ///
    /// Note: this function intentionally keeps the I/O call outside the mutex to avoid
    /// holding locks across awaits.
    pub async fn request_ai_edit(
        &self,
        req: AiEditRequest,
        client: std::sync::Arc<dyn AiClient>,
    ) -> Result<AiEditProposal, anyhow::Error> {
        // Build the adapter-level AiRequest expected by infra adapters.
        let ai_req = AiRequest {
            session_id: Id::new(),
            workspace_id: Id::new(),
            buffer_id: zaroxi_core_editor_buffer::ports::BufferId(req.buffer_id.clone()),
            content_snapshot: req.content.clone(),
        };

        // Call provider (async).
        let resp = client
            .request(ai_req)
            .await
            .map_err(|e| anyhow::anyhow!(format!("ai client error: {:?}", e)))?;

        // Build deterministic proposal id using epoch millis.
        let now = SystemTime::now().duration_since(UNIX_EPOCH)?;
        let id = format!("proposal-{}", now.as_millis());

        let summary: String = if resp.text.len() > 80 {
            format!("{}...", &resp.text[..80])
        } else {
            resp.text.clone()
        };

        let proposal = AiEditProposal {
            id: id.clone(),
            buffer_id: req.buffer_id.clone(),
            summary,
            proposal_text: resp.text.clone(),
            state: AiProposalState::Proposed,
        };

        // Store proposal in-memory (short critical section).
        {
            let mut state = self.state.lock().unwrap();
            state.pending_proposals.push(proposal.clone());
        }

        Ok(proposal)
    }

    /// Streaming, instrumented variant of [`Self::request_ai_edit`].
    ///
    /// Runs entirely on the tokio runtime (never blocks the render thread) and,
    /// when a `tracer` is provided, pushes `ZAROXI_AI_TRACE` events
    /// (prompt-build, request-sent, first-token, stream-complete + tokens/sec)
    /// into the non-blocking trace channel that the render loop drains per frame.
    /// The streamed tokens are accumulated into a single proposal, so callers of
    /// the existing request/accept flow keep working.
    pub async fn request_ai_edit_streaming(
        &self,
        req: AiEditRequest,
        client: std::sync::Arc<dyn AiClient>,
        tracer: Option<AiTracer>,
    ) -> Result<AiEditProposal, anyhow::Error> {
        use crate::ports::AiStreamItem;
        use crate::trace::AiTraceEvent;
        use tokio::time::Instant;

        // ── prompt / context-window build ──
        let t_prompt = Instant::now();
        let ai_req = AiRequest {
            session_id: Id::new(),
            workspace_id: Id::new(),
            buffer_id: zaroxi_core_editor_buffer::ports::BufferId(req.buffer_id.clone()),
            content_snapshot: req.content.clone(),
        };
        let prompt_build_ms = t_prompt.elapsed().as_secs_f32() * 1000.0;
        if let Some(t) = &tracer {
            t.emit(AiTraceEvent::PromptBuilt { ms: prompt_build_ms });
        }

        // ── dispatch + stream tokens ──
        let (tok_tx, mut tok_rx) = tokio::sync::mpsc::unbounded_channel::<AiStreamItem>();
        let t_sent = Instant::now();
        if let Some(t) = &tracer {
            t.emit(AiTraceEvent::RequestSent);
        }
        // Drive the producer concurrently so first-token timing is observable
        // for native-streaming backends.
        let producer = client.request_stream(ai_req, tok_tx);
        let producer_handle = tokio::spawn(producer);

        let mut text = String::new();
        let mut tokens = 0usize;
        let mut first_token_emitted = false;
        while let Some(item) = tok_rx.recv().await {
            match item {
                AiStreamItem::Token(tok) => {
                    if !first_token_emitted {
                        let ms = t_sent.elapsed().as_secs_f32() * 1000.0;
                        if let Some(t) = &tracer {
                            t.emit(AiTraceEvent::FirstToken { ms });
                        }
                        first_token_emitted = true;
                    }
                    tokens += 1;
                    text.push_str(&tok);
                }
                AiStreamItem::Done => break,
            }
        }

        // Surface backend / task errors after the stream drains.
        match producer_handle.await {
            Ok(Ok(())) => {}
            Ok(Err(e)) => return Err(anyhow::anyhow!(format!("ai client error: {:?}", e))),
            Err(e) => return Err(anyhow::anyhow!(format!("ai stream task join error: {:?}", e))),
        }

        let dur = t_sent.elapsed();
        if let Some(t) = &tracer {
            t.emit(AiTraceEvent::StreamComplete {
                ms: dur.as_secs_f32() * 1000.0,
                tokens,
                tokens_per_sec: AiTraceEvent::throughput(tokens, dur),
            });
        }

        // ── normalize into a proposal (parity with request_ai_edit) ──
        let now = SystemTime::now().duration_since(UNIX_EPOCH)?;
        let id = format!("proposal-{}", now.as_millis());
        let summary: String = if text.chars().count() > 80 {
            format!("{}...", text.chars().take(80).collect::<String>())
        } else {
            text.clone()
        };
        let proposal = AiEditProposal {
            id,
            buffer_id: req.buffer_id.clone(),
            summary,
            proposal_text: text,
            state: AiProposalState::Proposed,
        };
        {
            let mut state = self.state.lock().unwrap();
            state.pending_proposals.push(proposal.clone());
        }
        Ok(proposal)
    }

    /// List currently pending proposals (snapshot).
    pub fn list_pending(&self) -> Vec<AiEditProposal> {
        let state = self.state.lock().unwrap();
        state.pending_proposals.clone()
    }

    /// Get a pending proposal by id.
    pub fn get_pending(&self, id: &str) -> Option<AiEditProposal> {
        let state = self.state.lock().unwrap();
        state.pending_proposals.iter().find(|p| p.id == id).cloned()
    }

    /// Accept (apply) a pending proposal.
    ///
    /// This method updates the in-memory proposal state to Applied and returns an
    /// AiEditApplyResult. The actual application of the edit to a buffer must be
    /// performed by the workspace/orchestrator using the proposal.proposal_text
    /// via the editor transaction pipeline. For Phase 1 we mark the proposal as
    /// Applied here to reflect the intent; the caller should perform the real apply.
    pub fn accept_proposal(&self, id: &str) -> Result<AiEditApplyResult, anyhow::Error> {
        let mut state = self.state.lock().unwrap();
        if let Some(pos) = state.pending_proposals.iter().position(|p| p.id == id)
            && let Some(p) = state.pending_proposals.get_mut(pos)
        {
            p.state = AiProposalState::Applied;
            return Ok(AiEditApplyResult {
                ok: true,
                message: Some("Marked applied (service)".to_string()),
            });
        }
        Err(anyhow::anyhow!("proposal not found"))
    }

    /// Reject (clear) a pending proposal: mark rejected and remove from pending list.
    pub fn reject_proposal(&self, id: &str) -> Result<(), anyhow::Error> {
        let mut state = self.state.lock().unwrap();
        if let Some(pos) = state.pending_proposals.iter().position(|p| p.id == id) {
            state.pending_proposals.remove(pos);
            return Ok(());
        }
        Err(anyhow::anyhow!("proposal not found"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ports::{AiError, AiResponseDTO, BoxFuture};
    use crate::trace::AiTraceEvent;

    /// Minimal non-streaming client; the default `request_stream` adapter
    /// tokenizes its response, exercising the full streaming + trace path.
    struct CannedClient(&'static str);
    impl AiClient for CannedClient {
        fn request(&self, _req: AiRequest) -> BoxFuture<'static, Result<AiResponseDTO, AiError>> {
            let text = self.0.to_string();
            Box::pin(async move { Ok(AiResponseDTO { text }) })
        }
    }

    #[tokio::test]
    async fn streaming_accumulates_text_and_emits_trace_events() {
        let svc = AiService::new();
        let (tracer, mut rx) = AiTracer::channel();
        let client: std::sync::Arc<dyn AiClient> =
            std::sync::Arc::new(CannedClient("hello world from ai"));
        let req = AiEditRequest {
            session_id: "sess:1".to_string(),
            buffer_id: "buf:1".to_string(),
            content: "fn main() {}".to_string(),
        };

        let proposal =
            svc.request_ai_edit_streaming(req, client, Some(tracer)).await.expect("stream ok");

        // Tokens were re-assembled losslessly into the proposal text.
        assert_eq!(proposal.proposal_text, "hello world from ai");
        assert_eq!(svc.list_pending().len(), 1);

        let events = rx.drain();
        assert!(events.iter().any(|e| matches!(e, AiTraceEvent::PromptBuilt { .. })));
        assert!(events.iter().any(|e| matches!(e, AiTraceEvent::RequestSent)));
        assert!(events.iter().any(|e| matches!(e, AiTraceEvent::FirstToken { .. })));
        assert!(
            events
                .iter()
                .any(|e| matches!(e, AiTraceEvent::StreamComplete { tokens, .. } if *tokens > 0))
        );
    }
}
