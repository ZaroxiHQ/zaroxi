 use std::sync::{Arc, Mutex};
 use std::collections::HashMap;

 use crate::ports::{
     WorkspaceBootRequest, WorkspaceBootResponse, OpenBufferRequest, OpenBufferResponse,
     UpdateBufferRequest, UpdateBufferResponse,
     DispatchCommandRequest, DispatchCommandResponse, AppCommand, CommandResult, WorkspaceSessionDTO,
     ListBuffersRequest, ListBuffersResponse, SetActiveBufferRequest, SetActiveBufferResponse,
     GetActiveBufferRequest, GetActiveBufferResponse,
 };
 
 use zaroxi_domain_workspace::ports as domain_ports;
 use zaroxi_core_editor_buffer::ports as buffer_ports;
 use zaroxi_application_ai::ports as ai_ports;
 use zaroxi_kernel_types::Id;
 
 /// Concrete orchestrator implementing application use-cases.
 ///
 /// This struct belongs to the application layer. It composes domain and core ports,
 /// delegating side-effects to adapters provided by the composition root.
 pub struct WorkspaceOrchestrator {
     repo: Arc<dyn domain_ports::WorkspaceRepository>,
     buffer_store: Arc<dyn buffer_ports::BufferStore>,
     ai_client: Arc<dyn ai_ports::AiClient>,
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

 impl WorkspaceOrchestrator {
     /// Create a new orchestrator with concrete port implementations (adapters).
     pub fn new(
         repo: Arc<dyn domain_ports::WorkspaceRepository>,
         buffer_store: Arc<dyn buffer_ports::BufferStore>,
         ai_client: Arc<dyn ai_ports::AiClient>,
     ) -> Self {
         Self { repo, buffer_store, ai_client, sessions: Arc::new(Mutex::new(HashMap::new())) }
     }
 }

 impl crate::ports::WorkspaceService for WorkspaceOrchestrator {
     fn boot_workspace(&self, req: WorkspaceBootRequest) -> BoxFuture<'static, Result<WorkspaceBootResponse, UseCaseError>> {
         let repo = self.repo.clone();
         let sessions = self.sessions.clone();
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
             Ok(WorkspaceBootResponse { session })
         })
     }

     fn open_buffer(&self, req: OpenBufferRequest) -> BoxFuture<'static, Result<OpenBufferResponse, UseCaseError>> {
         let store = self.buffer_store.clone();
         let sessions = self.sessions.clone();
         Box::pin(async move {
             // Validate session exists
             {
                 let s = sessions.lock().unwrap();
                 if !s.contains_key(&req.session_id.0) {
                     return Err(UseCaseError::UnknownSession);
                 }
             }

             // Ask underlying store to open buffer
             let id = store.open_buffer(req.path.clone()).await.map_err(|_e| UseCaseError::UnknownBuffer)?;
             let buffer_id = id.0.clone();

             // Register buffer in session and set active if first
             {
                 let mut s = sessions.lock().unwrap();
                 if let Some(info) = s.get_mut(&req.session_id.0) {
                     info.open_buffers.push(buffer_id.clone());
                     if info.active_buffer.is_none() {
                         info.active_buffer = Some(buffer_id.clone());
                     }
                 }
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
         Box::pin(async move {
             let mut s = sessions.lock().unwrap();
             let info = s.get_mut(&req.session_id.0).ok_or(UseCaseError::UnknownSession)?;
             // Ensure requested buffer was opened in this session
             if !info.open_buffers.iter().any(|b| b == &req.buffer_id) {
                 return Err(UseCaseError::InvalidActiveBuffer(req.buffer_id));
             }
             info.active_buffer = Some(req.buffer_id.clone());
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
         Box::pin(async move {
             // Resolve active buffer id
             let active = {
                 let s = sessions.lock().unwrap();
                 let info = s.get(&req.session_id.0).ok_or(UseCaseError::UnknownSession)?;
                 info.active_buffer.clone().ok_or(UseCaseError::NoActiveBuffer)?
             };

             // Snapshot content for the AI request.
             let buf_id = buffer_ports::BufferId(active.clone());
             let content = store.get_text(&buf_id).unwrap_or_else(|| "".to_string());
             if content.trim().is_empty() {
                 return Err(UseCaseError::AiFailure("missing buffer content for explain".to_string()));
             }

             // Retrieve workspace id for ai request
             let workspace_id = {
                 let s = sessions.lock().unwrap();
                 s.get(&req.session_id.0).map(|i| i.workspace_id).ok_or(UseCaseError::UnknownSession)?
             };

             let ai_req = ai_ports::AiRequest {
                 session_id: req.session_id.0,
                 workspace_id,
                 buffer_id: active.clone(),
                 content_snapshot: content,
             };
             let res = ai.request(ai_req).await.map_err(|_e| UseCaseError::AiFailure("ai request failed".to_string()))?;
             Ok(DispatchCommandResponse { result: CommandResult { message: res.text } })
         })
     }

     fn dispatch_command(&self, req: DispatchCommandRequest) -> BoxFuture<'static, Result<DispatchCommandResponse, UseCaseError>> {
         let ai = self.ai_client.clone();
         let store = self.buffer_store.clone();
         let sessions = self.sessions.clone();
         Box::pin(async move {
             // Validate session exists
             let workspace_id = {
                 let s = sessions.lock().unwrap();
                 match s.get(&req.session_id.0) {
                     Some(w) => w.workspace_id,
                     None => return Err(UseCaseError::UnknownSession),
                 }
             };

             match req.command {
                 AppCommand::AiExplain { buffer_id } => {
                     // Snapshot content for the AI request.
                     let buf_id = buffer_ports::BufferId(buffer_id.clone());
                     let content = store.get_text(&buf_id).unwrap_or_else(|| "".to_string());
                     if content.trim().is_empty() {
                         return Err(UseCaseError::AiFailure("missing buffer content for explain".to_string()));
                     }
                     let ai_req = ai_ports::AiRequest {
                         session_id: req.session_id.0,
                         workspace_id,
                         buffer_id: buffer_id.clone(),
                         content_snapshot: content,
                     };
                     let res = ai.request(ai_req).await.map_err(|_e| UseCaseError::AiFailure("ai request failed".to_string()))?;
                     Ok(DispatchCommandResponse { result: CommandResult { message: res.text } })
                 }
                 AppCommand::InsertText { .. } => {
                     // Not implemented in Phase 5; return a successful no-op.
                     Ok(DispatchCommandResponse { result: CommandResult { message: "inserted (noop)".to_string() } })
                 }
             }
         })
     }

     fn update_buffer(&self, req: UpdateBufferRequest) -> BoxFuture<'static, Result<UpdateBufferResponse, UseCaseError>> {
         let store = self.buffer_store.clone();
         let sessions = self.sessions.clone();
         Box::pin(async move {
             // Validate session is known.
             {
                 let s = sessions.lock().unwrap();
                 if !s.contains_key(&req.session_id.0) {
                     return Err(UseCaseError::UnknownSession);
                 }
             }

             // Validate buffer id and content via domain rules.
             buffer_rules::validate_buffer_id(&req.buffer_id).map_err(|m| UseCaseError::InvalidMutation(m))?;
             buffer_rules::validate_content(&req.new_content).map_err(|m| UseCaseError::InvalidMutation(m))?;

             // Perform the mutation via BufferStore (infra).
             store.set_text(&buffer_ports::BufferId(req.buffer_id.clone()), req.new_content.clone())
                 .await
                 .map_err(|_e| UseCaseError::UnknownBuffer)?;

             Ok(UpdateBufferResponse { ok: true })
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
