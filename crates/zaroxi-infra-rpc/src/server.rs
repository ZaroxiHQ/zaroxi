#![doc = r#"
RPC transport adapter (implementation plan + skeleton)

This file contains the transport-layer skeleton for the RPC server and a
comprehensive implementation plan so the team can iteratively implement
the concrete transport (e.g. JSON-RPC over TCP, WebSocket or gRPC).

High-level architecture (aligned with workspace crates):

- zaroxi-protocol
  - defines stable request/response DTOs, method names and transport-safe
    error DTOs. No business logic here.

- zaroxi-domain-*
  - pure domain models and logic (e.g., zaroxi-domain-workspace,
    zaroxi-domain-editor, zaroxi-domain-ai-context).

- zaroxi-service-*
  - application services that orchestrate domain logic and external
    concerns (e.g., zaroxi-service-workspace, zaroxi-service-ai).

- zaroxi-infra-rpc (this crate)
  - transport bootstrap, handler registration, mapping between protocol
    DTOs and service calls, and error mapping to transport-friendly errors.

Implementation goals for this crate:
- Keep handlers thin: translate protocol types -> call service trait -> map response.
- Do not contain business rules.
- Provide an injectable AppContext with trait-based service references so the
  infra crate does not need to depend on concrete service crates during unit tests.
- Provide consistent error mapping (internal -> transport).

FILE RESPONSIBILITIES (suggested layout inside this crate)
- lib.rs: re-export server, registry, error, context, handlers modules.
- server.rs: bootstrap server, runtime lifecycle, connection accept loop.
- registry.rs: method registration and lookup (method name -> handler).
- context.rs: AppContext definition and DI helpers (traits for services).
- error.rs: RpcError and mapping helpers.
- handlers/mod.rs: small handler modules (workspace.rs, editor.rs, ai.rs),
  each exposing a function that returns a boxed Handler.

HANDLER PATTERN (one per RPC method or small cohesive group)
1. Signature: async fn handle(ctx: &AppContext, req: ProtocolRequest) -> Result<ProtocolResponse, DomainError>
2. Validate transport-level preconditions (auth token present, schema).
3. Map to service-layer types and call service trait.
4. Map service result to protocol response.
5. Let server.rs map DomainError -> RpcError -> transport encoding.

ERROR MAPPING
- Domain/Service errors: implement std::error::Error in the service crates.
- Define an RpcError in this crate (code, message, optional detail).
- Provide From conversions: From<DomainError> for RpcError (controlled mapping).
- Expose RpcError -> protocol::error::RpcErrorDto conversion.

EXAMPLE METHOD FLOW (workspace.open)
1. protocol defines WorkspaceOpenRequest/WorkspaceOpenResponse in zaroxi-protocol.
2. frontend sends request (method = "workspace.open", params = {...}).
3. infra-rpc:
   - receives raw payload, deserializes into protocol::rpc::Request<WorkspaceOpenRequest>.
   - looks up handler by method name -> handlers::workspace::open.
   - handler receives AppContext with a WorkspaceService trait object.
   - handler calls workspace_service.open_workspace(path).await
   - on success, handler returns WorkspaceOpenResponse.
   - server serializes successful response and writes back.
   - on error, server maps DomainError -> RpcError -> transport error response.

EXAMPLE CODE: lightweight skeleton types and helper functions follow.
This file intentionally keeps dependencies minimal and exposes trait shapes
so concrete service crates can implement them and be injected at runtime.
"#]

use std::collections::HashMap;
use std::sync::Arc;
use std::pin::Pin;
use std::future::Future;

#[allow(unused_imports)]
use serde::{Deserialize, Serialize};

/// Application context passed to handlers. This is intentionally trait-based
/// so tests can inject mocks and the infra crate does not force concrete
/// service implementations.
#[derive(Clone)]
pub struct AppContext {
    pub workspace_service: Arc<dyn WorkspaceService + Send + Sync>,
    pub ai_service: Arc<dyn AiService + Send + Sync>,
    // Add other shared dependencies (metrics, tracer, config) here.
}

/// Workspace service trait that the service crate must implement.
///
/// Note: This trait lives in infra-rpc as a minimal interface for handlers.
/// The concrete implementation should be provided by `zaroxi-service-workspace`.
#[allow(dead_code)]
pub trait WorkspaceService {
    fn open_workspace(&self, path: String) -> Pin<Box<dyn Future<Output = Result<WorkspaceOpenResult, WorkspaceError>> + Send>>;
    fn read_tree(&self, path: String) -> Pin<Box<dyn Future<Output = Result<WorkspaceTree, WorkspaceError>> + Send>>;
}

/// AI service trait shape for handlers to call into.
#[allow(dead_code)]
pub trait AiService {
    fn query_context(&self, prompt: String) -> Pin<Box<dyn Future<Output = Result<AiResult, AiError>> + Send>>;
}

/// Example domain/result structs used by the trait signatures.
/// In real code these should come from the service crates' public interfaces.
#[derive(Debug, Serialize, Deserialize)]
pub struct WorkspaceOpenResult {
    pub workspace_id: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct WorkspaceTree {
    pub root_path: String,
}

#[derive(Debug)]
pub struct WorkspaceError {
    pub message: String,
}

impl std::fmt::Display for WorkspaceError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "WorkspaceError: {}", self.message)
    }
}

impl std::error::Error for WorkspaceError {}

#[derive(Debug)]
pub struct AiResult {
    pub response: String,
}

#[derive(Debug)]
pub struct AiError {
    pub message: String,
}

impl std::fmt::Display for AiError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "AiError: {}", self.message)
    }
}

impl std::error::Error for AiError {}

/// A minimal RpcError type that will be mapped to a transport-safe error DTO.
#[derive(Debug, Serialize, Deserialize)]
pub struct RpcError {
    pub code: i32,
    pub message: String,
    /// Optional machine-readable details (kept small and stable).
    pub detail: Option<String>,
}

impl RpcError {
    pub fn internal(msg: impl Into<String>) -> Self {
        RpcError { code: -32603, message: msg.into(), detail: None }
    }

    pub fn invalid_params(msg: impl Into<String>) -> Self {
        RpcError { code: -32602, message: msg.into(), detail: None }
    }
}

/// Generic result type used by handlers.
pub type RpcResult<T> = Result<T, RpcError>;

/// Handler trait: every RPC method handler implements this.
pub trait Handler: Send + Sync + 'static {
    fn call(&self, ctx: AppContext, params: serde_json::Value) -> Pin<Box<dyn Future<Output = RpcResult<serde_json::Value>> + Send>>;
}

/// Registry maps method names to handlers.
///
/// A small registry keeps things explicit and easy to test. Handlers are
/// typically created in handlers/* and registered during server bootstrap.
pub struct Registry {
    handlers: HashMap<String, Box<dyn Handler>>,
}

impl Registry {
    pub fn new() -> Self {
        Self { handlers: HashMap::new() }
    }

    pub fn register(&mut self, method: impl Into<String>, handler: Box<dyn Handler>) {
        self.handlers.insert(method.into(), handler);
    }

    pub fn get(&self, method: &str) -> Option<&Box<dyn Handler>> {
        self.handlers.get(method)
    }
}

/// Lightweight server skeleton that wires AppContext and Registry together.
///
/// Concrete bootstrap should:
/// - construct concrete WorkspaceService and AiService implementations (from service crates),
/// - create AppContext with Arc::new(impl),
/// - build Registry by registering handlers (handlers::workspace::make_open_handler(ctx.workspace_service.clone()) ...),
/// - call RpcServer::serve(...) to start accepting transports.
///
/// This skeleton intentionally leaves transport (TCP/WebSocket/gRPC) to the
/// concrete bootstrap so tests can run handlers directly.
pub struct RpcServer {
    ctx: AppContext,
    registry: Registry,
}

impl RpcServer {
    pub fn new(ctx: AppContext, registry: Registry) -> Self {
        Self { ctx, registry }
    }

    /// Handle a single incoming request represented by a method name and
    /// raw JSON params. This is the thin handler dispatcher used by transports.
    pub async fn handle_request(&self, method: &str, params: serde_json::Value) -> serde_json::Value {
        match self.registry.get(method) {
            Some(handler) => {
                match handler.call(self.ctx.clone(), params).await {
                    Ok(result_json) => {
                        // Successful response envelope (transport to fill in specifics).
                        serde_json::json!({ "ok": true, "result": result_json })
                    }
                    Err(err) => {
                        serde_json::json!({ "ok": false, "error": { "code": err.code, "message": err.message, "detail": err.detail } })
                    }
                }
            }
            None => {
                let err = RpcError::invalid_params(format!("method not found: {}", method));
                serde_json::json!({ "ok": false, "error": { "code": err.code, "message": err.message } })
            }
        }
    }
}

/// Example handler implementation for workspace.open.
/// In practice this lives in handlers/workspace.rs and is registered during bootstrap.
pub struct WorkspaceOpenHandler {
    workspace_service: Arc<dyn WorkspaceService + Send + Sync>,
}

impl WorkspaceOpenHandler {
    pub fn new(workspace_service: Arc<dyn WorkspaceService + Send + Sync>) -> Self {
        Self { workspace_service }
    }
}

impl Handler for WorkspaceOpenHandler {
    fn call(&self, _ctx: AppContext, params: serde_json::Value) -> Pin<Box<dyn Future<Output = RpcResult<serde_json::Value>> + Send>> {
        let svc = self.workspace_service.clone();
        let params = params.clone();
        Box::pin(async move {
            // Transport-level validation: expect { "path": "..." }
            let path = params.get("path").and_then(|v| v.as_str()).ok_or_else(|| RpcError::invalid_params("missing 'path'"))?;
            // Call into service layer (thin adapter)
            match svc.open_workspace(path.to_string()).await {
                Ok(res) => {
                    // Map domain result to protocol response shape (here: simple JSON)
                    let resp = serde_json::json!({ "workspace_id": res.workspace_id });
                    Ok(resp)
                }
                Err(e) => {
                    // Map service/domain error to RpcError (stable codes/messages)
                    let rpc_err = RpcError::internal(format!("failed to open workspace: {}", e));
                    Err(rpc_err)
                }
            }
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use std::sync::Mutex;

    // A tiny in-test WorkspaceService mock.
    struct MockWorkspaceService {
        opened: Arc<Mutex<Vec<String>>>,
    }

    impl WorkspaceService for MockWorkspaceService {
        fn open_workspace(&self, path: String) -> Pin<Box<dyn Future<Output = Result<WorkspaceOpenResult, WorkspaceError>> + Send>> {
            let opened = self.opened.clone();
            let path_clone = path.clone();
            Box::pin(async move {
                opened.lock().unwrap().push(path_clone.clone());
                Ok(WorkspaceOpenResult { workspace_id: format!("id:{}", path_clone) })
            })
        }

        fn read_tree(&self, _path: String) -> Pin<Box<dyn Future<Output = Result<WorkspaceTree, WorkspaceError>> + Send>> {
            Box::pin(async move {
                Ok(WorkspaceTree { root_path: "/".to_string() })
            })
        }
    }

    #[tokio::test]
    async fn workspace_open_handler_success() {
        let svc: Arc<dyn WorkspaceService + Send + Sync> =
            Arc::new(MockWorkspaceService { opened: Arc::new(Mutex::new(vec![])) });
        let handler = WorkspaceOpenHandler::new(svc.clone());
        let ctx = AppContext {
            workspace_service: svc.clone(),
            ai_service: Arc::new(MockAiService {}),
        };
        let params = json!({ "path": "/home/project" });
        let result = handler.call(ctx, params).await;
        assert!(result.is_ok());
        let value = result.unwrap();
        assert_eq!(value.get("workspace_id").unwrap().as_str().unwrap(), "id:/home/project");
    }

    struct MockAiService {}

    impl AiService for MockAiService {
        fn query_context(&self, _prompt: String) -> Pin<Box<dyn Future<Output = Result<AiResult, AiError>> + Send>> {
            Box::pin(async move { Ok(AiResult { response: "ok".to_string() }) })
        }
    }
}
