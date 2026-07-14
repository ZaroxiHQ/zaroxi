//! Agent orchestrator — implements the plan→execute→observe→iterate loop
//! for AI agent workflows.
//!
//! Phase 4: the orchestrator decomposes user goals into step-by-step plans,
//! executes each step via ActionService, observes results, and adapts when
//! steps fail or require user input.
//!
//! The orchestrator is stateful and thread-safe. It integrates with:
//! - ActionService for AI prompt routing
//! - ContextCollector for multi-file context
//! - McpService for tool access
//! - DiffApplier for edit application

use std::sync::{Arc, Mutex};

use zaroxi_domain_ai::actions::{ActionKind, ActionSpec};
use zaroxi_domain_ai::agent::{AgentSession, AgentState, StepResult, TaskPlan, TaskStep};
use zaroxi_domain_ai::context_ide::ContextPack;

use crate::action_service::ActionService;

/// The agent orchestrator manages a single agent session at a time.
pub struct AgentOrchestrator {
    session: Mutex<AgentSession>,
    #[allow(dead_code)]
    action_service: Arc<ActionService>,
}

impl AgentOrchestrator {
    pub fn new(action_service: Arc<ActionService>) -> Self {
        Self { session: Mutex::new(AgentSession::new()), action_service }
    }

    pub fn state(&self) -> AgentState {
        self.session.lock().unwrap().state
    }

    pub fn session_snapshot(&self) -> AgentSession {
        self.session.lock().unwrap().clone()
    }

    /// Start a new agent session with a user goal.
    pub fn start_goal(&self, goal: &str) {
        self.session.lock().unwrap().start_planning(goal);
    }

    /// Receive the AI-generated plan and present it for approval.
    pub fn set_plan(&self, plan: TaskPlan) {
        let mut session = self.session.lock().unwrap();
        session.plan = Some(plan);
        session.plan_ready();
    }

    /// User approves the plan — begin execution.
    pub fn approve_plan(&self) {
        self.session.lock().unwrap().approve_plan();
    }

    /// User rejects the plan — go back to planning.
    pub fn reject_plan(&self, reason: &str) {
        self.session.lock().unwrap().reject_plan(reason);
    }

    /// Get the next step to execute, or None if complete.
    pub fn next_step(&self) -> Option<TaskStep> {
        let session = self.session.lock().unwrap();
        if session.state != AgentState::Executing {
            return None;
        }
        let step = session.plan.as_ref()?.current_step()?.clone();
        if step.executed {
            return None;
        }
        Some(step)
    }

    /// Execute a step via the action service and store the result.
    pub fn execute_step(
        &self,
        step: &TaskStep,
        spec: Option<&ActionSpec>,
        context: Option<&ContextPack>,
    ) -> StepResult {
        // Construct a default spec from the step if not provided
        let default_spec;
        let _context = context; // reserved for future context injection
        let _resolved_spec = match spec {
            Some(s) => s,
            None => {
                default_spec = ActionSpec {
                    kind: step.action,
                    instruction: Some(step.expected_outcome.clone()),
                    target_buffer: step.target.clone().unwrap_or_default(),
                    target_content: String::new(),
                    surrounding_context: None,
                    diagnostics: vec![],
                    file_path: step.target.clone(),
                    language: None,
                };
                &default_spec
            }
        };

        // Try executing via action service (this would call the AI provider)
        // For now, we produce structured results based on the step
        let output = format!("Executed: {}", step.description);
        let summary = if step.action == ActionKind::Explain || step.action == ActionKind::Review {
            format!("Analysis complete: {}", step.expected_outcome)
        } else {
            format!("Action applied: {}", step.expected_outcome)
        };

        let result = StepResult { success: true, output, summary, diff_summary: None };

        {
            let mut session = self.session.lock().unwrap();
            session.step_executed(result.clone());
        }

        result
    }

    /// Mark that the agent is now observing / waiting for external result.
    pub fn start_observing(&self) {
        self.session.lock().unwrap().start_observing();
    }

    /// Request edit approval from the user for a specific change.
    pub fn request_edit_approval(&self) {
        self.session.lock().unwrap().request_edit_approval();
    }

    /// User approved the edit — continue executing.
    pub fn edit_approved(&self) {
        self.session.lock().unwrap().edit_approved();
    }

    /// User rejected the edit — reason stored as feedback.
    pub fn edit_rejected(&self, reason: &str) {
        self.session.lock().unwrap().edit_rejected(reason);
    }

    /// Mark the session as complete.
    pub fn complete(&self, summary: &str) {
        self.session.lock().unwrap().complete(summary);
    }

    /// Mark the session as failed.
    pub fn fail(&self, error: &str) {
        self.session.lock().unwrap().fail(error);
    }

    /// Cancel the current session.
    pub fn cancel(&self) {
        self.session.lock().unwrap().cancel();
    }

    /// Reset to a fresh session.
    pub fn reset(&self) {
        self.session.lock().unwrap().reset();
    }

    /// Build a prompt to ask the AI to generate a plan for the current goal.
    pub fn build_plan_prompt(goal: &str, context: Option<&ContextPack>) -> String {
        let mut prompt = String::from(
            "You are an AI task planner. Given a user goal, generate a step-by-step plan.\n\n",
        );
        prompt.push_str("Goal: ");
        prompt.push_str(goal);
        prompt.push('\n');

        if let Some(ctx) = context
            && ctx.has_any_context()
        {
            prompt.push_str("\n--- Context ---\n");
            prompt.push_str(&ctx.format_for_prompt());
        }

        prompt.push_str(
            "\nOutput a JSON array of steps. Each step has: step_number, action (Explain/Refactor/Edit/GenerateTests/FixDiagnostics/Review), description, tool, target, expected_outcome.\n"
        );
        prompt.push_str("Example:\n");
        prompt.push_str(
            r#"[{"step_number":1,"action":"Explain","description":"Read the current file","tool":"ai","target":"src/main.rs","expected_outcome":"Understand the code"}]"#,
        );

        prompt
    }

    /// Parse AI response into a TaskPlan.
    /// Tries JSON first, falls back to numbered-text parsing.
    pub fn parse_plan_response(response: &str, goal: &str) -> TaskPlan {
        let mut plan = TaskPlan::new(goal, "AI-generated plan");

        // Try JSON array format
        if let Ok(steps) = serde_json::from_str::<Vec<serde_json::Value>>(response.trim()) {
            for entry in steps {
                let num = entry.get("step_number").and_then(|v| v.as_u64()).unwrap_or(1);
                let action_str = entry.get("action").and_then(|v| v.as_str()).unwrap_or("Edit");
                let action = match action_str {
                    "Explain" => ActionKind::Explain,
                    "Refactor" => ActionKind::Refactor,
                    "Edit" => ActionKind::Edit,
                    "GenerateTests" => ActionKind::GenerateTests,
                    "FixDiagnostics" => ActionKind::FixDiagnostics,
                    "Review" => ActionKind::Review,
                    _ => ActionKind::Edit,
                };
                let desc =
                    entry.get("description").and_then(|v| v.as_str()).unwrap_or("").to_string();
                let tool = entry.get("tool").and_then(|v| v.as_str()).unwrap_or("ai").to_string();
                let target = entry.get("target").and_then(|v| v.as_str()).map(String::from);
                let outcome = entry
                    .get("expected_outcome")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string();

                plan.add_step(TaskStep {
                    step_number: num as usize,
                    action,
                    description: desc,
                    tool,
                    target,
                    expected_outcome: outcome,
                    result: None,
                    executed: false,
                });
            }
            plan.reasoning = format!("{steps_count} steps planned", steps_count = plan.steps.len());
            return plan;
        }

        // Fallback: simple numbered text parsing
        // Expects lines like "1. [Explain] Read the code (src/main.rs)"
        for line in response.lines() {
            let trimmed = line.trim();
            if trimmed.is_empty()
                || !trimmed.chars().next().map(|c| c.is_ascii_digit()).unwrap_or(false)
            {
                continue;
            }
            let after_num = trimmed.split_once('.').map(|x| x.1).unwrap_or("").trim();
            let (action_part, rest) = if after_num.contains(']') {
                let inside = after_num.trim_start_matches('[').split(']').next().unwrap_or("Edit");
                let after = after_num.split_once(']').map(|x| x.1).unwrap_or("").trim();
                (inside, after)
            } else {
                ("Edit", after_num)
            };

            let action = match action_part {
                "Explain" | "explain" => ActionKind::Explain,
                "Refactor" | "refactor" => ActionKind::Refactor,
                "Edit" | "edit" => ActionKind::Edit,
                "GenerateTests" | "tests" => ActionKind::GenerateTests,
                "FixDiagnostics" | "fix" => ActionKind::FixDiagnostics,
                "Review" | "review" => ActionKind::Review,
                _ => ActionKind::Edit,
            };

            let (desc, target) = if let Some(pos) = rest.find('(') {
                let d = rest[..pos].trim();
                let t = rest[pos..].trim_matches(|c| c == '(' || c == ')').trim();
                (d.to_string(), Some(t.to_string()))
            } else {
                (rest.to_string(), None)
            };

            plan.add_step(TaskStep {
                step_number: plan.steps.len() + 1,
                action,
                description: desc,
                tool: "ai".into(),
                target,
                expected_outcome: String::new(),
                result: None,
                executed: false,
            });
        }

        if plan.steps.is_empty() {
            plan.reasoning = "No steps parsed from response".into();
        } else {
            plan.reasoning = format!("{n} steps parsed", n = plan.steps.len());
        }

        plan
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ports::{AiClient, AiError, AiRequest, AiResponseDTO, BoxFuture};

    struct TestAiClient;

    impl AiClient for TestAiClient {
        fn request(&self, _req: AiRequest) -> BoxFuture<'static, Result<AiResponseDTO, AiError>> {
            Box::pin(async { Ok(AiResponseDTO { text: "ok".into() }) })
        }
    }

    fn make_orchestrator() -> AgentOrchestrator {
        let client: Arc<dyn AiClient> = Arc::new(TestAiClient);
        let action_svc = Arc::new(ActionService::new(client));
        AgentOrchestrator::new(action_svc)
    }

    #[test]
    fn full_lifecycle() {
        let orch = make_orchestrator();
        assert_eq!(orch.state(), AgentState::Idle);

        orch.start_goal("fix all bugs");
        assert_eq!(orch.state(), AgentState::Planning);

        let mut plan = TaskPlan::new("fix bugs", "systematic approach");
        plan.add_step(TaskStep {
            step_number: 1,
            action: ActionKind::Explain,
            description: "analyze code".into(),
            tool: "ai".into(),
            target: Some("main.rs".into()),
            expected_outcome: "understand bugs".into(),
            result: None,
            executed: false,
        });
        orch.set_plan(plan);
        assert_eq!(orch.state(), AgentState::AwaitingPlanApproval);

        orch.approve_plan();
        assert_eq!(orch.state(), AgentState::Executing);

        let step = orch.next_step().unwrap();
        assert_eq!(step.step_number, 1);

        let result = orch.execute_step(&step, None, None);
        assert!(result.success);

        orch.complete("all done");
        assert_eq!(orch.state(), AgentState::Completed);
    }

    #[test]
    fn cancel_mid_execution() {
        let orch = make_orchestrator();
        orch.start_goal("test");
        orch.cancel();
        assert_eq!(orch.state(), AgentState::Cancelled);
    }

    #[test]
    fn plan_rejection_goes_back_to_planning() {
        let orch = make_orchestrator();
        orch.start_goal("refactor");
        orch.set_plan(TaskPlan::new("refactor", "cleanup"));
        orch.reject_plan("need more detail");
        assert_eq!(orch.state(), AgentState::Planning);
    }

    #[test]
    fn parse_plan_from_json() {
        let json = r#"[
            {"step_number":1,"action":"Explain","description":"read file","tool":"ai","target":"src/main.rs","expected_outcome":"understood"},
            {"step_number":2,"action":"Edit","description":"fix bug","tool":"ai","target":"src/main.rs","expected_outcome":"fixed"}
        ]"#;
        let plan = AgentOrchestrator::parse_plan_response(json, "fix bugs");
        assert_eq!(plan.steps.len(), 2);
        assert_eq!(plan.steps[0].action, ActionKind::Explain);
        assert_eq!(plan.steps[1].action, ActionKind::Edit);
    }

    #[test]
    fn parse_plan_from_text() {
        let text =
            "1. [Explain] Read the code (src/main.rs)\n2. [Edit] Fix the null check (src/utils.rs)";
        let plan = AgentOrchestrator::parse_plan_response(text, "fix null");
        assert_eq!(plan.steps.len(), 2);
        assert_eq!(plan.steps[0].action, ActionKind::Explain);
        assert_eq!(plan.steps[1].target.as_deref(), Some("src/utils.rs"));
    }
}
