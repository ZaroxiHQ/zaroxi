//! AI action service — routes action requests to the appropriate AI client
//! with prompt templates tailored to each action kind.
//!
//! Phase 2: builds action-specific prompts, incorporates context, handles
//! response normalization, and produces structured `DiffResult`s.

use zaroxi_domain_ai::actions::{ActionKind, ActionSpec, DiffResult};
use zaroxi_domain_ai::context_ide::ContextPack;

use crate::ports::{AiClient, AiRequest, AiResponseDTO};
use crate::trace::AiTracer;
use zaroxi_kernel_types::Id;

/// Service for dispatching AI actions with appropriate prompt engineering.
pub struct ActionService {
    /// The AI client used for all requests.
    client: std::sync::Arc<dyn AiClient>,
}

impl ActionService {
    pub fn new(client: std::sync::Arc<dyn AiClient>) -> Self {
        Self { client }
    }

    /// Build a complete prompt for an action.
    pub fn build_prompt(
        spec: &ActionSpec,
        context: Option<&ContextPack>,
        extra_instruction: Option<&str>,
    ) -> String {
        let mut prompt = String::new();

        // System-level instruction
        prompt.push_str("Instruction: ");
        prompt.push_str(spec.kind.prompt_instruction());
        prompt.push('\n');

        if let Some(ref instr) = spec.instruction {
            if !instr.is_empty() {
                prompt.push_str("Request: ");
                prompt.push_str(instr);
                prompt.push('\n');
            }
        }
        if let Some(extra) = extra_instruction {
            prompt.push_str("Additional guidance: ");
            prompt.push_str(extra);
            prompt.push('\n');
        }

        // File context
        if let Some(ref path) = spec.file_path {
            prompt.push_str("File: ");
            prompt.push_str(path);
            prompt.push('\n');
        }
        if let Some(ref lang) = spec.language {
            prompt.push_str("Language: ");
            prompt.push_str(lang);
            prompt.push('\n');
        }

        // Attach IDE context
        if let Some(ctx) = context {
            if ctx.has_any_context() {
                prompt.push_str("\n--- Context ---\n");
                prompt.push_str(&ctx.format_for_prompt());
            }
        }

        // Attach diagnostics
        if !spec.diagnostics.is_empty() {
            prompt.push_str("\n--- Diagnostics ---\n");
            for diag in &spec.diagnostics {
                prompt.push_str(&format!("[{}] {}", diag.severity, diag.message));
                if let (Some(l), Some(c)) = (diag.line, diag.column) {
                    prompt.push_str(&format!(" (line {l}, col {c})"));
                }
                prompt.push('\n');
            }
        }

        // Surrounding context
        if let Some(ref surrounding) = spec.surrounding_context {
            if !surrounding.is_empty() {
                prompt.push_str("\n--- Surrounding Code ---\n");
                prompt.push_str(surrounding);
                prompt.push('\n');
            }
        }

        // Target content
        prompt.push_str("\n--- Code ---\n");
        prompt.push_str(&spec.target_content);
        prompt.push('\n');

        // Output format instruction for structured responses
        prompt.push_str("\nProvide your response below. ");
        match spec.kind {
            ActionKind::Edit | ActionKind::Refactor | ActionKind::FixDiagnostics => {
                prompt.push_str("Output the modified code in a fenced code block (```).");
            }
            ActionKind::Explain | ActionKind::Review => {
                prompt.push_str("Be concise and structured.");
            }
            ActionKind::GenerateTests => {
                prompt.push_str("Output the test code in a fenced code block (```).");
            }
        }

        prompt
    }

    /// Execute an action synchronously (non-streaming).
    pub async fn execute(
        &self,
        spec: &ActionSpec,
        context: Option<&ContextPack>,
        tracer: Option<AiTracer>,
    ) -> Result<ActionResponse, String> {
        let prompt = Self::build_prompt(spec, context, None);
        let request = AiRequest {
            session_id: Id::new(),
            workspace_id: Id::new(),
            buffer_id: zaroxi_core_editor_buffer::ports::BufferId(spec.target_buffer.clone()),
            content_snapshot: prompt,
        };

        if let Some(t) = &tracer {
            t.emit(crate::trace::AiTraceEvent::RequestSent);
        }

        let response =
            self.client.request(request).await.map_err(|e| format!("AI request failed: {e}"))?;

        let diff = Self::normalize_response(spec, &response);

        Ok(ActionResponse {
            action_kind: spec.kind,
            text: response.text.clone(),
            diff,
            buffer_id: spec.target_buffer.clone(),
        })
    }

    /// Execute with streaming support via the trace channel.
    pub async fn execute_streaming(
        &self,
        spec: &ActionSpec,
        context: Option<&ContextPack>,
        tracer: AiTracer,
    ) -> Result<ActionResponse, String> {
        use crate::ports::AiStreamItem;
        let prompt = Self::build_prompt(spec, context, None);
        let request = AiRequest {
            session_id: Id::new(),
            workspace_id: Id::new(),
            buffer_id: zaroxi_core_editor_buffer::ports::BufferId(spec.target_buffer.clone()),
            content_snapshot: prompt,
        };

        let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();
        tracer.emit(crate::trace::AiTraceEvent::RequestSent);

        let producer = self.client.request_stream(request, tx);
        let handle = tokio::spawn(producer);

        let mut text = String::new();
        while let Some(item) = rx.recv().await {
            match item {
                AiStreamItem::Token(tok) => {
                    text.push_str(&tok);
                }
                AiStreamItem::Done => break,
            }
        }

        match handle.await {
            Ok(Ok(())) => {}
            Ok(Err(e)) => return Err(format!("Stream error: {e}")),
            Err(e) => return Err(format!("Task error: {e}")),
        }

        let response = AiResponseDTO { text: text.clone() };
        let diff = Self::normalize_response(spec, &response);

        Ok(ActionResponse {
            action_kind: spec.kind,
            text,
            diff,
            buffer_id: spec.target_buffer.clone(),
        })
    }

    /// Normalize the AI response into a structured `DiffResult` if the action
    /// type suggests a code output.
    fn normalize_response(spec: &ActionSpec, response: &AiResponseDTO) -> Option<DiffResult> {
        match spec.kind {
            ActionKind::Edit
            | ActionKind::Refactor
            | ActionKind::FixDiagnostics
            | ActionKind::GenerateTests => {
                let changes = zaroxi_domain_ai::diff::parse_diff_from_response(
                    &response.text,
                    &spec.target_content,
                );
                if changes.is_empty() {
                    None
                } else {
                    Some(DiffResult {
                        buffer_id: spec.target_buffer.clone(),
                        changes,
                        full_replacement: None,
                        summary: format!("{} result", spec.kind.display_name()),
                    })
                }
            }
            ActionKind::Explain | ActionKind::Review => {
                // These actions produce explanatory text, not code diffs.
                None
            }
        }
    }
}

/// The result of executing an AI action.
#[derive(Debug, Clone)]
pub struct ActionResponse {
    pub action_kind: ActionKind,
    pub text: String,
    pub diff: Option<DiffResult>,
    pub buffer_id: String,
}

impl ActionResponse {
    pub fn has_diff(&self) -> bool {
        self.diff.as_ref().map(|d| d.has_changes()).unwrap_or(false)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn build_prompt_includes_action_instruction_and_code() {
        let spec = ActionSpec {
            kind: ActionKind::Explain,
            instruction: Some("What does this do?".into()),
            target_buffer: "buf:test".into(),
            target_content: "fn add(a: i32, b: i32) -> i32 { a + b }".into(),
            surrounding_context: None,
            diagnostics: vec![],
            file_path: Some("src/lib.rs".into()),
            language: Some("rust".into()),
        };
        let prompt = ActionService::build_prompt(&spec, None, None);
        assert!(prompt.contains("Explain the following code"));
        assert!(prompt.contains("What does this do?"));
        assert!(prompt.contains("src/lib.rs"));
        assert!(prompt.contains("rust"));
        assert!(prompt.contains("fn add"));
    }

    #[test]
    fn build_prompt_includes_diagnostics() {
        let spec = ActionSpec {
            kind: ActionKind::FixDiagnostics,
            instruction: None,
            target_buffer: "buf:test".into(),
            target_content: "fn main() {}".into(),
            surrounding_context: None,
            diagnostics: vec![zaroxi_domain_ai::actions::DiagnosticInfo {
                severity: "ERROR".into(),
                message: "unused variable `x`".into(),
                line: Some(10),
                column: Some(5),
            }],
            file_path: None,
            language: None,
        };
        let prompt = ActionService::build_prompt(&spec, None, None);
        assert!(prompt.contains("Diagnostics"));
        assert!(prompt.contains("ERROR"));
        assert!(prompt.contains("unused variable"));
        assert!(prompt.contains("line 10"));
    }

    #[test]
    fn build_prompt_includes_context_pack() {
        let spec = ActionSpec {
            kind: ActionKind::Edit,
            instruction: Some("optimize".into()),
            target_buffer: "buf:main".into(),
            target_content: "fn main() {}".into(),
            surrounding_context: None,
            diagnostics: vec![],
            file_path: None,
            language: None,
        };
        let ctx = zaroxi_domain_ai::context_ide::ContextPackBuilder::new(1000)
            .add_active_file("main.rs", "fn main() {}", 5)
            .add_workspace_root("/project", "my-crate", 4)
            .build();
        let prompt = ActionService::build_prompt(&spec, Some(&ctx), None);
        assert!(prompt.contains("Context"));
        assert!(prompt.contains("Active File"));
        assert!(prompt.contains("Workspace"));
    }

    #[test]
    fn action_response_has_diff() {
        let resp = ActionResponse {
            action_kind: ActionKind::Edit,
            text: "updated".into(),
            diff: Some(DiffResult {
                buffer_id: "buf:a".into(),
                changes: vec![zaroxi_domain_ai::actions::DiffChange::Insert {
                    index: 0,
                    text: "new".into(),
                }],
                full_replacement: None,
                summary: "test".into(),
            }),
            buffer_id: "buf:a".into(),
        };
        assert!(resp.has_diff());
    }

    #[test]
    fn action_response_no_diff_for_explain() {
        let resp = ActionResponse {
            action_kind: ActionKind::Explain,
            text: "explanation".into(),
            diff: None,
            buffer_id: "buf:a".into(),
        };
        assert!(!resp.has_diff());
    }
}
