 use std::sync::{Arc, Mutex};
 use std::collections::HashMap;
 
 use crate::ports::{
     WorkspaceBootRequest, WorkspaceBootResponse, OpenBufferRequest, OpenBufferResponse,
     UpdateBufferRequest, UpdateBufferResponse,
     DispatchCommandRequest, DispatchCommandResponse, AppCommand, CommandResult, WorkspaceSessionDTO,
     ListBuffersRequest, ListBuffersResponse, SetActiveBufferRequest, SetActiveBufferResponse,
     GetActiveBufferRequest, GetActiveBufferResponse,
     BoxFuture, UseCaseError,
     // Transaction seam (Phase 4)
     ApplyTextTransactionRequest, ApplyTextTransactionResponse, TextEdit,
     CommandRecord, CommandKind, WorkspaceEvent, WorkspaceEventKind,
     GetRecentCommandsRequest, GetRecentCommandsResponse, GetRecentEventsRequest, GetRecentEventsResponse,
     // Snapshot/query types (Phase 7)
     GetSessionSnapshotRequest, GetSessionSnapshotResponse, SessionSnapshot, BufferSnapshot,
     // Phase 8 checkpoint types
     CreateCheckpointRequest, CreateCheckpointResponse, RestoreCheckpointRequest, RestoreCheckpointResponse, Checkpoint,
     // Editor-state types (Phase 3)
     EditorCursor, EditorState, SetEditorCursorRequest, SetEditorCursorResponse, SetSelectionRequest, SetSelectionResponse,
     ClearSelectionRequest, ClearSelectionResponse, GetEditorStateRequest, GetEditorStateResponse,
     // Viewport/viewport APIs (Phase 8)
     SetViewportRequest, SetViewportResponse, ScrollViewportRequest, ScrollViewportResponse,
     SessionId,
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
     history: Arc<dyn crate::ports::HistoryRepository>,
     /// Durability adapter for persisting checkpoints.
     durability: Arc<dyn crate::ports::DurabilityRepository>,
     /// In-memory session -> session info mapping for the simple slice.
     sessions: Arc<Mutex<HashMap<Id, SessionInfo>>>,
 }

 /// Per-session minimal state owned by the application orchestrator.
 #[derive(Clone, Debug)]
 struct SessionInfo {
     workspace_id: Id,
     open_buffers: Vec<buffer_ports::BufferId>,     // list of buffer ids opened in this session (order of opening)
     active_buffer: Option<buffer_ports::BufferId>, // currently selected buffer id
     /// Editor transient state per buffer (cursor + optional selection).
     editor_states: std::collections::HashMap<buffer_ports::BufferId, EditorState>,
     /// Per-buffer viewport state (line-based). Managed by the orchestrator.
     viewport_states: std::collections::HashMap<buffer_ports::BufferId, crate::ports::ViewportState>,
 }
 
 use zaroxi_domain_buffer::rules as buffer_rules;

 /// Internal helper: apply a typed text transaction using the orchestrator logic.
 ///
 /// This function contains the core apply_text_transaction implementation that
 /// mutates editor transient state, delegates the actual edit to the BufferStore,
 /// and records history/events. It mirrors the previous method body but is
 /// factored out so other use-cases (dispatching typed edit commands) can call
 /// it without capturing `&self`.
 fn apply_text_transaction_impl(
     store: Arc<dyn buffer_ports::BufferStore>,
     sessions: Arc<Mutex<HashMap<Id, SessionInfo>>>,
     history: Arc<dyn crate::ports::HistoryRepository>,
     req: ApplyTextTransactionRequest,
 ) -> BoxFuture<'static, Result<ApplyTextTransactionResponse, UseCaseError>> {
     Box::pin(async move {
         // Validate session exists
         let session_known = { let s = sessions.lock().unwrap(); s.contains_key(&req.session_id.0) };
         if !session_known {
             let cmd = CommandRecord::new_failure(
                 CommandKind::UpdateBuffer { buffer_id: req.buffer_id.clone() },
                 Some(req.session_id.0),
                 None,
                 Some(req.buffer_id.clone()),
                 Some("unknown session".to_string()),
             );
             let _ = history.record_command(cmd).await;
             return Err(UseCaseError::UnknownSession);
         }

         // Ensure buffer is opened in session and prepare editor-state mutation
         {
             // Short-lived membership check: acquire lock only to test membership and immediately drop it.
             let is_open = {
                 let s = sessions.lock().unwrap();
                 s.get(&req.session_id.0).map(|info| info.open_buffers.iter().any(|b| b == &req.buffer_id)).unwrap_or(false)
             };

             if !is_open {
                 let cmd = CommandRecord::new_failure(
                     CommandKind::UpdateBuffer { buffer_id: req.buffer_id.clone() },
                     Some(req.session_id.0),
                     None,
                     Some(req.buffer_id.clone()),
                     Some("invalid active buffer".to_string()),
                 );
                 // history.record_command may await; do this while NOT holding the sessions lock.
                 let _ = history.record_command(cmd).await;
                 return Err(UseCaseError::InvalidActiveBuffer(req.buffer_id.to_string()));
             }

             // Now acquire the lock again briefly to mutate editor-state.
             let mut s = sessions.lock().unwrap();
             let info = s.get_mut(&req.session_id.0).ok_or(UseCaseError::UnknownSession)?;

             // Simple cursor/selection update policy (Phase 4 minimal):
             // We treat EditorCursor.column as a flat character index while line==0.
             // - Insert at the cursor index advances the cursor by inserted chars.
             // - Delete shifts cursor left if it was after the deleted range.
             // - Replace adjusts cursor depending on whether it was inside replaced region.
             let entry = info.editor_states.entry(req.buffer_id.clone()).or_insert(EditorState { cursor: EditorCursor::zero(), selection: None });
             let mut new_cursor = entry.cursor.clone();
             match &req.transaction {
                 TextEdit::Insert { index, text } => {
                     if entry.cursor.line == 0 && entry.cursor.column as usize == *index {
                         new_cursor.column = new_cursor.column.saturating_add(text.chars().count() as u32);
                     }
                     entry.selection = None;
                     entry.cursor = new_cursor.clone();
                 }
                 TextEdit::Delete { start, end } => {
                     let del_len = if *end > *start { *end - *start } else { 0 };
                     if entry.cursor.line == 0 {
                         let cidx = entry.cursor.column as usize;
                         if cidx > *start {
                             let sub = std::cmp::min(cidx - *start, del_len);
                             new_cursor.column = new_cursor.column.saturating_sub(sub as u32);
                         }
                     }
                     entry.selection = None;
                     entry.cursor = new_cursor.clone();
                 }
                 TextEdit::Replace { start, end, text } => {
                     let old_len = if *end > *start { *end - *start } else { 0 };
                     let new_len = text.chars().count();
                     if entry.cursor.line == 0 {
                         let cidx = entry.cursor.column as usize;
                         if cidx >= *start && cidx <= *end {
                             // cursor inside replaced region -> move to end of replacement
                             new_cursor.column = (*start + new_len) as u32;
                         } else if cidx > *end {
                             if new_len >= old_len {
                                 new_cursor.column = new_cursor.column.saturating_add((new_len - old_len) as u32);
                             } else {
                                 new_cursor.column = new_cursor.column.saturating_sub((old_len - new_len) as u32);
                             }
                         }
                     }
                     entry.selection = None;
                     entry.cursor = new_cursor.clone();
                 }
             }
         }

         // Apply the transaction to the buffer via the BufferStore
         if let Err(_e) = store.apply_transaction(&req.buffer_id, req.transaction.clone()).await {
             let cmd = CommandRecord::new_failure(
                 CommandKind::UpdateBuffer { buffer_id: req.buffer_id.clone() },
                 Some(req.session_id.0),
                 None,
                 Some(req.buffer_id.clone()),
                 Some("unknown buffer".to_string()),
             );
             let _ = history.record_command(cmd).await;
             return Err(UseCaseError::UnknownBuffer);
         }

         // record success and event
         let workspace_id_opt = {
             let s = sessions.lock().unwrap();
             s.get(&req.session_id.0).map(|i| i.workspace_id)
         };

         let cmd = CommandRecord::new_success(
             CommandKind::UpdateBuffer { buffer_id: req.buffer_id.clone() },
             Some(req.session_id.0),
             workspace_id_opt,
             Some(req.buffer_id.clone()),
             Some("transaction applied".to_string()),
         );
         let _ = history.record_command(cmd).await;

         if let Some(ws) = workspace_id_opt {
             let ev = WorkspaceEvent {
                 id: Uuid::new_v4(),
                 timestamp: Utc::now(),
                 session_id: req.session_id.clone(),
                 workspace_id: ws,
                 kind: WorkspaceEventKind::BufferUpdated { buffer_id: req.buffer_id.clone() },
             };
             let _ = history.record_event(ev).await;
         }

         // Read updated content and editor-state to return
         let content = store.get_text(&req.buffer_id);
         let state = {
             let s = sessions.lock().unwrap();
             s.get(&req.session_id.0).and_then(|info| info.editor_states.get(&req.buffer_id).cloned())
         };

         Ok(ApplyTextTransactionResponse { ok: true, state: state.unwrap_or(EditorState { cursor: EditorCursor::zero(), selection: None }), content })
     })
 }

 impl WorkspaceOrchestrator {
     /// Create a new orchestrator with concrete port implementations (adapters).
     /// This legacy constructor uses a no-op history recorder so existing tests remain unchanged.
     pub fn new(
         repo: Arc<dyn domain_ports::WorkspaceRepository>,
         buffer_store: Arc<dyn buffer_ports::BufferStore>,
         ai_client: Arc<dyn ai_ports::AiClient>,
     ) -> Self {
         // default no-op history repository and no-op durability
         let noop = NoopHistory::new();
         let noop_dur = NoopDurability::new();
         Self { repo, buffer_store, ai_client, history: Arc::new(noop), durability: Arc::new(noop_dur), sessions: Arc::new(Mutex::new(HashMap::new())) }
     }

     /// Create a new orchestrator with an explicit history repository (for harness/infra composition).
     pub fn new_with_history(
         repo: Arc<dyn domain_ports::WorkspaceRepository>,
         buffer_store: Arc<dyn buffer_ports::BufferStore>,
         ai_client: Arc<dyn ai_ports::AiClient>,
         history: Arc<dyn crate::ports::HistoryRepository>,
     ) -> Self {
         // default no-op durability when one is not provided
         let noop_dur = NoopDurability::new();
         Self { repo, buffer_store, ai_client, history, durability: Arc::new(noop_dur), sessions: Arc::new(Mutex::new(HashMap::new())) }
     }
 
     /// Create a new orchestrator with an explicit history repository and durability adapter.
     pub fn new_with_history_and_durability(
         repo: Arc<dyn domain_ports::WorkspaceRepository>,
         buffer_store: Arc<dyn buffer_ports::BufferStore>,
         ai_client: Arc<dyn ai_ports::AiClient>,
         history: Arc<dyn crate::ports::HistoryRepository>,
         durability: Arc<dyn crate::ports::DurabilityRepository>,
     ) -> Self {
         Self { repo, buffer_store, ai_client, history, durability, sessions: Arc::new(Mutex::new(HashMap::new())) }
     }
 }

 /// A lightweight no-op history recorder (used by tests that don't provide a history impl).
 struct NoopHistory;

 impl NoopHistory {
     fn new() -> Self { NoopHistory }
 }
 
 impl crate::ports::HistoryRepository for NoopHistory {
     fn record_command(&self, _rec: CommandRecord) -> BoxFuture<'static, Result<(), String>> {
         Box::pin(async { Ok(()) })
     }
     fn record_event(&self, _ev: WorkspaceEvent) -> BoxFuture<'static, Result<(), String>> {
         Box::pin(async { Ok(()) })
     }
     fn get_recent_commands(&self, _session_id: SessionId, _limit: usize) -> BoxFuture<'static, Result<Vec<CommandRecord>, String>> {
         Box::pin(async { Ok(Vec::new()) })
     }
     fn get_recent_events(&self, _session_id: SessionId, _limit: usize) -> BoxFuture<'static, Result<Vec<WorkspaceEvent>, String>> {
         Box::pin(async { Ok(Vec::new()) })
     }
 }
 
 /// No-op durability adapter used when none is provided at composition time.
 struct NoopDurability;
 
 impl NoopDurability {
     fn new() -> Self { NoopDurability }
 }
 
 impl crate::ports::DurabilityRepository for NoopDurability {
     fn save_checkpoint(&self, _checkpoint: crate::ports::Checkpoint) -> BoxFuture<'static, Result<String, crate::ports::DurabilityError>> {
         Box::pin(async { Err(crate::ports::DurabilityError::Io("noop durability not configured".to_string())) })
     }
     fn load_checkpoint(&self, _location: String) -> BoxFuture<'static, Result<crate::ports::Checkpoint, crate::ports::DurabilityError>> {
         Box::pin(async { Err(crate::ports::DurabilityError::NotFound("noop durability not configured".to_string())) })
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
                 s.insert(session_id, SessionInfo { workspace_id: dto.id, open_buffers: Vec::new(), active_buffer: None, editor_states: std::collections::HashMap::new(), viewport_states: std::collections::HashMap::new() });
             }
             let session = WorkspaceSessionDTO { session_id: crate::ports::SessionId(session_id), workspace_id: dto.id };

             // Record command and event
             let cmd = CommandRecord::new_success(
                 CommandKind::BootWorkspace { path: req.path.clone() },
                 Some(session.session_id.0),
                 Some(dto.id),
                 None,
                 Some("workspace opened".to_string()),
             );
             let _ = history.record_command(cmd).await;

             let ev = WorkspaceEvent {
                 id: Uuid::new_v4(),
                 timestamp: Utc::now(),
                 session_id: session.session_id.clone(),
                 workspace_id: dto.id,
                 kind: WorkspaceEventKind::SessionOpened { session_id: session.session_id.clone(), workspace_id: dto.id },
             };
             let _ = history.record_event(ev).await;

             Ok(WorkspaceBootResponse { session })
         })
     }

     fn open_buffer(&self, req: OpenBufferRequest) -> BoxFuture<'static, Result<OpenBufferResponse, UseCaseError>> {
         let store = self.buffer_store.clone();
         let sessions = self.sessions.clone();
         let history = self.history.clone();
         Box::pin(async move {
             // Validate session exists (release lock before awaiting)
             let session_exists = {
                 let s = sessions.lock().unwrap();
                 s.contains_key(&req.session_id.0)
             };
             if !session_exists {
                 // record failed command
                 let cmd = CommandRecord {
                     id: Uuid::new_v4(),
                     timestamp: Utc::now(),
                     kind: CommandKind::OpenBuffer { path: req.path.clone() },
                     session_id: Some(req.session_id.0),
                     workspace_id: None,
                     buffer_id: None,
                     success: false,
                     result: None,
                     error: Some("unknown session".to_string()),
                 };
                 let _ = history.record_command(cmd).await;
                 return Err(UseCaseError::UnknownSession);
             }

             // Ask underlying store to open buffer (handle infra failure explicitly)
             let id = match store.open_buffer(req.path.clone()).await {
                 Ok(id) => id,
                 Err(_e) => {
                     // record failed command
                     let cmd = CommandRecord::new_failure(
                         CommandKind::OpenBuffer { path: req.path.clone() },
                         Some(req.session_id.0),
                         None,
                         None,
                         Some("unknown buffer".to_string()),
                     );
                     let _ = history.record_command(cmd).await;
                     return Err(UseCaseError::UnknownBuffer);
                 }
             };
             let buffer_id = id.clone();
 
             // Register buffer in session and set active if first
             let workspace_id_opt = {
                 let mut s = sessions.lock().unwrap();
                 if let Some(info) = s.get_mut(&req.session_id.0) {
                     info.open_buffers.push(buffer_id.clone());
                     // initialize lightweight editor state for this buffer as empty cursor at (0,0)
                     info.editor_states.entry(buffer_id.clone()).or_insert(EditorState { cursor: EditorCursor::zero(), selection: None });
                     // initialize a default viewport for this buffer (1-based top_line).
                     info.viewport_states.entry(buffer_id.clone()).or_insert(crate::ports::ViewportState { top_line: 1, window_height: 10, center_cursor: true });
                     if info.active_buffer.is_none() {
                         info.active_buffer = Some(buffer_id.clone());
                     }
                     Some(info.workspace_id)
                 } else {
                     None
                 }
             };

             // Record success command and event
             let cmd = CommandRecord::new_success(
                 CommandKind::OpenBuffer { path: req.path.clone() },
                 Some(req.session_id.0),
                 workspace_id_opt,
                 Some(buffer_id.clone()),
                 Some(format!("opened {}", buffer_id)),
             );
             let _ = history.record_command(cmd).await;

             if let Some(ws) = workspace_id_opt {
                 let ev = WorkspaceEvent {
                     id: Uuid::new_v4(),
                     timestamp: Utc::now(),
                     session_id: req.session_id.clone(),
                     workspace_id: ws,
                     kind: WorkspaceEventKind::BufferOpened { buffer_id: buffer_id.clone(), path: req.path.clone() },
                 };
                 let _ = history.record_event(ev).await;
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
             // Validate membership without holding lock across awaits
             let invalid = {
                 let s = sessions.lock().unwrap();
                 match s.get(&req.session_id.0) {
                     Some(info) => !info.open_buffers.iter().any(|b| b == &req.buffer_id),
                     None => return Err(UseCaseError::UnknownSession),
                 }
             };
             if invalid {
                 // record failure
                 let cmd = CommandRecord::new_failure(
                     CommandKind::SetActiveBuffer { buffer_id: req.buffer_id.clone() },
                     Some(req.session_id.0),
                     None,
                     Some(req.buffer_id.clone()),
                     Some("invalid active buffer".to_string()),
                 );
                 let _ = history.record_command(cmd).await;
                 return Err(UseCaseError::InvalidActiveBuffer(req.buffer_id.to_string()));
             }

             // Perform mutation while holding the lock briefly and capture old/new/ws
             let (old, workspace_id) = {
                 let mut s = sessions.lock().unwrap();
                 let info = s.get_mut(&req.session_id.0).ok_or(UseCaseError::UnknownSession)?;
                 let old = info.active_buffer.clone();
                 info.active_buffer = Some(req.buffer_id.clone());
                 (old, info.workspace_id)
             };

             // record success command and event
             let cmd = CommandRecord::new_success(
                 CommandKind::SetActiveBuffer { buffer_id: req.buffer_id.clone() },
                 Some(req.session_id.0),
                 Some(workspace_id),
                 Some(req.buffer_id.clone()),
                 Some("active buffer set".to_string()),
             );
             let _ = history.record_command(cmd).await;

             let ev = WorkspaceEvent {
                 id: Uuid::new_v4(),
                 timestamp: Utc::now(),
                 session_id: req.session_id.clone(),
                 workspace_id,
                 kind: WorkspaceEventKind::ActiveBufferChanged { old, new: Some(req.buffer_id.clone()) },
             };
             let _ = history.record_event(ev).await;

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

     fn set_editor_cursor(&self, req: SetEditorCursorRequest) -> BoxFuture<'static, Result<SetEditorCursorResponse, UseCaseError>> {
         let sessions = self.sessions.clone();
         Box::pin(async move {
             // validate session and membership
             let mut s = sessions.lock().unwrap();
             let info = s.get_mut(&req.session_id.0).ok_or(UseCaseError::UnknownSession)?;
             if !info.open_buffers.iter().any(|b| b == &req.buffer_id) {
                 return Err(UseCaseError::InvalidActiveBuffer(req.buffer_id.to_string()));
             }
             let entry = info.editor_states.entry(req.buffer_id.clone()).or_insert(EditorState { cursor: EditorCursor::zero(), selection: None });
             entry.cursor = req.cursor.clone();
             Ok(SetEditorCursorResponse { ok: true })
         })
     }

     fn set_editor_selection(&self, req: SetSelectionRequest) -> BoxFuture<'static, Result<SetSelectionResponse, UseCaseError>> {
         let sessions = self.sessions.clone();
         Box::pin(async move {
             let mut s = sessions.lock().unwrap();
             let info = s.get_mut(&req.session_id.0).ok_or(UseCaseError::UnknownSession)?;
             if !info.open_buffers.iter().any(|b| b == &req.buffer_id) {
                 return Err(UseCaseError::InvalidActiveBuffer(req.buffer_id.to_string()));
             }
             let entry = info.editor_states.entry(req.buffer_id.clone()).or_insert(EditorState { cursor: EditorCursor::zero(), selection: None });
             entry.selection = Some(req.selection.clone());
             Ok(SetSelectionResponse { ok: true })
         })
     }

     fn clear_editor_selection(&self, req: ClearSelectionRequest) -> BoxFuture<'static, Result<ClearSelectionResponse, UseCaseError>> {
         let sessions = self.sessions.clone();
         Box::pin(async move {
             let mut s = sessions.lock().unwrap();
             let info = s.get_mut(&req.session_id.0).ok_or(UseCaseError::UnknownSession)?;
             if !info.open_buffers.iter().any(|b| b == &req.buffer_id) {
                 return Err(UseCaseError::InvalidActiveBuffer(req.buffer_id.to_string()));
             }
             if let Some(entry) = info.editor_states.get_mut(&req.buffer_id) {
                 entry.selection = None;
             }
             Ok(ClearSelectionResponse { ok: true })
         })
     }

     fn get_editor_state(&self, req: GetEditorStateRequest) -> BoxFuture<'static, Result<GetEditorStateResponse, UseCaseError>> {
         let sessions = self.sessions.clone();
         Box::pin(async move {
             let s = sessions.lock().unwrap();
             let info = s.get(&req.session_id.0).ok_or(UseCaseError::UnknownSession)?;
             if !info.open_buffers.iter().any(|b| b == &req.buffer_id) {
                 return Err(UseCaseError::InvalidActiveBuffer(req.buffer_id.to_string()));
             }
             let state = info.editor_states.get(&req.buffer_id).cloned();
             Ok(GetEditorStateResponse { state })
         })
     }
 
     fn set_viewport_state(&self, req: SetViewportRequest) -> BoxFuture<'static, Result<SetViewportResponse, UseCaseError>> {
         let sessions = self.sessions.clone();
         Box::pin(async move {
             // Validate session and membership
             let mut s = sessions.lock().unwrap();
             let info = s.get_mut(&req.session_id.0).ok_or(UseCaseError::UnknownSession)?;
             if !info.open_buffers.iter().any(|b| b == &req.buffer_id) {
                 return Err(UseCaseError::InvalidActiveBuffer(req.buffer_id.to_string()));
             }
             info.viewport_states.insert(req.buffer_id.clone(), req.viewport.clone());
             Ok(SetViewportResponse { ok: true })
         })
     }
 
     fn scroll_viewport(&self, req: ScrollViewportRequest) -> BoxFuture<'static, Result<ScrollViewportResponse, UseCaseError>> {
         let sessions = self.sessions.clone();
         Box::pin(async move {
             let mut s = sessions.lock().unwrap();
             let info = s.get_mut(&req.session_id.0).ok_or(UseCaseError::UnknownSession)?;
             if !info.open_buffers.iter().any(|b| b == &req.buffer_id) {
                 return Err(UseCaseError::InvalidActiveBuffer(req.buffer_id.to_string()));
             }
 
             // Update viewport state with clamped top_line (1-based).
             let vp = info.viewport_states.entry(req.buffer_id.clone()).or_insert(crate::ports::ViewportState { top_line: 1, window_height: 10, center_cursor: false });
             let current = vp.top_line as isize;
             let mut new_top = current.saturating_add(req.delta_lines);
             if new_top < 1 {
                 new_top = 1;
             }
             vp.top_line = new_top as usize;
 
             Ok(ScrollViewportResponse { ok: true, new_viewport: vp.clone() })
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
             let content = store.get_text(&active).unwrap_or_else(|| "".to_string());
             if content.trim().is_empty() {
                 // record failure
                 let cmd = CommandRecord::new_failure(
                     CommandKind::ExplainActiveBuffer,
                     Some(req.session_id.0),
                     Some(workspace_id),
                     Some(active.clone()),
                     Some("missing buffer content for explain".to_string()),
                 );
                 let _ = history.record_command(cmd).await;
                 return Err(UseCaseError::AiFailure("missing buffer content for explain".to_string()));
             }

             let ai_req = ai_ports::AiRequest {
                 session_id: req.session_id.0,
                 workspace_id,
                 buffer_id: active.clone(),
                 content_snapshot: content.clone(),
             };

             // Perform AI request and record failures/successes explicitly (avoid awaiting while holding locks)
             let res = match ai.request(ai_req).await {
                 Ok(r) => r,
                 Err(_e) => {
                     let cmd = CommandRecord::new_failure(
                         CommandKind::ExplainActiveBuffer,
                         Some(req.session_id.0),
                         Some(workspace_id),
                         Some(active.clone()),
                         Some("ai request failed".to_string()),
                     );
                     let _ = history.record_command(cmd).await;
                     return Err(UseCaseError::AiFailure("ai request failed".to_string()));
                 }
             };

             // record success and event
             let cmd = CommandRecord::new_success(
                 CommandKind::ExplainActiveBuffer,
                 Some(req.session_id.0),
                 Some(workspace_id),
                 Some(active.clone()),
                 Some(res.text.clone()),
             );
             let _ = history.record_command(cmd).await;

             let ev = WorkspaceEvent {
                 id: Uuid::new_v4(),
                 timestamp: Utc::now(),
                 session_id: req.session_id.clone(),
                 workspace_id,
                 kind: WorkspaceEventKind::ExplainExecuted { buffer_id: active.clone(), result: res.text.clone() },
             };
             let _ = history.record_event(ev).await;

             Ok(DispatchCommandResponse { result: CommandResult { message: res.text } })
         })
     }

     fn dispatch_command(&self, req: DispatchCommandRequest) -> BoxFuture<'static, Result<DispatchCommandResponse, UseCaseError>> {
         let ai = self.ai_client.clone();
         let store = self.buffer_store.clone();
         let sessions = self.sessions.clone();
         let history = self.history.clone();

         // Clone the command for history recording (we'll move req into the async closure)
         let cmd_for_history = req.command.clone();

         Box::pin(async move {
             // Resolve workspace id for session; avoid holding lock across await.
             let workspace_opt = {
                 let s = sessions.lock().unwrap();
                 s.get(&req.session_id.0).map(|w| w.workspace_id)
             };
             let workspace_id = match workspace_opt {
                 Some(w) => w,
                 None => {
                     // record failed dispatch
                     let cmd = CommandRecord::new_failure(
                         CommandKind::DispatchAppCommand { command: cmd_for_history.clone() },
                         Some(req.session_id.0),
                         None,
                         None,
                         Some("unknown session".to_string()),
                     );
                     let _ = history.record_command(cmd).await;
                     return Err(UseCaseError::UnknownSession)
                 }
             };

             match req.command {
                 AppCommand::AiExplain { buffer_id } => {
                     // (unchanged) Snapshot content for the AI request.
                     let content = store.get_text(&buffer_id).unwrap_or_else(|| "".to_string());
                     if content.trim().is_empty() {
                         // record failed dispatch
                         let cmd = CommandRecord::new_failure(
                             CommandKind::DispatchAppCommand { command: AppCommand::AiExplain { buffer_id: buffer_id.clone() } },
                             Some(req.session_id.0),
                             Some(workspace_id),
                             Some(buffer_id.clone()),
                             Some("missing buffer content for explain".to_string()),
                         );
                         let _ = history.record_command(cmd).await;
                         return Err(UseCaseError::AiFailure("missing buffer content for explain".to_string()));
                     }
                     let ai_req = ai_ports::AiRequest {
                         session_id: req.session_id.0,
                         workspace_id,
                         buffer_id: buffer_id.clone(),
                         content_snapshot: content,
                     };

                     let res = match ai.request(ai_req).await {
                         Ok(r) => r,
                         Err(_e) => {
                             let cmd = CommandRecord::new_failure(
                                 CommandKind::DispatchAppCommand { command: AppCommand::AiExplain { buffer_id: buffer_id.clone() } },
                                 Some(req.session_id.0),
                                 Some(workspace_id),
                                 Some(buffer_id.clone()),
                                 Some("ai request failed".to_string()),
                             );
                             let _ = history.record_command(cmd).await;
                             return Err(UseCaseError::AiFailure("ai request failed".to_string()));
                         }
                     };

                     // record success
                     let cmd = CommandRecord::new_success(
                         CommandKind::DispatchAppCommand { command: AppCommand::AiExplain { buffer_id: buffer_id.clone() } },
                         Some(req.session_id.0),
                         Some(workspace_id),
                         Some(buffer_id.clone()),
                         Some(res.text.clone()),
                     );
                     let _ = history.record_command(cmd).await;

                     Ok(DispatchCommandResponse { result: CommandResult { message: res.text } })
                 }

                 // InsertText: resolve cursor -> char index and apply a single Insert transaction.
                 AppCommand::InsertText { buffer_id, text } => {
                     // Membership check
                     let is_open = {
                         let s = sessions.lock().unwrap();
                         s.get(&req.session_id.0).map(|info| info.open_buffers.iter().any(|b| b == &buffer_id)).unwrap_or(false)
                     };
                     if !is_open {
                         let cmd = CommandRecord::new_failure(
                             CommandKind::DispatchAppCommand { command: AppCommand::InsertText { buffer_id: buffer_id.clone(), text: text.clone() } },
                             Some(req.session_id.0),
                             Some(workspace_id),
                             Some(buffer_id.clone()),
                             Some("invalid active buffer".to_string()),
                         );
                         let _ = history.record_command(cmd).await;
                         return Err(UseCaseError::InvalidActiveBuffer(buffer_id.to_string()));
                     }

                     // Determine insertion index from editor cursor (flatten to char index).
                     let content = store.get_text(&buffer_id).unwrap_or_else(|| "".to_string());
                     let cursor = {
                         let s = sessions.lock().unwrap();
                         s.get(&req.session_id.0).and_then(|info| info.editor_states.get(&buffer_id).cloned())
                     }.unwrap_or(EditorState { cursor: EditorCursor::zero(), selection: None }).cursor;

                     // Helper to convert (line,column) to char index; if line out of range clamp.
                     let pos_to_index = |content: &str, pos: EditorCursor| -> usize {
                         if pos.line == 0 {
                             return pos.column as usize;
                         }
                         // find start of the requested line
                         let mut line = 0usize;
                         let mut idx = 0usize;
                         for ch in content.chars() {
                             if line == pos.line as usize {
                                 break;
                             }
                             idx += ch.len_utf8();
                             if ch == '\n' {
                                 line += 1;
                             }
                         }
                         // now idx is byte index of line start; convert to char offset within content up to that byte index + column
                         let prefix = &content[..idx];
                         let mut chars = prefix.chars().count();
                         chars += pos.column as usize;
                         chars
                     };

                     let index = pos_to_index(&content, cursor);

                     let txn_req = ApplyTextTransactionRequest {
                         session_id: req.session_id.clone(),
                         buffer_id: buffer_id.clone(),
                         transaction: TextEdit::Insert { index, text: text.clone() },
                     };

                     // Delegate to the shared helper implementation to apply transaction and update state.
                     match apply_text_transaction_impl(store.clone(), sessions.clone(), history.clone(), txn_req).await {
                         Ok(_resp) => {
                             let cmd = CommandRecord::new_success(
                                 CommandKind::DispatchAppCommand { command: AppCommand::InsertText { buffer_id: buffer_id.clone(), text: text.clone() } },
                                 Some(req.session_id.0),
                                 Some(workspace_id),
                                 Some(buffer_id.clone()),
                                 Some("insert applied".to_string()),
                             );
                             let _ = history.record_command(cmd).await;
                             Ok(DispatchCommandResponse { result: CommandResult { message: format!("inserted") } })
                         }
                         Err(e) => {
                             let cmd = CommandRecord::new_failure(
                                 CommandKind::DispatchAppCommand { command: AppCommand::InsertText { buffer_id: buffer_id.clone(), text: text.clone() } },
                                 Some(req.session_id.0),
                                 Some(workspace_id),
                                 Some(buffer_id.clone()),
                                 Some(e.to_string()),
                             );
                             let _ = history.record_command(cmd).await;
                             Err(e)
                         }
                     }
                 }

                 // DeleteSelection: requires a selection present on the buffer inside session.
                 AppCommand::DeleteSelection { buffer_id } => {
                     // Ensure session has this buffer open
                     let is_open = {
                         let s = sessions.lock().unwrap();
                         s.get(&req.session_id.0).map(|info| info.open_buffers.iter().any(|b| b == &buffer_id)).unwrap_or(false)
                     };
                     if !is_open {
                         let cmd = CommandRecord::new_failure(
                             CommandKind::DispatchAppCommand { command: AppCommand::DeleteSelection { buffer_id: buffer_id.clone() } },
                             Some(req.session_id.0),
                             Some(workspace_id),
                             Some(buffer_id.clone()),
                             Some("invalid active buffer".to_string()),
                         );
                         let _ = history.record_command(cmd).await;
                         return Err(UseCaseError::InvalidActiveBuffer(buffer_id.to_string()));
                     }

                     // Read editor-state for selection
                     let selection_opt = {
                         let s = sessions.lock().unwrap();
                         s.get(&req.session_id.0).and_then(|info| info.editor_states.get(&buffer_id).and_then(|st| st.selection.clone()))
                     };

                     let selection = match selection_opt {
                         Some(s) => s,
                         None => {
                             let cmd = CommandRecord::new_failure(
                                 CommandKind::DispatchAppCommand { command: AppCommand::DeleteSelection { buffer_id: buffer_id.clone() } },
                                 Some(req.session_id.0),
                                 Some(workspace_id),
                                 Some(buffer_id.clone()),
                                 Some("no selection".to_string()),
                             );
                             let _ = history.record_command(cmd).await;
                             return Err(UseCaseError::InvalidMutation("no selection".to_string()));
                         }
                     };

                     // Convert selection anchor/active to char indices using buffer content.
                     let content = store.get_text(&buffer_id).unwrap_or_else(|| "".to_string());

                     let pos_to_index = |content: &str, pos: EditorCursor| -> usize {
                         if pos.line == 0 {
                             return pos.column as usize;
                         }
                         let mut line = 0usize;
                         let mut idx = 0usize;
                         for ch in content.chars() {
                             if line == pos.line as usize {
                                 break;
                             }
                             idx += ch.len_utf8();
                             if ch == '\n' {
                                 line += 1;
                             }
                         }
                         let prefix = &content[..idx];
                         let mut chars = prefix.chars().count();
                         chars += pos.column as usize;
                         chars
                     };

                     let a = pos_to_index(&content, selection.anchor);
                     let b = pos_to_index(&content, selection.active);
                     let (start, end) = if a <= b { (a, b) } else { (b, a) };

                     let txn_req = ApplyTextTransactionRequest {
                         session_id: req.session_id.clone(),
                         buffer_id: buffer_id.clone(),
                         transaction: TextEdit::Delete { start, end },
                     };

                     match apply_text_transaction_impl(store.clone(), sessions.clone(), history.clone(), txn_req).await {
                         Ok(_) => {
                             let cmd = CommandRecord::new_success(
                                 CommandKind::DispatchAppCommand { command: AppCommand::DeleteSelection { buffer_id: buffer_id.clone() } },
                                 Some(req.session_id.0),
                                 Some(workspace_id),
                                 Some(buffer_id.clone()),
                                 Some("deleted selection".to_string()),
                             );
                             let _ = history.record_command(cmd).await;
                             Ok(DispatchCommandResponse { result: CommandResult { message: "deleted".to_string() } })
                         }
                         Err(e) => {
                             let cmd = CommandRecord::new_failure(
                                 CommandKind::DispatchAppCommand { command: AppCommand::DeleteSelection { buffer_id: buffer_id.clone() } },
                                 Some(req.session_id.0),
                                 Some(workspace_id),
                                 Some(buffer_id.clone()),
                                 Some(e.to_string()),
                             );
                             let _ = history.record_command(cmd).await;
                             Err(e)
                         }
                     }
                 }

                 // ReplaceSelection: if selection present -> Replace, else Insert at cursor.
                 AppCommand::ReplaceSelection { buffer_id, text } => {
                     let is_open = {
                         let s = sessions.lock().unwrap();
                         s.get(&req.session_id.0).map(|info| info.open_buffers.iter().any(|b| b == &buffer_id)).unwrap_or(false)
                     };
                     if !is_open {
                         let cmd = CommandRecord::new_failure(
                             CommandKind::DispatchAppCommand { command: AppCommand::ReplaceSelection { buffer_id: buffer_id.clone(), text: text.clone() } },
                             Some(req.session_id.0),
                             Some(workspace_id),
                             Some(buffer_id.clone()),
                             Some("invalid active buffer".to_string()),
                         );
                         let _ = history.record_command(cmd).await;
                         return Err(UseCaseError::InvalidActiveBuffer(buffer_id.to_string()));
                     }

                     let content = store.get_text(&buffer_id).unwrap_or_else(|| "".to_string());

                     let selection_opt = {
                         let s = sessions.lock().unwrap();
                         s.get(&req.session_id.0).and_then(|info| info.editor_states.get(&buffer_id).and_then(|st| st.selection.clone()))
                     };

                     if let Some(selection) = selection_opt {
                         let pos_to_index = |content: &str, pos: EditorCursor| -> usize {
                             if pos.line == 0 {
                                 return pos.column as usize;
                             }
                             let mut line = 0usize;
                             let mut idx = 0usize;
                             for ch in content.chars() {
                                 if line == pos.line as usize {
                                     break;
                                 }
                                 idx += ch.len_utf8();
                                 if ch == '\n' {
                                     line += 1;
                                 }
                             }
                             let prefix = &content[..idx];
                             let mut chars = prefix.chars().count();
                             chars += pos.column as usize;
                             chars
                         };

                         let a = pos_to_index(&content, selection.anchor);
                         let b = pos_to_index(&content, selection.active);
                         let (start, end) = if a <= b { (a, b) } else { (b, a) };

                         let txn_req = ApplyTextTransactionRequest {
                             session_id: req.session_id.clone(),
                             buffer_id: buffer_id.clone(),
                             transaction: TextEdit::Replace { start, end, text: text.clone() },
                         };

                         match apply_text_transaction_impl(store.clone(), sessions.clone(), history.clone(), txn_req).await {
                             Ok(_) => {
                                 let cmd = CommandRecord::new_success(
                                     CommandKind::DispatchAppCommand { command: AppCommand::ReplaceSelection { buffer_id: buffer_id.clone(), text: text.clone() } },
                                     Some(req.session_id.0),
                                     Some(workspace_id),
                                     Some(buffer_id.clone()),
                                     Some("replaced selection".to_string()),
                                 );
                                 let _ = history.record_command(cmd).await;
                                 Ok(DispatchCommandResponse { result: CommandResult { message: "replaced".to_string() } })
                             }
                             Err(e) => {
                                 let cmd = CommandRecord::new_failure(
                                     CommandKind::DispatchAppCommand { command: AppCommand::ReplaceSelection { buffer_id: buffer_id.clone(), text: text.clone() } },
                                     Some(req.session_id.0),
                                     Some(workspace_id),
                                     Some(buffer_id.clone()),
                                     Some(e.to_string()),
                                 );
                                 let _ = history.record_command(cmd).await;
                                 Err(e)
                             }
                         }
                     } else {
                         // No selection -> insert at cursor
                         let cursor = {
                             let s = sessions.lock().unwrap();
                             s.get(&req.session_id.0).and_then(|info| info.editor_states.get(&buffer_id).cloned())
                         }.unwrap_or(EditorState { cursor: EditorCursor::zero(), selection: None }).cursor;

                         let pos_to_index = |content: &str, pos: EditorCursor| -> usize {
                             if pos.line == 0 {
                                 return pos.column as usize;
                             }
                             let mut line = 0usize;
                             let mut idx = 0usize;
                             for ch in content.chars() {
                                 if line == pos.line as usize {
                                     break;
                                 }
                                 idx += ch.len_utf8();
                                 if ch == '\n' {
                                     line += 1;
                                 }
                             }
                             let prefix = &content[..idx];
                             let mut chars = prefix.chars().count();
                             chars += pos.column as usize;
                             chars
                         };

                         let index = pos_to_index(&content, cursor);

                         let txn_req = ApplyTextTransactionRequest {
                             session_id: req.session_id.clone(),
                             buffer_id: buffer_id.clone(),
                             transaction: TextEdit::Insert { index, text: text.clone() },
                         };

                         match apply_text_transaction_impl(store.clone(), sessions.clone(), history.clone(), txn_req).await {
                             Ok(_) => {
                                 let cmd = CommandRecord::new_success(
                                     CommandKind::DispatchAppCommand { command: AppCommand::ReplaceSelection { buffer_id: buffer_id.clone(), text: text.clone() } },
                                     Some(req.session_id.0),
                                     Some(workspace_id),
                                     Some(buffer_id.clone()),
                                     Some("inserted (replace-no-selection)".to_string()),
                                 );
                                 let _ = history.record_command(cmd).await;
                                 Ok(DispatchCommandResponse { result: CommandResult { message: "inserted".to_string() } })
                             }
                             Err(e) => {
                                 let cmd = CommandRecord::new_failure(
                                     CommandKind::DispatchAppCommand { command: AppCommand::ReplaceSelection { buffer_id: buffer_id.clone(), text: text.clone() } },
                                     Some(req.session_id.0),
                                     Some(workspace_id),
                                     Some(buffer_id.clone()),
                                     Some(e.to_string()),
                                 );
                                 let _ = history.record_command(cmd).await;
                                 Err(e)
                             }
                         }
                     }
                 }

                 // IndentLine: insert a small indent at the beginning of the current line.
                 AppCommand::IndentLine { buffer_id } => {
                     let is_open = {
                         let s = sessions.lock().unwrap();
                         s.get(&req.session_id.0).map(|info| info.open_buffers.iter().any(|b| b == &buffer_id)).unwrap_or(false)
                     };
                     if !is_open {
                         let cmd = CommandRecord::new_failure(
                             CommandKind::DispatchAppCommand { command: AppCommand::IndentLine { buffer_id: buffer_id.clone() } },
                             Some(req.session_id.0),
                             Some(workspace_id),
                             Some(buffer_id.clone()),
                             Some("invalid active buffer".to_string()),
                         );
                         let _ = history.record_command(cmd).await;
                         return Err(UseCaseError::InvalidActiveBuffer(buffer_id.to_string()));
                     }

                     let content = store.get_text(&buffer_id).unwrap_or_else(|| "".to_string());
                     let cursor = {
                         let s = sessions.lock().unwrap();
                         s.get(&req.session_id.0).and_then(|info| info.editor_states.get(&buffer_id).cloned())
                     }.unwrap_or(EditorState { cursor: EditorCursor::zero(), selection: None }).cursor;

                     // Compute line start index
                     let mut line = 0usize;
                     let mut byte_idx = 0usize;
                     for ch in content.chars() {
                         if line == cursor.line as usize {
                             break;
                         }
                         byte_idx += ch.len_utf8();
                         if ch == '\n' {
                             line += 1;
                         }
                     }
                     // Convert byte_idx to char index
                     let line_start_chars = content[..byte_idx].chars().count();

                     let txn_req = ApplyTextTransactionRequest {
                         session_id: req.session_id.clone(),
                         buffer_id: buffer_id.clone(),
                         transaction: TextEdit::Insert { index: line_start_chars, text: "    ".to_string() },
                     };

                     match apply_text_transaction_impl(store.clone(), sessions.clone(), history.clone(), txn_req).await {
                         Ok(_) => {
                             let cmd = CommandRecord::new_success(
                                 CommandKind::DispatchAppCommand { command: AppCommand::IndentLine { buffer_id: buffer_id.clone() } },
                                 Some(req.session_id.0),
                                 Some(workspace_id),
                                 Some(buffer_id.clone()),
                                 Some("indented".to_string()),
                             );
                             let _ = history.record_command(cmd).await;
                             Ok(DispatchCommandResponse { result: CommandResult { message: "indented".to_string() } })
                         }
                         Err(e) => {
                             let cmd = CommandRecord::new_failure(
                                 CommandKind::DispatchAppCommand { command: AppCommand::IndentLine { buffer_id: buffer_id.clone() } },
                                 Some(req.session_id.0),
                                 Some(workspace_id),
                                 Some(buffer_id.clone()),
                                 Some(e.to_string()),
                             );
                             let _ = history.record_command(cmd).await;
                             Err(e)
                         }
                     }
                 }
             }
         })
     }

     fn update_buffer(&self, req: UpdateBufferRequest) -> BoxFuture<'static, Result<UpdateBufferResponse, UseCaseError>> {
         let store = self.buffer_store.clone();
         let sessions = self.sessions.clone();
         let history = self.history.clone();
         Box::pin(async move {
             // Validate session is known (release lock before awaiting)
             let session_known = {
                 let s = sessions.lock().unwrap();
                 s.contains_key(&req.session_id.0)
             };
             if !session_known {
                 // record failed update
                 let cmd = CommandRecord::new_failure(
                     CommandKind::UpdateBuffer { buffer_id: req.buffer_id.clone() },
                     Some(req.session_id.0),
                     None,
                     Some(req.buffer_id.clone()),
                     Some("unknown session".to_string()),
                 );
                 let _ = history.record_command(cmd).await;
                 return Err(UseCaseError::UnknownSession);
             }

             if let Err(m) = buffer_rules::validate_content(&req.new_content) {
                 let cmd = CommandRecord::new_failure(
                     CommandKind::UpdateBuffer { buffer_id: req.buffer_id.clone() },
                     Some(req.session_id.0),
                     None,
                     Some(req.buffer_id.clone()),
                     Some(m.clone()),
                 );
                 let _ = history.record_command(cmd).await;
                 return Err(UseCaseError::InvalidMutation(m));
             }

             // Perform the mutation via BufferStore (infra).
             if let Err(_e) = store.set_text(&req.buffer_id, req.new_content.clone()).await {
                 // record failed mutation
                 let cmd = CommandRecord::new_failure(
                     CommandKind::UpdateBuffer { buffer_id: req.buffer_id.clone() },
                     Some(req.session_id.0),
                     None,
                     Some(req.buffer_id.clone()),
                     Some("unknown buffer".to_string()),
                 );
                 let _ = history.record_command(cmd).await;
                 return Err(UseCaseError::UnknownBuffer);
             }

             // record success and event
             let workspace_id_opt = {
                 let s = sessions.lock().unwrap();
                 s.get(&req.session_id.0).map(|i| i.workspace_id)
             };

             let cmd = CommandRecord::new_success(
                 CommandKind::UpdateBuffer { buffer_id: req.buffer_id.clone() },
                 Some(req.session_id.0),
                 workspace_id_opt,
                 Some(req.buffer_id.clone()),
                 Some("updated".to_string()),
             );
             let _ = history.record_command(cmd).await;

             if let Some(ws) = workspace_id_opt {
                 let ev = WorkspaceEvent {
                     id: Uuid::new_v4(),
                     timestamp: Utc::now(),
                     session_id: req.session_id.clone(),
                     workspace_id: ws,
                     kind: WorkspaceEventKind::BufferUpdated { buffer_id: req.buffer_id.clone() },
                 };
                 let _ = history.record_event(ev).await;
             }

             Ok(UpdateBufferResponse { ok: true })
         })
     }
 
     fn apply_text_transaction(&self, req: ApplyTextTransactionRequest) -> BoxFuture<'static, Result<ApplyTextTransactionResponse, UseCaseError>> {
         let store = self.buffer_store.clone();
         let sessions = self.sessions.clone();
         let history = self.history.clone();
         Box::pin(async move {
             // Validate session exists
             let session_known = { let s = sessions.lock().unwrap(); s.contains_key(&req.session_id.0) };
             if !session_known {
                 let cmd = CommandRecord::new_failure(
                     CommandKind::UpdateBuffer { buffer_id: req.buffer_id.clone() },
                     Some(req.session_id.0),
                     None,
                     Some(req.buffer_id.clone()),
                     Some("unknown session".to_string()),
                 );
                 let _ = history.record_command(cmd).await;
                 return Err(UseCaseError::UnknownSession);
             }
 
             // Ensure buffer is opened in session and prepare editor-state mutation
             {
                 // Short-lived membership check: acquire lock only to test membership and immediately drop it.
                 let is_open = {
                     let s = sessions.lock().unwrap();
                     s.get(&req.session_id.0).map(|info| info.open_buffers.iter().any(|b| b == &req.buffer_id)).unwrap_or(false)
                 };
 
                 if !is_open {
                     let cmd = CommandRecord::new_failure(
                         CommandKind::UpdateBuffer { buffer_id: req.buffer_id.clone() },
                         Some(req.session_id.0),
                         None,
                         Some(req.buffer_id.clone()),
                         Some("invalid active buffer".to_string()),
                     );
                     // history.record_command may await; do this while NOT holding the sessions lock.
                     let _ = history.record_command(cmd).await;
                     return Err(UseCaseError::InvalidActiveBuffer(req.buffer_id.to_string()));
                 }
 
                 // Now acquire the lock again briefly to mutate editor-state.
                 let mut s = sessions.lock().unwrap();
                 let info = s.get_mut(&req.session_id.0).ok_or(UseCaseError::UnknownSession)?;
 
                 // Simple cursor/selection update policy (Phase 4 minimal):
                 // We treat EditorCursor.column as a flat character index while line==0.
                 // - Insert at the cursor index advances the cursor by inserted chars.
                 // - Delete shifts cursor left if it was after the deleted range.
                 // - Replace adjusts cursor depending on whether it was inside replaced region.
                 let entry = info.editor_states.entry(req.buffer_id.clone()).or_insert(EditorState { cursor: EditorCursor::zero(), selection: None });
                 let mut new_cursor = entry.cursor.clone();
                 match &req.transaction {
                     TextEdit::Insert { index, text } => {
                         if entry.cursor.line == 0 && entry.cursor.column as usize == *index {
                             new_cursor.column = new_cursor.column.saturating_add(text.chars().count() as u32);
                         }
                         entry.selection = None;
                         entry.cursor = new_cursor.clone();
                     }
                     TextEdit::Delete { start, end } => {
                         let del_len = if *end > *start { *end - *start } else { 0 };
                         if entry.cursor.line == 0 {
                             let cidx = entry.cursor.column as usize;
                             if cidx > *start {
                                 let sub = std::cmp::min(cidx - *start, del_len);
                                 new_cursor.column = new_cursor.column.saturating_sub(sub as u32);
                             }
                         }
                         entry.selection = None;
                         entry.cursor = new_cursor.clone();
                     }
                     TextEdit::Replace { start, end, text } => {
                         let old_len = if *end > *start { *end - *start } else { 0 };
                         let new_len = text.chars().count();
                         if entry.cursor.line == 0 {
                             let cidx = entry.cursor.column as usize;
                             if cidx >= *start && cidx <= *end {
                                 // cursor inside replaced region -> move to end of replacement
                                 new_cursor.column = (*start + new_len) as u32;
                             } else if cidx > *end {
                                 if new_len >= old_len {
                                     new_cursor.column = new_cursor.column.saturating_add((new_len - old_len) as u32);
                                 } else {
                                     new_cursor.column = new_cursor.column.saturating_sub((old_len - new_len) as u32);
                                 }
                             }
                         }
                         entry.selection = None;
                         entry.cursor = new_cursor.clone();
                     }
                 }
             }
 
             // Apply the transaction to the buffer via the BufferStore
             if let Err(_e) = store.apply_transaction(&req.buffer_id, req.transaction.clone()).await {
                 let cmd = CommandRecord::new_failure(
                     CommandKind::UpdateBuffer { buffer_id: req.buffer_id.clone() },
                     Some(req.session_id.0),
                     None,
                     Some(req.buffer_id.clone()),
                     Some("unknown buffer".to_string()),
                 );
                 let _ = history.record_command(cmd).await;
                 return Err(UseCaseError::UnknownBuffer);
             }
 
             // record success and event
             let workspace_id_opt = {
                 let s = sessions.lock().unwrap();
                 s.get(&req.session_id.0).map(|i| i.workspace_id)
             };
 
             let cmd = CommandRecord::new_success(
                 CommandKind::UpdateBuffer { buffer_id: req.buffer_id.clone() },
                 Some(req.session_id.0),
                 workspace_id_opt,
                 Some(req.buffer_id.clone()),
                 Some("transaction applied".to_string()),
             );
             let _ = history.record_command(cmd).await;
 
             if let Some(ws) = workspace_id_opt {
                 let ev = WorkspaceEvent {
                     id: Uuid::new_v4(),
                     timestamp: Utc::now(),
                     session_id: req.session_id.clone(),
                     workspace_id: ws,
                     kind: WorkspaceEventKind::BufferUpdated { buffer_id: req.buffer_id.clone() },
                 };
                 let _ = history.record_event(ev).await;
             }
 
             // Read updated content and editor-state to return
             let content = store.get_text(&req.buffer_id);
             let state = {
                 let s = sessions.lock().unwrap();
                 s.get(&req.session_id.0).and_then(|info| info.editor_states.get(&req.buffer_id).cloned())
             };
 
             Ok(ApplyTextTransactionResponse { ok: true, state: state.unwrap_or(EditorState { cursor: EditorCursor::zero(), selection: None }), content })
         })
     }
 
     fn get_recent_commands(&self, req: GetRecentCommandsRequest) -> BoxFuture<'static, Result<GetRecentCommandsResponse, UseCaseError>> {
         let history = self.history.clone();
         Box::pin(async move {
             let recs: Vec<CommandRecord> = history.get_recent_commands(req.session_id.clone(), req.limit).await.map_err(|_e| UseCaseError::AiFailure("history query failed".to_string()))?;
             Ok(GetRecentCommandsResponse { commands: recs })
         })
     }

     fn get_recent_events(&self, req: GetRecentEventsRequest) -> BoxFuture<'static, Result<GetRecentEventsResponse, UseCaseError>> {
         let history = self.history.clone();
         Box::pin(async move {
             let evs: Vec<WorkspaceEvent> = history.get_recent_events(req.session_id.clone(), req.limit).await.map_err(|_e| UseCaseError::AiFailure("history query failed".to_string()))?;
             Ok(GetRecentEventsResponse { events: evs })
         })
     }
 
     fn get_session_snapshot(&self, req: GetSessionSnapshotRequest) -> BoxFuture<'static, Result<GetSessionSnapshotResponse, UseCaseError>> {
         let sessions = self.sessions.clone();
         let store = self.buffer_store.clone();
         let history = self.history.clone();
         Box::pin(async move {
             // Resolve session info (synchronous lookup).
             let info = {
                 let s = sessions.lock().unwrap();
                 s.get(&req.session_id.0).cloned().ok_or(UseCaseError::UnknownSession)?
             };
 
             let opened = info.open_buffers.clone();
             let active = info.active_buffer.clone();
             let workspace_id = info.workspace_id;
 
             // Snapshot buffer contents for opened buffers (sync read path from BufferStore).
             let mut buffers: Vec<BufferSnapshot> = Vec::new();
             for b in opened.iter() {
                 let content = store.get_text(&b.clone());
                 buffers.push(BufferSnapshot { buffer_id: b.clone(), content });
             }
 
             // Recent commands and events (read from history port).
             let commands = history.get_recent_commands(req.session_id.clone(), req.recent_limit).await.map_err(|_e| UseCaseError::AiFailure("history query failed".to_string()))?;
             let events = history.get_recent_events(req.session_id.clone(), req.recent_limit).await.map_err(|_e| UseCaseError::AiFailure("history query failed".to_string()))?;
 
             let snapshot = SessionSnapshot {
                 session_id: req.session_id.clone(),
                 workspace_id,
                 opened_buffers: opened,
                 active_buffer: active,
                 buffers,
                 recent_commands: commands,
                 recent_events: events,
             };
 
             Ok(GetSessionSnapshotResponse { snapshot })
         })
     }
 
     fn create_checkpoint(&self, req: CreateCheckpointRequest) -> BoxFuture<'static, Result<CreateCheckpointResponse, UseCaseError>> {
         let sessions = self.sessions.clone();
         let store = self.buffer_store.clone();
         let history = self.history.clone();
         Box::pin(async move {
             // Resolve session info (synchronous lookup).
             let info = {
                 let s = sessions.lock().unwrap();
                 s.get(&req.session_id.0).cloned().ok_or(UseCaseError::UnknownSession)?
             };
 
             let opened = info.open_buffers.clone();
             let active = info.active_buffer.clone();
             let workspace_id = info.workspace_id;
 
             // Snapshot buffer contents for opened buffers (sync read path from BufferStore).
             let mut buffers: Vec<BufferSnapshot> = Vec::new();
             for b in opened.iter() {
                 let content = store.get_text(&b.clone());
                 buffers.push(BufferSnapshot { buffer_id: b.clone(), content });
             }
 
             // Recent commands and events (read from history port).
             let commands = history.get_recent_commands(req.session_id.clone(), 50).await.map_err(|_e| UseCaseError::AiFailure("history query failed".to_string()))?;
             let events = history.get_recent_events(req.session_id.clone(), 50).await.map_err(|_e| UseCaseError::AiFailure("history query failed".to_string()))?;
 
             let checkpoint = Checkpoint {
                 version: 1,
                 session_id: req.session_id.clone(),
                 workspace_id,
                 opened_buffers: opened,
                 active_buffer: active,
                 buffers,
                 recent_commands: commands,
                 recent_events: events,
                 created_at: Utc::now(),
             };
 
             Ok(CreateCheckpointResponse { checkpoint })
         })
     }
 
     fn save_checkpoint(&self, req: crate::ports::SaveCheckpointRequest) -> BoxFuture<'static, Result<crate::ports::SaveCheckpointResponse, UseCaseError>> {
         let store = self.buffer_store.clone();
         let sessions = self.sessions.clone();
         let history = self.history.clone();
         let durability = self.durability.clone();
         Box::pin(async move {
             // Resolve session info (synchronous lookup).
             let info = {
                 let s = sessions.lock().unwrap();
                 s.get(&req.session_id.0).cloned().ok_or(UseCaseError::UnknownSession)?
             };
 
             let opened = info.open_buffers.clone();
             let active = info.active_buffer.clone();
             let workspace_id = info.workspace_id;
 
             // Snapshot buffer contents for opened buffers (sync read path from BufferStore).
             let mut buffers: Vec<crate::ports::BufferSnapshot> = Vec::new();
             for b in opened.iter() {
                 let content = store.get_text(&b.clone());
                 buffers.push(crate::ports::BufferSnapshot { buffer_id: b.clone(), content });
             }
 
             // Recent commands and events (read from history port).
             let commands = history.get_recent_commands(req.session_id.clone(), 50).await.map_err(|_e| UseCaseError::AiFailure("history query failed".to_string()))?;
             let events = history.get_recent_events(req.session_id.clone(), 50).await.map_err(|_e| UseCaseError::AiFailure("history query failed".to_string()))?;
 
             let checkpoint = crate::ports::Checkpoint {
                 version: 1,
                 session_id: req.session_id.clone(),
                 workspace_id,
                 opened_buffers: opened,
                 active_buffer: active,
                 buffers,
                 recent_commands: commands,
                 recent_events: events,
                 created_at: Utc::now(),
             };
 
             // Persist via durability adapter (serialize is responsibility of adapter or can be performed by adapter).
             let loc = match durability.save_checkpoint(checkpoint).await {
                 Ok(l) => l,
                 Err(e) => return Err(UseCaseError::DurabilityFailure(e.to_string())),
             };
 
             Ok(crate::ports::SaveCheckpointResponse { location: loc })
         })
     }
 
     fn load_checkpoint(&self, req: crate::ports::LoadCheckpointRequest) -> BoxFuture<'static, Result<crate::ports::LoadCheckpointResponse, UseCaseError>> {
         let store = self.buffer_store.clone();
         let sessions = self.sessions.clone();
         let history = self.history.clone();
         let durability = self.durability.clone();
         Box::pin(async move {
             // Load checkpoint bytes/record from durability adapter.
             let ck = match durability.load_checkpoint(req.location.clone()).await {
                 Ok(c) => c,
                 Err(e) => {
                     // Map durability errors to explicit use-case errors.
                     match e {
                         crate::ports::DurabilityError::Malformed(s) => return Err(UseCaseError::InvalidCheckpoint(s)),
                         crate::ports::DurabilityError::UnknownVersion(v) => return Err(UseCaseError::InvalidCheckpoint(format!("unknown version {}", v))),
                         _ => return Err(UseCaseError::DurabilityFailure(e.to_string())),
                     }
                 }
             };
 
             // Validate target session id is not already in use.
             {
                 let s = sessions.lock().unwrap();
                 if s.contains_key(&ck.session_id.0) {
                     return Err(UseCaseError::SessionAlreadyExists(ck.session_id.clone()));
                 }
             }
 
             // Ensure buffers exist in the store and apply contents if provided.
             for b in ck.opened_buffers.iter() {
                 // If buffer missing, attempt to open by deriving path from id "buf:<path>"
                 if store.get_text(&b.clone()).is_none() {
                     if let Some(path) = b.path() {
                         match store.open_buffer(path).await {
                             Ok(_id) => {}
                             Err(_) => return Err(UseCaseError::InvalidCheckpoint(format!("cannot open buffer {}", b))),
                         }
                     } else {
                         return Err(UseCaseError::InvalidCheckpoint(format!("invalid buffer id {}", b)));
                     }
                 }
 
                 // Apply content snapshot when present.
                 if let Some(bs) = ck.buffers.iter().find(|bs| bs.buffer_id == *b) {
                     if let Some(content) = &bs.content {
                         if let Err(_) = store.set_text(&b.clone(), content.clone()).await {
                             return Err(UseCaseError::InvalidCheckpoint(format!("failed to set buffer {}", b)));
                         }
                     }
                 }
             }
 
             // Insert session info
             {
                 let mut s = sessions.lock().unwrap();
                 s.insert(ck.session_id.0, SessionInfo { workspace_id: ck.workspace_id, open_buffers: ck.opened_buffers.clone(), active_buffer: ck.active_buffer.clone(), editor_states: std::collections::HashMap::new(), viewport_states: std::collections::HashMap::new() });
             }
 
             // Record provided recent commands and events into history (best-effort).
             for c in ck.recent_commands.iter() {
                 let _ = history.record_command(c.clone()).await;
             }
             for e in ck.recent_events.iter() {
                 let _ = history.record_event(e.clone()).await;
             }
 
             let session = WorkspaceSessionDTO { session_id: ck.session_id.clone(), workspace_id: ck.workspace_id };
 
             Ok(RestoreCheckpointResponse { session, replaced_session_id: None })
         })
     }
 
     fn restore_checkpoint(&self, req: RestoreCheckpointRequest) -> BoxFuture<'static, Result<RestoreCheckpointResponse, UseCaseError>> {
         let store = self.buffer_store.clone();
         let sessions = self.sessions.clone();
         let history = self.history.clone();
         Box::pin(async move {
             let ck = req.checkpoint;
 
             // Validate target session id is not already in use.
             {
                 let s = sessions.lock().unwrap();
                 if s.contains_key(&ck.session_id.0) {
                     return Err(UseCaseError::SessionAlreadyExists(ck.session_id.clone()));
                 }
             }
 
             // Ensure buffers exist in the store and apply contents if provided.
             for b in ck.opened_buffers.iter() {
                 // If buffer missing, attempt to open by deriving path from id "buf:<path>"
                 if store.get_text(&b.clone()).is_none() {
                     if let Some(path) = b.path() {
                         match store.open_buffer(path).await {
                             Ok(_id) => {}
                             Err(_) => return Err(UseCaseError::InvalidCheckpoint(format!("cannot open buffer {}", b))),
                         }
                     } else {
                         return Err(UseCaseError::InvalidCheckpoint(format!("invalid buffer id {}", b)));
                     }
                 }
 
                 // Apply content snapshot when present.
                 if let Some(bs) = ck.buffers.iter().find(|bs| bs.buffer_id == *b) {
                     if let Some(content) = &bs.content {
                         if let Err(_) = store.set_text(&b.clone(), content.clone()).await {
                             return Err(UseCaseError::InvalidCheckpoint(format!("failed to set buffer {}", b)));
                         }
                     }
                 }
             }
 
             // Insert session info
             {
                 let mut s = sessions.lock().unwrap();
                 s.insert(ck.session_id.0, SessionInfo { workspace_id: ck.workspace_id, open_buffers: ck.opened_buffers.clone(), active_buffer: ck.active_buffer.clone(), editor_states: std::collections::HashMap::new(), viewport_states: std::collections::HashMap::new() });
             }
 
             // Record provided recent commands and events into history (best-effort).
             for c in ck.recent_commands.iter() {
                 let _ = history.record_command(c.clone()).await;
             }
             for e in ck.recent_events.iter() {
                 let _ = history.record_event(e.clone()).await;
             }
 
             let session = WorkspaceSessionDTO { session_id: ck.session_id.clone(), workspace_id: ck.workspace_id };
 
             Ok(RestoreCheckpointResponse { session, replaced_session_id: None })
         })
     }
 }

 impl crate::ports::WorkspaceView for WorkspaceOrchestrator {
     fn get_buffer_content(&self, buffer_id: crate::ports::BufferId) -> BoxFuture<'static, Result<Option<String>, crate::ports::UseCaseError>> {
         let store = self.buffer_store.clone();
         // synchronous read path from BufferStore
         Box::pin(async move {
             Ok(store.get_text(&buffer_id))
         })
     }

     fn get_active_buffer_content(&self, session_id: crate::ports::SessionId) -> BoxFuture<'static, Result<Option<String>, crate::ports::UseCaseError>> {
         let sessions = self.sessions.clone();
         let store = self.buffer_store.clone();
         Box::pin(async move {
             // Resolve session synchronously
             let info_opt = {
                 let s = sessions.lock().unwrap();
                 s.get(&session_id.0).cloned()
             };
             let info = info_opt.ok_or(crate::ports::UseCaseError::UnknownSession)?;
             if let Some(active) = info.active_buffer {
                 Ok(store.get_text(&active))
             } else {
                 Err(crate::ports::UseCaseError::NoActiveBuffer)
             }
         })
     }

     fn get_active_editor_document(&self, req: crate::ports::GetActiveEditorDocumentRequest) -> BoxFuture<'static, Result<crate::ports::GetActiveEditorDocumentResponse, crate::ports::UseCaseError>> {
         let sessions = self.sessions.clone();
         let store = self.buffer_store.clone();
         Box::pin(async move {
             // Resolve session synchronously
             let info_opt = {
                 let s = sessions.lock().unwrap();
                 s.get(&req.session_id.0).cloned()
             };
             let info = info_opt.ok_or(crate::ports::UseCaseError::UnknownSession)?;
             let active = info.active_buffer.ok_or(crate::ports::UseCaseError::NoActiveBuffer)?;
 
             // Snapshot content for the active buffer (sync read path).
             let content = store.get_text(&active);
 
             // Editor transient state for the buffer (if present)
             let state = info.editor_states.get(&active).cloned().unwrap_or(crate::ports::EditorState { cursor: crate::ports::EditorCursor::zero(), selection: None });
 
             let line_count = content.as_ref().map(|c| c.lines().count()).unwrap_or(0);
             let current_line = content.as_ref().and_then(|c| c.lines().nth(state.cursor.line as usize).map(|s| s.to_string()));
 
             let doc = crate::ports::EditorDocument {
                 buffer_id: active.clone(),
                 content,
                 cursor: state.cursor,
                 selection: state.selection,
                 line_count,
                 current_line,
             };
 
             Ok(crate::ports::GetActiveEditorDocumentResponse { document: doc })
         })
     }
 
     fn get_visible_lines(&self, req: crate::ports::GetVisibleLinesRequest) -> BoxFuture<'static, Result<crate::ports::GetVisibleLinesResponse, crate::ports::UseCaseError>> {
         let sessions = self.sessions.clone();
         let store = self.buffer_store.clone();
         Box::pin(async move {
             // Resolve session info
             let info = {
                 let s = sessions.lock().unwrap();
                 s.get(&req.session_id.0).cloned().ok_or(crate::ports::UseCaseError::UnknownSession)?
             };
 
             // Ensure buffer is opened in session
             if !info.open_buffers.iter().any(|b| b == &req.buffer_id) {
                 return Err(crate::ports::UseCaseError::InvalidActiveBuffer(req.buffer_id.to_string()));
             }
 
             // Snapshot content
             let content = store.get_text(&req.buffer_id);
             let state = info.editor_states.get(&req.buffer_id).cloned().unwrap_or(crate::ports::EditorState { cursor: crate::ports::EditorCursor::zero(), selection: None });
             let viewport = info.viewport_states.get(&req.buffer_id).cloned().unwrap_or(crate::ports::ViewportState { top_line: 1, window_height: 10, center_cursor: false });
 
             let line_count = content.as_ref().map(|c| c.lines().count()).unwrap_or(0);
             let current_line = content.as_ref().and_then(|c| c.lines().nth(state.cursor.line as usize).map(|s| s.to_string()));
 
             let doc = crate::ports::EditorDocument {
                 buffer_id: req.buffer_id.clone(),
                 content,
                 cursor: state.cursor,
                 selection: state.selection,
                 line_count,
                 current_line,
             };
 
             // Delegate to view seam for projection using viewport state.
             let window = crate::view::project_visible_lines_for_viewport(&doc, &viewport);
 
             Ok(crate::ports::GetVisibleLinesResponse { window })
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
             // Use core helper to construct BufferId from the path; avoids stringly creation here.
             let id = buffer_ports::BufferId::from(path);
             Box::pin(async move {
                 Ok(id)
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

         fn apply_transaction(&self, id: &buffer_ports::BufferId, _txn: buffer_ports::TextEdit) -> crate::ports::BoxFuture<'static, Result<(), buffer_ports::BufferError>> {
             let key = id.0.clone();
             Box::pin(async move {
                 // Minimal fake behavior: accept transactions for buffer ids that look valid.
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
         // Prefer typed assertion: the BufferId is expected to map to a filesystem path.
         assert!(open_res.buffer_id.path().is_some());
 
         // Dispatch AI explain
         let cmd = DispatchCommandRequest { session_id: boot_res.session.session_id.clone(), command: AppCommand::AiExplain { buffer_id: open_res.buffer_id.clone() } };
         let cmd_res = orch.dispatch_command(cmd).await.expect("dispatch ok");
         assert!(cmd_res.result.message.contains("fake-explain"));
     }
 }
