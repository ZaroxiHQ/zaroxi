 use std::sync::{Arc, Mutex};
 use std::collections::HashMap;

 use crate::ports::{
     WorkspaceBootRequest, WorkspaceBootResponse, OpenBufferRequest, OpenBufferResponse,
     UpdateBufferRequest, UpdateBufferResponse,
     DispatchCommandRequest, DispatchCommandResponse, AppCommand, CommandResult, WorkspaceSessionDTO,
     ListBuffersRequest, ListBuffersResponse, SetActiveBufferRequest, SetActiveBufferResponse,
     GetActiveBufferRequest, GetActiveBufferResponse,
     BoxFuture, UseCaseError,
     CommandRecord, CommandKind, WorkspaceEvent, WorkspaceEventKind, DynHistoryRepository,
     GetRecentCommandsRequest, GetRecentCommandsResponse, GetRecentEventsRequest, GetRecentEventsResponse,
     SessionId, WorkspaceId,
 };
 
 use zaroxi_domain_workspace::ports as domain_ports;
 use zaroxi_core_editor_buffer::ports as buffer_ports;
 use zaroxi_application_ai::ports as ai_ports;
 use zaroxi_kernel_types::Id;
 use chrono::Utc;
 use uuid::Uuid;
 
 /// Concrete orchestrator implementing application use-cases.
 ///
 /// This struct belongs to the application layer. It composes domain and core ports,
 /// delegating side-effects to adapters provided by the composition root.
 pub struct WorkspaceOrchestrator {
     repo: Arc<dyn domain_ports::WorkspaceRepository>,
     buffer_store: Arc<dyn buffer_ports::BufferStore>,
     ai_client: Arc<dyn ai_ports::AiClient>,
     /// Optional history repository for recording commands and events.
     history: Arc<dyn DynHistoryRepositoryMarker + Send + Sync>,
     /// In-memory session -> session info mapping for the simple slice.
     sessions: Arc<Mutex<HashMap<Id, SessionInfo>>>,
 }

 /// Per-session minimal state owned by the application orchestrator.
 #[derive(Clone, Debug)]
 struct SessionInfo {
     workspace_id: Id,
     open_buffers: Vec<String>,     // list of buffer ids opened in this session (order of opening)
     active_buffer: Option<String>, // currently selected buffer id
 }
 
 use crate::ports::BoxFuture;
 use crate::ports::UseCaseError;
 use zaroxi_domain_buffer::rules as buffer_rules;

 // Define a thin marker trait so we can hold a dyn without exposing the concrete HistoryRepository
 // implementation details here; the real trait is defined in ports and dyn object is passed through.
 pub trait DynHistoryRepositoryMarker {
     fn record_command_box(&self, rec: CommandRecord) -> BoxFuture<'static, Result<(), String>>;
     fn record_event_box(&self, ev: WorkspaceEvent) -> BoxFuture<'static, Result<(), String>>;
     fn get_recent_commands_box(&self, session_id: SessionId, limit: usize) -> BoxFuture<'static, Result<Vec<CommandRecord>, String>>;
     fn get_recent_events_box(&self, session_id: SessionId, limit: usize) -> BoxFuture<'static, Result<Vec<WorkspaceEvent>, String>>;
 }

 // Adapter impl: forward to the real ports::HistoryRepository trait object when constructing the orchestrator.
 impl<T: crate::ports::HistoryRepository + ?Sized> DynHistoryRepositoryMarker for T {
     fn record_command_box(&self, rec: CommandRecord) -> BoxFuture<'static, Result<(), String>> {
         self.record_command(rec)
     }
     fn record_event_box(&self, ev: WorkspaceEvent) -> BoxFuture<'static, Result<(), String>> {
         self.record_event(ev)
     }
     fn get_recent_commands_box(&self, session_id: SessionId, limit: usize) -> BoxFuture<'static, Result<Vec<CommandRecord>, String>> {
         self.get_recent_commands(session_id, limit)
     }
     fn get_recent_events_box(&self, session_id: SessionId, limit: usize) -> BoxFuture<'static, Result<Vec<WorkspaceEvent>, String>> {
         self.get_recent_events(session_id, limit)
     }
 }

 impl WorkspaceOrchestrator {
     /// Create a new orchestrator with concrete port implementations (adapters).
     /// This legacy constructor uses a no-op history recorder so existing tests remain unchanged.
     pub fn new(
         repo: Arc<dyn domain_ports::WorkspaceRepository>,
         buffer_store: Arc<dyn buffer_ports::BufferStore>,
         ai_client: Arc<dyn ai_ports::AiClient>,
     ) -> Self {
         // default no-op history repository
         let noop = NoopHistory::new();
         Self { repo, buffer_store, ai_client, history: Arc::new(noop), sessions: Arc::new(Mutex::new(HashMap::new())) }
     }

     /// Create a new orchestrator with an explicit history repository (for harness/infra composition).
     pub fn new_with_history(
         repo: Arc<dyn domain_ports::WorkspaceRepository>,
         buffer_store: Arc<dyn buffer_ports::BufferStore>,
         ai_client: Arc<dyn ai_ports::AiClient>,
         history: Arc<dyn crate::ports::HistoryRepository>,
     ) -> Self {
         Self { repo, buffer_store, ai_client, history: history as Arc<dyn DynHistoryRepositoryMarker + Send + Sync>, sessions: Arc::new(Mutex::new(HashMap::new())) }
     }
 }

 /// A lightweight no-op history recorder (used by tests that don't provide a history impl).
 struct NoopHistory;

 impl NoopHistory {
     fn new() -> Self { NoopHistory }
 }

 impl DynHistoryRepositoryMarker for NoopHistory {
     fn record_command_box(&self, _rec: CommandRecord) -> BoxFuture<'static, Result<(), String>> {
         Box::pin(async { Ok(()) })
     }
     fn record_event_box(&self, _ev: WorkspaceEvent) -> BoxFuture<'static, Result<(), String>> {
         Box::pin(async { Ok(()) })
     }
     fn get_recent_commands_box(&self, _session_id: SessionId, _limit: usize) -> BoxFuture<'static, Result<Vec<CommandRecord>, String>> {
         Box::pin(async { Ok(Vec::new()) })
     }
     fn get_recent_events_box(&self, _session_id: SessionId, _limit: usize) -> BoxFuture<'static, Result<Vec<WorkspaceEvent>, String>> {
         Box::pin(async { Ok(Vec::new()) })
     }
 }

 impl crate::ports::WorkspaceService for WorkspaceOrchestrator {
     fn boot_workspace(&self, req: WorkspaceBootRequest) -> BoxFuture<'static, Result<WorkspaceBootResponse, UseCaseError>> {
         let repo = self.repo.clone();
         let sessions = self.sessions.clone();
         let history = self.history.clone();
         Box::pin(async move {
             let domain_cmd = domain_ports::WorkspaceOpenCommand { path: req.path.clone() };
             let dto = repo.open_workspace(domain_cmd).await.map_err(|_e| UseCaseError::UnknownWorkspace)?;
             // Create a session id for this UI session.
             let session_id = Id::new();
             // Store session info: workspace id, empty buffer list.
             {
                 let mut s = sessions.lock().unwrap();
                 s.insert(session_id, SessionInfo { workspace_id: dto.id, open_buffers: Vec::new(), active_buffer: None });
             }
             let session = WorkspaceSessionDTO { session_id: crate::ports::SessionId(session_id), workspace_id: dto.id };

             // Record command and event
             let cmd = CommandRecord {
                 id: Uuid::new_v4(),
                 timestamp: Utc::now(),
                 kind: CommandKind::BootWorkspace { path: req.path.clone() },
                 session_id: Some(session.session_id.clone()),
                 workspace_id: Some(dto.id),
                 buffer_id: None,
                 success: true,
                 result: Some("workspace opened".to_string()),
                 error: None,
             };
             let _ = history.record_command_box(cmd).await;

             let ev = WorkspaceEvent {
                 id: Uuid::new_v4(),
                 timestamp: Utc::now(),
                 session_id: session.session_id.clone(),
                 workspace_id: dto.id,
                 kind: WorkspaceEventKind::SessionOpened { session_id: session.session_id.clone(), workspace_id: dto.id },
             };
             let _ = history.record_event_box(ev).await;

             Ok(WorkspaceBootResponse { session })
         })
     }

     fn open_buffer(&self, req: OpenBufferRequest) -> BoxFuture<'static, Result<OpenBufferResponse, UseCaseError>> {
         let store = self.buffer_store.clone();
         let sessions = self.sessions.clone();
         let history = self.history.clone();
         Box::pin(async move {
             // Validate session exists
             {
                 let s = sessions.lock().unwrap();
                 if !s.contains_key(&req.session_id.0) {
                     // record failed command
                     let cmd = CommandRecord {
                         id: Uuid::new_v4(),
                         timestamp: Utc::now(),
                         kind: CommandKind::OpenBuffer { path: req.path.clone() },
                         session_id: Some(req.session_id.clone()),
                         workspace_id: None,
                         buffer_id: None,
                         success: false,
                         result: None,
                         error: Some("unknown session".to_string()),
                     };
                     let _ = history.record_command_box(cmd).await;
                     return Err(UseCaseError::UnknownSession);
                 }
             }

             // Ask underlying store to open buffer
             let id = store.open_buffer(req.path.clone()).await.map_err(|_e| {
                 // record failed command
                 let cmd = CommandRecord {
                     id: Uuid::new_v4(),
                     timestamp: Utc::now(),
                     kind: CommandKind::OpenBuffer { path: req.path.clone() },
                     session_id: Some(req.session_id.clone()),
                     workspace_id: None,
                     buffer_id: None,
                     success: false,
                     result: None,
                     error: Some("unknown buffer".to_string()),
                 };
                 let _ = history.record_command_box(cmd);
                 UseCaseError::UnknownBuffer
             })?;
             let buffer_id = id.0.clone();

             // Register buffer in session and set active if first
             let workspace_id_opt = {
                 let mut s = sessions.lock().unwrap();
                 if let Some(info) = s.get_mut(&req.session_id.0) {
                     info.open_buffers.push(buffer_id.clone());
                     if info.active_buffer.is_none() {
                         info.active_buffer = Some(buffer_id.clone());
                     }
                     Some(info.workspace_id)
                 } else {
                     None
                 }
             };

             // Record success command and event
             let cmd = CommandRecord {
                 id: Uuid::new_v4(),
                 timestamp: Utc::now(),
                 kind: CommandKind::OpenBuffer { path: req.path.clone() },
                 session_id: Some(req.session_id.clone()),
                 workspace_id: workspace_id_opt.map(|id| id),
                 buffer_id: Some(buffer_id.clone()),
                 success: true,
                 result: Some(format!("opened {}", buffer_id)),
                 error: None,
             };
             let _ = history.record_command_box(cmd).await;

             if let Some(ws) = workspace_id_opt {
                 let ev = WorkspaceEvent {
                     id: Uuid::new_v4(),
                     timestamp: Utc::now(),
                     session_id: req.session_id.clone(),
                     workspace_id: ws,
                     kind: WorkspaceEventKind::BufferOpened { buffer_id: buffer_id.clone(), path: req.path.clone() },
                 };
                 let _ = history.record_event_box(ev).await;
             }

             Ok(OpenBufferResponse { buffer_id })
         })
     }

     fn list_open_buffers(&self, req: ListBuffersRequest) -> BoxFuture<'static, Result<ListBuffersResponse, UseCaseError>> {
         let sessions = self.sessions.clone();
         Box::pin(async move {
             let s = sessions.lock().unwrap();
             let info = s.get(&req.session_id.0).ok_or(UseCaseError::UnknownSession)?;
             Ok(ListBuffersResponse { buffer_ids: info.open_buffers.clone(), active_buffer: info.active_buffer.clone() })
         })
     }

     fn set_active_buffer(&self, req: SetActiveBufferRequest) -> BoxFuture<'static, Result<SetActiveBufferResponse, UseCaseError>> {
         let sessions = self.sessions.clone();
         let history = self.history.clone();
         Box::pin(async move {
             let mut s = sessions.lock().unwrap();
             let info = s.get_mut(&req.session_id.0).ok_or(UseCaseError::UnknownSession)?;
             // Ensure requested buffer was opened in this session
             if !info.open_buffers.iter().any(|b| b == &req.buffer_id) {
                 // record failure
                 let cmd = CommandRecord {
                     id: Uuid::new_v4(),
                     timestamp: Utc::now(),
                     kind: CommandKind::SetActiveBuffer { buffer_id: req.buffer_id.clone() },
                     session_id: Some(req.session_id.clone()),
                     workspace_id: Some(info.workspace_id),
                     buffer_id: Some(req.buffer_id.clone()),
                     success: false,
                     result: None,
                     error: Some("invalid active buffer".to_string()),
                 };
                 let _ = history.record_command_box(cmd).await;
                 return Err(UseCaseError::InvalidActiveBuffer(req.buffer_id));
             }
             let old = info.active_buffer.clone();
             info.active_buffer = Some(req.buffer_id.clone());

             // record success command and event
             let cmd = CommandRecord {
                 id: Uuid::new_v4(),
                 timestamp: Utc::now(),
                 kind: CommandKind::SetActiveBuffer { buffer_id: req.buffer_id.clone() },
                 session_id: Some(req.session_id.clone()),
                 workspace_id: Some(info.workspace_id),
                 buffer_id: Some(req.buffer_id.clone()),
                 success: true,
                 result: Some("active buffer set".to_string()),
                 error: None,
             };
             let _ = history.record_command_box(cmd).await;

             let ev = WorkspaceEvent {
                 id: Uuid::new_v4(),
                 timestamp: Utc::now(),
                 session_id: req.session_id.clone(),
                 workspace_id: info.workspace_id,
                 kind: WorkspaceEventKind::ActiveBufferChanged { old, new: info.active_buffer.clone() },
             };
             let _ = history.record_event_box(ev).await;

             Ok(SetActiveBufferResponse { ok: true })
         })
     }

     fn get_active_buffer(&self, req: GetActiveBufferRequest) -> BoxFuture<'static, Result<GetActiveBufferResponse, UseCaseError>> {
         let sessions = self.sessions.clone();
         Box::pin(async move {
             let s = sessions.lock().unwrap();
             let info = s.get(&req.session_id.0).ok_or(UseCaseError::UnknownSession)?;
             match &info.active_buffer {
                 Some(b) => Ok(GetActiveBufferResponse { buffer_id: b.clone() }),
                 None => Err(UseCaseError::NoActiveBuffer),
             }
         })
     }

     fn explain_active_buffer(&self, req: GetActiveBufferRequest) -> BoxFuture<'static, Result<DispatchCommandResponse, UseCaseError>> {
         let ai = self.ai_client.clone();
         let store = self.buffer_store.clone();
         let sessions = self.sessions.clone();
         let history = self.history.clone();
         Box::pin(async move {
             // Resolve active buffer id
             let (active, workspace_id) = {
                 let s = sessions.lock().unwrap();
                 let info = s.get(&req.session_id.0).ok_or(UseCaseError::UnknownSession)?;
                 (info.active_buffer.clone().ok_or(UseCaseError::NoActiveBuffer)?, info.workspace_id)
             };

             // Snapshot content for the AI request.
             let buf_id = buffer_ports::BufferId(active.clone());
             let content = store.get_text(&buf_id).unwrap_or_else(|| "".to_string());
             if content.trim().is_empty() {
                 // record failure
                 let cmd = CommandRecord {
                     id: Uuid::new_v4(),
                     timestamp: Utc::now(),
                     kind: CommandKind::ExplainActiveBuffer,
                     session_id: Some(req.session_id.clone()),
                     workspace_id: Some(workspace_id),
                     buffer_id: Some(active.clone()),
                     success: false,
                     result: None,
                     error: Some("missing buffer content for explain".to_string()),
                 };
                 let _ = history.record_command_box(cmd).await;
                 return Err(UseCaseError::AiFailure("missing buffer content for explain".to_string()));
             }

             let ai_req = ai_ports::AiRequest {
                 session_id: req.session_id.0,
                 workspace_id,
                 buffer_id: active.clone(),
                 content_snapshot: content.clone(),
             };
             let res = ai.request(ai_req).await.map_err(|_e| {
                 // record failure
                 let cmd = CommandRecord {
                     id: Uuid::new_v4(),
                     timestamp: Utc::now(),
                     kind: CommandKind::ExplainActiveBuffer,
                     session_id: Some(req.session_id.clone()),
                     workspace_id: Some(workspace_id),
                     buffer_id: Some(active.clone()),
                     success: false,
                     result: None,
                     error: Some("ai request failed".to_string()),
                 };
                 let _ = history.record_command_box(cmd);
                 UseCaseError::AiFailure("ai request failed".to_string())
             })?;

             // record success and event
             let cmd = CommandRecord {
                 id: Uuid::new_v4(),
                 timestamp: Utc::now(),
                 kind: CommandKind::ExplainActiveBuffer,
                 session_id: Some(req.session_id.clone()),
                 workspace_id: Some(workspace_id),
                 buffer_id: Some(active.clone()),
                 success: true,
                 result: Some(res.text.clone()),
                 error: None,
             };
             let _ = history.record_command_box(cmd).await;

             let ev = WorkspaceEvent {
                 id: Uuid::new_v4(),
                 timestamp: Utc::now(),
                 session_id: req.session_id.clone(),
                 workspace_id,
                 kind: WorkspaceEventKind::ExplainExecuted { buffer_id: active.clone(), result: res.text.clone() },
             };
             let _ = history.record_event_box(ev).await;

             Ok(DispatchCommandResponse { result: CommandResult { message: res.text } })
         })
     }

     fn dispatch_command(&self, req: DispatchCommandRequest) -> BoxFuture<'static, Result<DispatchCommandResponse, UseCaseError>> {
         let ai = self.ai_client.clone();
         let store = self.buffer_store.clone();
         let sessions = self.sessions.clone();
         let history = self.history.clone();
         Box::pin(async move {
             // Validate session exists
             let workspace_id = {
                 let s = sessions.lock().unwrap();
                 match s.get(&req.session_id.0) {
                     Some(w) => w.workspace_id,
                     None => {
                         // record failed dispatch
                         let cmd = CommandRecord {
                             id: Uuid::new_v4(),
                             timestamp: Utc::now(),
                             kind: CommandKind::DispatchAppCommand { command: req.command.clone() },
                             session_id: Some(req.session_id.clone()),
                             workspace_id: None,
                             buffer_id: None,
                             success: false,
                             result: None,
                             error: Some("unknown session".to_string()),
                         };
                         let _ = history.record_command_box(cmd).await;
                         return Err(UseCaseError::UnknownSession)
                     },
                 }
             };

             match req.command {
                 AppCommand::AiExplain { buffer_id } => {
                     // Snapshot content for the AI request.
                     let buf_id = buffer_ports::BufferId(buffer_id.clone());
                     let content = store.get_text(&buf_id).unwrap_or_else(|| "".to_string());
                     if content.trim().is_empty() {
                         // record failed dispatch
                         let cmd = CommandRecord {
                             id: Uuid::new_v4(),
                             timestamp: Utc::now(),
                             kind: CommandKind::DispatchAppCommand { command: AppCommand::AiExplain { buffer_id: buffer_id.clone() } },
                             session_id: Some(req.session_id.clone()),
                             workspace_id: Some(workspace_id),
                             buffer_id: Some(buffer_id.clone()),
                             success: false,
                             result: None,
                             error: Some("missing buffer content for explain".to_string()),
                         };
                         let _ = history.record_command_box(cmd).await;
                         return Err(UseCaseError::AiFailure("missing buffer content for explain".to_string()));
                     }
                     let ai_req = ai_ports::AiRequest {
                         session_id: req.session_id.0,
                         workspace_id,
                         buffer_id: buffer_id.clone(),
                         content_snapshot: content,
                     };
                     let res = ai.request(ai_req).await.map_err(|_e| {
                         let cmd = CommandRecord {
                             id: Uuid::new_v4(),
                             timestamp: Utc::now(),
                             kind: CommandKind::DispatchAppCommand { command: AppCommand::AiExplain { buffer_id: buffer_id.clone() } },
                             session_id: Some(req.session_id.clone()),
                             workspace_id: Some(workspace_id),
                             buffer_id: Some(buffer_id.clone()),
                             success: false,
                             result: None,
                             error: Some("ai request failed".to_string()),
                         };
                         let _ = history.record_command_box(cmd);
                         UseCaseError::AiFailure("ai request failed".to_string())
                     })?;
                     // record success
                     let cmd = CommandRecord {
                         id: Uuid::new_v4(),
                         timestamp: Utc::now(),
                         kind: CommandKind::DispatchAppCommand { command: AppCommand::AiExplain { buffer_id: buffer_id.clone() } },
                         session_id: Some(req.session_id.clone()),
                         workspace_id: Some(workspace_id),
                         buffer_id: Some(buffer_id.clone()),
                         success: true,
                         result: Some(res.text.clone()),
                         error: None,
                     };
                     let _ = history.record_command_box(cmd).await;

                     Ok(DispatchCommandResponse { result: CommandResult { message: res.text } })
                 }
                 AppCommand::InsertText { .. } => {
                     // Not implemented in Phase 5; return a successful no-op.
                     // record noop command
                     let cmd = CommandRecord {
                         id: Uuid::new_v4(),
                         timestamp: Utc::now(),
                         kind: CommandKind::DispatchAppCommand { command: req.command.clone() },
                         session_id: Some(req.session_id.clone()),
                         workspace_id: Some(workspace_id),
                         buffer_id: None,
                         success: true,
                         result: Some("inserted (noop)".to_string()),
                         error: None,
                     };
                     let _ = history.record_command_box(cmd).await;
                     Ok(DispatchCommandResponse { result: CommandResult { message: "inserted (noop)".to_string() } })
                 }
             }
         })
     }

     fn update_buffer(&self, req: UpdateBufferRequest) -> BoxFuture<'static, Result<UpdateBufferResponse, UseCaseError>> {
         let store = self.buffer_store.clone();
         let sessions = self.sessions.clone();
         let history = self.history.clone();
         Box::pin(async move {
             // Validate session is known.
             {
                 let s = sessions.lock().unwrap();
                 if !s.contains_key(&req.session_id.0) {
                     // record failed update
                     let cmd = CommandRecord {
                         id: Uuid::new_v4(),
                         timestamp: Utc::now(),
                         kind: CommandKind::UpdateBuffer { buffer_id: req.buffer_id.clone() },
                         session_id: Some(req.session_id.clone()),
                         workspace_id: None,
                         buffer_id: Some(req.buffer_id.clone()),
                         success: false,
                         result: None,
                         error: Some("unknown session".to_string()),
                     };
                     let _ = history.record_command_box(cmd).await;
                     return Err(UseCaseError::UnknownSession);
                 }
             }

             // Validate buffer id and content via domain rules.
             if let Err(m) = buffer_rules::validate_buffer_id(&req.buffer_id) {
                 let cmd = CommandRecord {
                     id: Uuid::new_v4(),
                     timestamp: Utc::now(),
                     kind: CommandKind::UpdateBuffer { buffer_id: req.buffer_id.clone() },
                     session_id: Some(req.session_id.clone()),
                     workspace_id: None,
                     buffer_id: Some(req.buffer_id.clone()),
                     success: false,
                     result: None,
                     error: Some(m.clone()),
                 };
                 let _ = history.record_command_box(cmd).await;
                 return Err(UseCaseError::InvalidMutation(m));
             }
             if let Err(m) = buffer_rules::validate_content(&req.new_content) {
                 let cmd = CommandRecord {
                     id: Uuid::new_v4(),
                     timestamp: Utc::now(),
                     kind: CommandKind::UpdateBuffer { buffer_id: req.buffer_id.clone() },
                     session_id: Some(req.session_id.clone()),
                     workspace_id: None,
                     buffer_id: Some(req.buffer_id.clone()),
                     success: false,
                     result: None,
                     error: Some(m.clone()),
                 };
                 let _ = history.record_command_box(cmd).await;
                 return Err(UseCaseError::InvalidMutation(m));
             }

             // Perform the mutation via BufferStore (infra).
             store.set_text(&buffer_ports::BufferId(req.buffer_id.clone()), req.new_content.clone())
                 .await
                 .map_err(|_e| {
                     // record failed mutation
                     let cmd = CommandRecord {
                         id: Uuid::new_v4(),
                         timestamp: Utc::now(),
                         kind: CommandKind::UpdateBuffer { buffer_id: req.buffer_id.clone() },
                         session_id: Some(req.session_id.clone()),
                         workspace_id: None,
                         buffer_id: Some(req.buffer_id.clone()),
                         success: false,
                         result: None,
                         error: Some("unknown buffer".to_string()),
                     };
                     let _ = history.record_command_box(cmd);
                     UseCaseError::UnknownBuffer
                 })?;

             // record success and event
             let workspace_id_opt = {
                 let s = sessions.lock().unwrap();
                 s.get(&req.session_id.0).map(|i| i.workspace_id)
             };

             let cmd = CommandRecord {
                 id: Uuid::new_v4(),
                 timestamp: Utc::now(),
                 kind: CommandKind::UpdateBuffer { buffer_id: req.buffer_id.clone() },
                 session_id: Some(req.session_id.clone()),
                 workspace_id: workspace_id_opt,
                 buffer_id: Some(req.buffer_id.clone()),
                 success: true,
                 result: Some("updated".to_string()),
                 error: None,
             };
             let _ = history.record_command_box(cmd).await;

             if let Some(ws) = workspace_id_opt {
                 let ev = WorkspaceEvent {
                     id: Uuid::new_v4(),
                     timestamp: Utc::now(),
                     session_id: req.session_id.clone(),
                     workspace_id: ws,
                     kind: WorkspaceEventKind::BufferUpdated { buffer_id: req.buffer_id.clone() },
                 };
                 let _ = history.record_event_box(ev).await;
             }

             Ok(UpdateBufferResponse { ok: true })
         })
     }

     fn get_recent_commands(&self, req: GetRecentCommandsRequest) -> BoxFuture<'static, Result<GetRecentCommandsResponse, UseCaseError>> {
         let history = self.history.clone();
         Box::pin(async move {
             let recs = history.get_recent_commands_box(req.session_id.clone(), req.limit).await.map_err(|_e| UseCaseError::AiFailure("history query failed".to_string()))?;
             Ok(GetRecentCommandsResponse { commands: recs })
         })
     }

     fn get_recent_events(&self, req: GetRecentEventsRequest) -> BoxFuture<'static, Result<GetRecentEventsResponse, UseCaseError>> {
         let history = self.history.clone();
         Box::pin(async move {
             let evs = history.get_recent_events_box(req.session_id.clone(), req.limit).await.map_err(|_e| UseCaseError::AiFailure("history query failed".to_string()))?;
             Ok(GetRecentEventsResponse { events: evs })
         })
     }
 }
 
 #[cfg(test)]
 mod tests {
     use super::*;
     use crate::ports::WorkspaceService;
     use std::sync::Arc;
     use std::path::PathBuf;
 
     // Lightweight test doubles implementing the required ports.
     struct FakeRepo;
     impl domain_ports::WorkspaceRepository for FakeRepo {
         fn open_workspace(&self, cmd: domain_ports::WorkspaceOpenCommand) -> crate::ports::BoxFuture<'static, Result<domain_ports::WorkspaceDTO, domain_ports::DomainError>> {
             Box::pin(async move {
                 Ok(domain_ports::WorkspaceDTO { id: Id::new(), root_path: cmd.path.clone(), name: "TestWS".to_string() })
             })
         }
     }
 
     struct FakeBufferStore;
     impl buffer_ports::BufferStore for FakeBufferStore {
         fn open_buffer(&self, path: PathBuf) -> crate::ports::BoxFuture<'static, Result<buffer_ports::BufferId, buffer_ports::BufferError>> {
             Box::pin(async move {
                 Ok(buffer_ports::BufferId(format!("buf:{}", path.to_string_lossy())))
             })
         }
    
         fn get_text(&self, _id: &buffer_ports::BufferId) -> Option<String> {
             Some("fn main() {}".to_string())
         }

         fn set_text(&self, id: &buffer_ports::BufferId, _content: String) -> crate::ports::BoxFuture<'static, Result<(), buffer_ports::BufferError>> {
             let key = id.0.clone();
             Box::pin(async move {
                 // Lightweight fake behavior: accept writes for any buffer id that looks like a BufferId produced by open_buffer.
                 if key.starts_with("buf:") {
                     Ok(())
                 } else {
                     Err(buffer_ports::BufferError("buffer not found".to_string()))
                 }
             })
         }
     }
 
     struct FakeAi;
     impl ai_ports::AiClient for FakeAi {
         fn request(&self, req: ai_ports::AiRequest) -> ai_ports::BoxFuture<'static, Result<ai_ports::AiResponseDTO, ai_ports::AiError>> {
             Box::pin(async move {
                 Ok(ai_ports::AiResponseDTO { text: format!("fake-explain: {}", req.buffer_id) })
             })
         }
     }
 
     #[tokio::test]
     async fn end_to_end_usecase_flow() {
         let repo = Arc::new(FakeRepo) as Arc<dyn domain_ports::WorkspaceRepository>;
         let buffer = Arc::new(FakeBufferStore) as Arc<dyn buffer_ports::BufferStore>;
         let ai = Arc::new(FakeAi) as Arc<dyn ai_ports::AiClient>;
 
         let orch = WorkspaceOrchestrator::new(repo, buffer, ai);
 
         // Boot workspace
         let boot = WorkspaceBootRequest { path: PathBuf::from("./sample") };
         let boot_res = orch.boot_workspace(boot).await.expect("boot ok");
         // session id is now typed; ensure it's present.
         assert!(boot_res.session.session_id.0.to_string().len() > 0);
 
         // Open buffer
         let open = OpenBufferRequest { session_id: boot_res.session.session_id.clone(), path: PathBuf::from("main.rs") };
         let open_res = orch.open_buffer(open).await.expect("open ok");
         assert!(open_res.buffer_id.starts_with("buf:"));
 
         // Dispatch AI explain
         let cmd = DispatchCommandRequest { session_id: boot_res.session.session_id.clone(), command: AppCommand::AiExplain { buffer_id: open_res.buffer_id.clone() } };
         let cmd_res = orch.dispatch_command(cmd).await.expect("dispatch ok");
         assert!(cmd_res.result.message.contains("fake-explain"));
     }
 }
