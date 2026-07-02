#![cfg(test)]

use crate::ports::BoxFuture;
use std::sync::Arc;

use zaroxi_application_workspace::ports::{
    ApplyTextTransactionRequest, ApplyTextTransactionResponse, BufferId, ClearSelectionRequest,
    EditorDocument, GetActiveEditorDocumentRequest, GetActiveEditorDocumentResponse,
    GetEditorStateRequest, GetEditorStateResponse, GetRecentCommandsRequest,
    GetRecentEventsRequest, GetVisibleLinesRequest, GetVisibleLinesResponse, ListBuffersRequest,
    ListBuffersResponse, OpenBufferRequest, OpenBufferResponse, SessionId, SetActiveBufferRequest,
    SetEditorCursorRequest, SetSelectionRequest, UpdateBufferRequest, UpdateBufferResponse,
    UseCaseError, WorkspaceBootRequest, WorkspaceBootResponse, WorkspaceService, WorkspaceView,
};
use zaroxi_interface_desktop::desktop::DesktopComposition;
use zaroxi_interface_desktop::desktop::apply_ai_edit_active;
use zaroxi_interface_desktop::desktop::cancel_ai_edit_active;
use zaroxi_interface_desktop::desktop::request_ai_edit_active;
use zaroxi_interface_desktop::ports;
use zaroxi_kernel_types::Id;

struct FakeView {
    doc: EditorDocument,
}

impl FakeView {
    fn new(buffer_id: BufferId, content: Option<String>) -> Self {
        FakeView {
            doc: EditorDocument {
                buffer_id,
                content,
                cursor: ports::EditorCursor::zero(),
                selection: None,
                line_count: 1,
                current_line: None,
            },
        }
    }
}

impl WorkspaceView for FakeView {
    fn get_buffer_content(
        &self,
        _buffer_id: BufferId,
    ) -> BoxFuture<'static, Result<Option<String>, UseCaseError>> {
        let content = self.doc.content.clone();
        Box::pin(async move { Ok(content) })
    }

    fn get_active_buffer_content(
        &self,
        _session_id: SessionId,
    ) -> BoxFuture<'static, Result<Option<String>, UseCaseError>> {
        let content = self.doc.content.clone();
        Box::pin(async move { Ok(content) })
    }

    fn get_active_editor_document(
        &self,
        _req: GetActiveEditorDocumentRequest,
    ) -> BoxFuture<'static, Result<GetActiveEditorDocumentResponse, UseCaseError>> {
        let doc = self.doc.clone();
        Box::pin(async move { Ok(GetActiveEditorDocumentResponse { document: doc }) })
    }

    fn get_visible_lines(
        &self,
        _req: GetVisibleLinesRequest,
    ) -> BoxFuture<'static, Result<GetVisibleLinesResponse, UseCaseError>> {
        Box::pin(async move { Err(UseCaseError::UnknownSession) })
    }
}

#[allow(dead_code)]
struct FakeService {
    last_update: std::sync::Mutex<Option<String>>,
    // store pending AI proposals per-session per-buffer for test determinism
    pending: std::sync::Mutex<
        std::collections::HashMap<
            zaroxi_application_workspace::ports::SessionId,
            std::collections::HashMap<ports::BufferId, String>,
        >,
    >,
}

#[allow(dead_code)]
impl FakeService {
    fn new() -> Self {
        FakeService {
            last_update: std::sync::Mutex::new(None),
            pending: std::sync::Mutex::new(std::collections::HashMap::new()),
        }
    }
}

impl WorkspaceService for FakeService {
    fn boot_workspace(
        &self,
        _req: WorkspaceBootRequest,
    ) -> BoxFuture<'static, Result<WorkspaceBootResponse, UseCaseError>> {
        Box::pin(async move { Err(UseCaseError::UnknownWorkspace) })
    }
    fn open_buffer(
        &self,
        _req: OpenBufferRequest,
    ) -> BoxFuture<'static, Result<OpenBufferResponse, UseCaseError>> {
        Box::pin(async move { Err(UseCaseError::UnknownBuffer) })
    }
    fn list_open_buffers(
        &self,
        _req: ListBuffersRequest,
    ) -> BoxFuture<'static, Result<ListBuffersResponse, UseCaseError>> {
        Box::pin(async move { Err(UseCaseError::UnknownSession) })
    }
    fn set_active_buffer(
        &self,
        _req: SetActiveBufferRequest,
    ) -> BoxFuture<'static, Result<ports::SetActiveBufferResponse, UseCaseError>> {
        Box::pin(async move { Err(UseCaseError::UnknownSession) })
    }
    fn get_active_buffer(
        &self,
        _req: ports::GetActiveBufferRequest,
    ) -> BoxFuture<'static, Result<ports::GetActiveBufferResponse, UseCaseError>> {
        Box::pin(async move { Err(UseCaseError::UnknownSession) })
    }
    fn set_editor_cursor(
        &self,
        _req: SetEditorCursorRequest,
    ) -> BoxFuture<'static, Result<ports::SetEditorCursorResponse, UseCaseError>> {
        Box::pin(async move { Err(UseCaseError::UnknownSession) })
    }
    fn set_editor_selection(
        &self,
        _req: SetSelectionRequest,
    ) -> BoxFuture<'static, Result<ports::SetSelectionResponse, UseCaseError>> {
        Box::pin(async move { Err(UseCaseError::UnknownSession) })
    }
    fn clear_editor_selection(
        &self,
        _req: ClearSelectionRequest,
    ) -> BoxFuture<'static, Result<ports::ClearSelectionResponse, UseCaseError>> {
        Box::pin(async move { Err(UseCaseError::UnknownSession) })
    }
    fn get_editor_state(
        &self,
        _req: GetEditorStateRequest,
    ) -> BoxFuture<'static, Result<GetEditorStateResponse, UseCaseError>> {
        Box::pin(async move { Err(UseCaseError::UnknownSession) })
    }
    fn set_viewport_state(
        &self,
        _req: ports::SetViewportRequest,
    ) -> BoxFuture<'static, Result<ports::SetViewportResponse, UseCaseError>> {
        Box::pin(async move { Err(UseCaseError::UnknownSession) })
    }
    fn scroll_viewport(
        &self,
        _req: ports::ScrollViewportRequest,
    ) -> BoxFuture<'static, Result<ports::ScrollViewportResponse, UseCaseError>> {
        Box::pin(async move { Err(UseCaseError::UnknownSession) })
    }
    fn explain_active_buffer(
        &self,
        _req: ports::GetActiveBufferRequest,
    ) -> BoxFuture<'static, Result<ports::DispatchCommandResponse, UseCaseError>> {
        Box::pin(async move { Err(UseCaseError::UnknownSession) })
    }
    fn dispatch_command(
        &self,
        _req: ports::DispatchCommandRequest,
    ) -> BoxFuture<'static, Result<ports::DispatchCommandResponse, UseCaseError>> {
        Box::pin(async move { Err(UseCaseError::UnknownSession) })
    }
    fn update_buffer(
        &self,
        req: UpdateBufferRequest,
    ) -> BoxFuture<'static, Result<UpdateBufferResponse, UseCaseError>> {
        let mut guard = self.last_update.lock().unwrap();
        *guard = Some(req.new_content.clone());
        Box::pin(async move { Ok(UpdateBufferResponse { ok: true }) })
    }
    fn apply_text_transaction(
        &self,
        _req: ApplyTextTransactionRequest,
    ) -> BoxFuture<'static, Result<ApplyTextTransactionResponse, UseCaseError>> {
        Box::pin(async move { Err(UseCaseError::UnknownSession) })
    }
    fn get_recent_commands(
        &self,
        _req: GetRecentCommandsRequest,
    ) -> BoxFuture<'static, Result<ports::GetRecentCommandsResponse, UseCaseError>> {
        Box::pin(async move { Ok(ports::GetRecentCommandsResponse { commands: Vec::new() }) })
    }
    fn get_recent_events(
        &self,
        _req: GetRecentEventsRequest,
    ) -> BoxFuture<'static, Result<ports::GetRecentEventsResponse, UseCaseError>> {
        Box::pin(async move { Ok(ports::GetRecentEventsResponse { events: Vec::new() }) })
    }
    fn get_session_snapshot(
        &self,
        _req: ports::GetSessionSnapshotRequest,
    ) -> BoxFuture<'static, Result<ports::GetSessionSnapshotResponse, UseCaseError>> {
        Box::pin(async move { Err(UseCaseError::UnknownSession) })
    }
    fn create_checkpoint(
        &self,
        _req: ports::CreateCheckpointRequest,
    ) -> BoxFuture<'static, Result<ports::CreateCheckpointResponse, UseCaseError>> {
        Box::pin(async move { Err(UseCaseError::UnknownSession) })
    }
    fn save_checkpoint(
        &self,
        _req: ports::SaveCheckpointRequest,
    ) -> BoxFuture<'static, Result<ports::SaveCheckpointResponse, UseCaseError>> {
        Box::pin(async move { Err(UseCaseError::UnknownSession) })
    }
    fn load_checkpoint(
        &self,
        _req: ports::LoadCheckpointRequest,
    ) -> BoxFuture<'static, Result<ports::LoadCheckpointResponse, UseCaseError>> {
        Box::pin(async move { Err(UseCaseError::UnknownSession) })
    }
    fn restore_checkpoint(
        &self,
        _req: ports::RestoreCheckpointRequest,
    ) -> BoxFuture<'static, Result<ports::RestoreCheckpointResponse, UseCaseError>> {
        Box::pin(async move { Err(UseCaseError::UnknownSession) })
    }

    // Phase 10: application-level AI orchestration API (test mock implementations).
    fn request_ai_edit(
        &self,
        req: crate::ports::RequestAiEditRequest,
    ) -> BoxFuture<'static, Result<crate::ports::RequestAiEditResponse, UseCaseError>> {
        let proposal =
            format!("// AI Edit: proposed change\n{}", req.content.clone().unwrap_or_default());
        // store pending proposal keyed by session and buffer
        {
            let mut p = self.pending.lock().unwrap();
            let sess = p.entry(req.session_id.clone()).or_default();
            sess.insert(req.buffer_id.clone(), proposal.clone());
        }
        let resp = crate::ports::RequestAiEditResponse {
            proposal: crate::ports::AiProposal {
                target_buffer: req.buffer_id.clone(),
                proposal_text: proposal.clone(),
                summary: Some("AI edit proposed".to_string()),
            },
        };
        Box::pin(async move { Ok(resp) })
    }

    fn apply_ai_edit(
        &self,
        req: crate::ports::ApplyAiEditRequest,
    ) -> BoxFuture<'static, Result<crate::ports::ApplyAiEditResponse, UseCaseError>> {
        // For test double: accept the provided proposal text and record it as the last update.
        // This makes the request->apply flow deterministic in tests without requiring a full
        // session reconciliation step inside the fake service.
        let mut guard = self.last_update.lock().unwrap();
        *guard = Some(req.proposal_text.clone());
        Box::pin(async move { Ok(crate::ports::ApplyAiEditResponse { ok: true }) })
    }

    fn cancel_ai_edit(
        &self,
        _req: crate::ports::CancelAiEditRequest,
    ) -> BoxFuture<'static, Result<crate::ports::CancelAiEditResponse, UseCaseError>> {
        Box::pin(async move { Ok(crate::ports::CancelAiEditResponse { ok: true }) })
    }

    fn attempt_close_session(
        &self,
        _req: ports::GetSessionSnapshotRequest,
    ) -> BoxFuture<'static, Result<ports::GetSessionSnapshotResponse, UseCaseError>> {
        Box::pin(async move { Err(UseCaseError::UnknownSession) })
    }
    fn resolve_close_session_save_all(
        &self,
        _req: ports::SaveCheckpointRequest,
    ) -> BoxFuture<'static, Result<ports::SaveCheckpointResponse, UseCaseError>> {
        Box::pin(async move { Err(UseCaseError::UnknownSession) })
    }
    fn resolve_close_session_discard_all(
        &self,
        _req: ports::SaveCheckpointRequest,
    ) -> BoxFuture<'static, Result<ports::SaveCheckpointResponse, UseCaseError>> {
        Box::pin(async move { Ok(ports::SaveCheckpointResponse { location: String::new() }) })
    }
}

#[tokio::test]
async fn ai_request_and_apply_flow() {
    // Create composition and a fake view that references a known buffer id.
    let mut comp = DesktopComposition::new();
    let buf_path = std::path::Path::new("file1.txt");
    let buf_id = ports::BufferId::from_path(buf_path);
    let view = Arc::new(FakeView::new(buf_id.clone(), Some("original content".to_string())));

    // Build a small orchestrator-backed service so request/apply touch a real buffer store.
    use std::path::PathBuf;
    use zaroxi_application_ai::ports as ai_ports;
    use zaroxi_application_workspace::usecases::WorkspaceOrchestrator;
    use zaroxi_core_editor_buffer::ports as buffer_ports;
    use zaroxi_domain_workspace::ports as domain_ports;

    // Minimal infra fakes to compose an orchestrator instance for the test.
    struct TestRepo;
    impl domain_ports::WorkspaceRepository for TestRepo {
        fn open_workspace(
            &self,
            _cmd: domain_ports::WorkspaceOpenCommand,
        ) -> crate::ports::BoxFuture<
            'static,
            Result<domain_ports::WorkspaceDTO, domain_ports::DomainError>,
        > {
            Box::pin(async move {
                Ok(domain_ports::WorkspaceDTO {
                    id: Id::new(),
                    root_path: PathBuf::from("."),
                    name: "test".to_string(),
                })
            })
        }
    }

    struct TestBufferStore;
    impl buffer_ports::BufferStore for TestBufferStore {
        fn open_buffer(
            &self,
            path: PathBuf,
        ) -> crate::ports::BoxFuture<
            'static,
            Result<buffer_ports::BufferId, buffer_ports::BufferError>,
        > {
            let id = buffer_ports::BufferId::from_path(&path);
            Box::pin(async move { Ok(id) })
        }
        fn get_text(&self, _id: &buffer_ports::BufferId) -> Option<String> {
            Some("original content".to_string())
        }
        fn set_text(
            &self,
            _id: &buffer_ports::BufferId,
            _content: String,
        ) -> crate::ports::BoxFuture<'static, Result<(), buffer_ports::BufferError>> {
            Box::pin(async move { Ok(()) })
        }
        fn apply_transaction(
            &self,
            _id: &buffer_ports::BufferId,
            _txn: buffer_ports::TextEdit,
        ) -> crate::ports::BoxFuture<'static, Result<(), buffer_ports::BufferError>> {
            Box::pin(async move { Ok(()) })
        }
    }

    struct TestAi;
    impl ai_ports::AiClient for TestAi {
        fn request(
            &self,
            req: ai_ports::AiRequest,
        ) -> ai_ports::BoxFuture<'static, Result<ai_ports::AiResponseDTO, ai_ports::AiError>>
        {
            let buf = req.buffer_id.clone();
            Box::pin(async move {
                Ok(ai_ports::AiResponseDTO { text: format!("fake-explain: {}", buf) })
            })
        }
    }

    let repo =
        std::sync::Arc::new(TestRepo) as std::sync::Arc<dyn domain_ports::WorkspaceRepository>;
    let buf_store =
        std::sync::Arc::new(TestBufferStore) as std::sync::Arc<dyn buffer_ports::BufferStore>;
    let ai_client = std::sync::Arc::new(TestAi) as std::sync::Arc<dyn ai_ports::AiClient>;

    let orch = WorkspaceOrchestrator::new(repo, buf_store, ai_client);
    let service_arc: std::sync::Arc<dyn crate::ports::WorkspaceService> = std::sync::Arc::new(orch);

    // Boot workspace and open the buffer via the orchestrator so a session and buffer membership exist.
    let boot = crate::ports::WorkspaceBootRequest { path: PathBuf::from(".") };
    let boot_res = service_arc.boot_workspace(boot).await.expect("boot ok");
    let session_id = boot_res.session.session_id.clone();

    let open_req = crate::ports::OpenBufferRequest {
        session_id: session_id.clone(),
        path: PathBuf::from("file1.txt"),
    };
    let _open_res = service_arc.open_buffer(open_req).await.expect("open ok");

    // Request AI edit (application orchestrator stores authoritative proposal).
    let req_res = request_ai_edit_active(
        &mut comp,
        view.clone(),
        session_id.clone(),
        Some(service_arc.clone()),
    )
    .await;
    assert!(req_res.is_ok(), "request_ai_edit_active failed: {:?}", req_res);

    // Ensure ai_projection is present and proposed.
    let md = comp.latest_metadata().expect("metadata expected");
    let ai = md.ai_projection.expect("ai projection expected");
    assert_eq!(ai.state, Some(zaroxi_interface_desktop::desktop::AiState::Proposed));
    assert!(ai.proposal_text.is_some());

    // Apply the proposal using the same orchestrator service.
    let apply_res = apply_ai_edit_active(
        &mut comp,
        view.clone(),
        session_id.clone(),
        Some(boot_res.session.workspace_id),
        service_arc.clone(),
    )
    .await;
    assert!(apply_res.is_ok(), "apply_ai_edit_active failed: {:?}", apply_res);

    // After apply, projection should be Applied.
    let md2 = comp.latest_metadata().expect("metadata expected after apply");
    let ai2 = md2.ai_projection.expect("ai projection expected after apply");
    assert_eq!(ai2.state, Some(zaroxi_interface_desktop::desktop::AiState::Applied));
}

#[tokio::test]
async fn ai_cancel_clears_proposal() {
    let mut comp = DesktopComposition::new();
    let buf_id = ports::BufferId::from_path(std::path::Path::new("file2.txt"));
    let view = Arc::new(FakeView::new(buf_id.clone(), Some("something".to_string())));
    let session_id = SessionId(Id::new());

    let _ = request_ai_edit_active(&mut comp, view.clone(), session_id.clone(), None).await;
    assert!(comp.latest_metadata().and_then(|m| m.ai_projection).is_some());

    cancel_ai_edit_active(&mut comp, None, None);
    assert!(comp.latest_metadata().and_then(|m| m.ai_projection).is_none());
}

/// Phase 2: after requesting an AI edit, the AI panel content view is populated
/// from the proposal and the transcript reflects proposal-backed text (not idle).
#[tokio::test]
async fn ai_proposal_populates_content_view_and_transcript() {
    use zaroxi_interface_desktop::gui::{ShellFrame, Size};

    let mut comp = DesktopComposition::new();
    let buf_id = ports::BufferId::from_path(std::path::Path::new("src/lib.rs"));
    let view = Arc::new(FakeView::new(buf_id.clone(), Some("fn main() {}".to_string())));
    let session_id = SessionId(Id::new());
    let fake_svc = Arc::new(FakeService::new());

    // Request an AI edit — this populates ai_projection and ai_panel_content_view.
    let res =
        request_ai_edit_active(&mut comp, view.clone(), session_id.clone(), Some(fake_svc)).await;
    assert!(res.is_ok(), "request_ai_edit_active failed: {:?}", res);

    // The content view should be present and carry proposal text (via build_work_content).
    let work = comp.build_work_content();
    let cv = work.ai_panel_content.expect("ai_panel_content must be populated after proposal");
    assert_eq!(cv.title, "Assistant");
    assert!(cv.subtitle.contains("Proposal for"));
    assert!(
        cv.lines.iter().any(|l| l.contains("AI Edit: proposed change")),
        "proposal body must appear in content lines"
    );
    assert!(
        cv.lines.iter().any(|l| l.contains("Accept") && l.contains("Reject")),
        "action labels must appear in content lines"
    );

    // Transcript: render the shell with composition attached.
    let shell = ShellFrame::new(
        Size { width: 1280, height: 800 },
        zaroxi_interface_theme::theme::ZaroxiTheme::Dark,
    );
    let transcript = shell.render_lines(Some(&comp));
    let joined = transcript.join("\n");

    // Idle placeholder must NOT appear.
    assert!(
        !joined.contains("No active AI session"),
        "idle subtitle must not appear when proposal is live"
    );
    // Proposal-backed content must appear.
    assert!(joined.contains("Proposal for"), "transcript must include proposal subtitle");
    assert!(
        joined.contains("AI Edit: proposed change"),
        "transcript must include proposal body text"
    );
    assert!(
        joined.contains("Accept") && joined.contains("Reject") && joined.contains("Edit"),
        "transcript must include action labels Accept/Reject/Edit"
    );
}

/// Phase 19: AI review / apply / reject flow via the command bar.
#[tokio::test]
async fn ai_commands_review_apply_reject_via_command_bar() {
    use zaroxi_application_workspace::workspace_view::command_bar_labels;
    use zaroxi_interface_desktop::actions::{execute_command_by_index, open_command_bar};

    let labels = command_bar_labels();
    assert!(labels.contains(&"AI review active buffer".to_string()));
    assert!(labels.contains(&"Apply AI proposal".to_string()));
    assert!(labels.contains(&"Reject AI proposal".to_string()));

    let mut comp = DesktopComposition::new();
    let buf_id = ports::BufferId::from_path(std::path::Path::new("src/lib.rs"));
    let view = Arc::new(FakeView::new(buf_id.clone(), Some("fn main() {}".to_string())));
    let session_id = SessionId(Id::new());
    let fake_svc = Arc::new(FakeService::new());

    // Open the command bar so labels are available.
    let _ = open_command_bar(&mut comp).await;

    // Find the AI review command index.
    let review_idx = labels.iter().position(|l| l == "AI review active buffer").unwrap();

    // Request AI review via command bar.
    let res = execute_command_by_index(
        &mut comp,
        view.clone(),
        Some(fake_svc.clone()),
        session_id.clone(),
        None,
        review_idx,
    )
    .await
    .unwrap();
    assert!(res.success, "AI review should succeed: {:?}", res.message);

    let proj = comp.latest_metadata().and_then(|m| m.ai_projection.clone());
    assert!(proj.is_some(), "ai_projection should be set after AI review");
    let proj = proj.unwrap();
    assert!(proj.proposal_text.is_some(), "proposal text should be populated");

    // AI panel content in work content should show proposal
    let work = comp.build_work_content();
    let ai_content = work.ai_panel_content.expect("ai_panel_content in work content");
    assert!(ai_content.subtitle.contains("Proposal for"));
    assert!(ai_content.lines.iter().any(|l| l.contains("Accept")));

    // Apply
    let apply_idx = labels.iter().position(|l| l == "Apply AI proposal").unwrap();
    let res = execute_command_by_index(
        &mut comp,
        view.clone(),
        Some(fake_svc.clone()),
        session_id.clone(),
        None,
        apply_idx,
    )
    .await
    .unwrap();
    assert!(res.success, "apply should succeed");

    let applied_text = fake_svc.last_update.lock().unwrap().clone();
    assert!(applied_text.is_some(), "applied text should be recorded");

    // Phase 23: after apply, AI panel should show applied state.
    let work_after = comp.build_work_content();
    let ai_after = work_after.ai_panel_content.expect("ai panel content after apply");
    assert!(
        ai_after.subtitle.contains("Applied:"),
        "should show Applied: subtitle after apply, got: {}",
        ai_after.subtitle
    );

    // Re-request then reject
    let reject_idx = labels.iter().position(|l| l == "Reject AI proposal").unwrap();
    execute_command_by_index(
        &mut comp,
        view.clone(),
        Some(fake_svc.clone()),
        session_id.clone(),
        None,
        review_idx,
    )
    .await
    .unwrap();
    execute_command_by_index(
        &mut comp,
        view.clone(),
        Some(fake_svc.clone()),
        session_id,
        None,
        reject_idx,
    )
    .await
    .unwrap();
    assert!(comp.latest_metadata().and_then(|m| m.ai_projection).is_none());
}

/// Phase 20: explain via command bar exercises the dispatch path.
#[tokio::test]
async fn explain_via_command_bar_status() {
    use zaroxi_application_workspace::workspace_view::command_bar_labels;
    use zaroxi_interface_desktop::actions::{execute_command_by_index, open_command_bar};

    let labels = command_bar_labels();
    let explain_idx = labels.iter().position(|l| l == "Explain active buffer").unwrap();

    let mut comp = DesktopComposition::new();
    let buf_id = ports::BufferId::from_path(std::path::Path::new("src/main.rs"));
    let view = Arc::new(FakeView::new(buf_id.clone(), Some("fn main() {}".to_string())));
    let session_id = SessionId(Id::new());
    let fake_svc = Arc::new(FakeService::new());

    let _ = open_command_bar(&mut comp).await;
    let res = execute_command_by_index(
        &mut comp,
        view.clone(),
        Some(fake_svc),
        session_id,
        None,
        explain_idx,
    )
    .await;
    assert!(res.is_ok(), "explain command bar dispatch should complete");
}
