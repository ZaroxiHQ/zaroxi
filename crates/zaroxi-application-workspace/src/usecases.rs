 use std::sync::Arc;

 use crate::ports::{
     WorkspaceBootRequest, WorkspaceBootResponse, OpenBufferRequest, OpenBufferResponse,
     DispatchCommandRequest, DispatchCommandResponse, AppCommand, CommandResult, WorkspaceSessionDTO,
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
     /// In-memory session -> workspace mapping for the simple slice.
     sessions: Arc<Mutex<HashMap<Id, Id>>>,
 }
 
 use crate::ports::BoxFuture;
 
     fn open_buffer(&self, req: OpenBufferRequest) -> BoxFuture<'static, Result<OpenBufferResponse, String>> {
         let store = self.buffer_store.clone();
         Box::pin(async move {
             let id = store.open_buffer(req.path.clone()).await.map_err(|e| e.0)?;
             Ok(OpenBufferResponse { buffer_id: id.0 })
         })
     }
 
     fn dispatch_command(&self, req: DispatchCommandRequest) -> BoxFuture<'static, Result<DispatchCommandResponse, String>> {
         let ai = self.ai_client.clone();
         let store = self.buffer_store.clone();
         Box::pin(async move {
             match req.command {
                 AppCommand::AiExplain { buffer_id } => {
                     // Snapshot content for the AI request.
                     let buf_id = buffer_ports::BufferId(buffer_id.clone());
                     let content = store.get_text(&buf_id).unwrap_or_else(|| "".to_string());
                     let ai_req = ai_ports::AiRequest {
                         session_id: (req.session_id.0),
                         workspace_id: (Id::new()), // we don't persist workspace mapping here in the simple orchestrator; in Phase 3 this will be stored
                         buffer_id: buffer_id.clone(),
                         content_snapshot: content,
                     };
                     let res = ai.request(ai_req).await.map_err(|e| e.0)?;
                     Ok(DispatchCommandResponse { result: CommandResult { message: res.text } })
                 }
                 AppCommand::InsertText { .. } => {
                     // Not implemented in Phase 2; return a successful no-op.
                     Ok(DispatchCommandResponse { result: CommandResult { message: "inserted (noop)".to_string() } })
                 }
             }
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
