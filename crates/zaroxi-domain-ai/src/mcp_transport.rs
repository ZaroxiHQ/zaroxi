//! MCP transport trait — abstract interface for MCP server communication.
//!
//! Application and infrastructure layers implement this trait to provide
//! actual stdout/stdin or SSE-based transports. The domain layer defines
//! the contract; infrastructure provides the concrete implementations.

use crate::mcp_jsonrpc::JsonRpcMessage;

/// An active MCP transport connection.
///
/// Implementations handle the underlying I/O (stdio pipes, SSE stream),
/// while callers send/receive JSON-RPC messages through this trait.
pub trait McpTransportConnection: Send {
    /// Send a JSON-RPC message to the server.
    fn send(&mut self, json: &str) -> Result<(), String>;

    /// Receive the next JSON-RPC message from the server (blocking).
    fn receive(&mut self) -> Result<JsonRpcMessage, String>;

    /// Check if the connection is still alive.
    fn is_alive(&self) -> bool;

    /// Close the connection and release resources.
    fn close(&mut self) -> Result<(), String>;
}

/// Factory trait for creating transport connections from configuration.
pub trait McpTransportFactory: Send + Sync {
    /// Open a connection to an MCP server based on its transport config.
    fn connect(
        &self,
        transport: &crate::mcp::McpTransport,
    ) -> Result<Box<dyn McpTransportConnection>, String>;
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mcp::McpTransport;

    /// A simple in-memory transport for unit testing.
    struct TestTransport {
        sent: Vec<String>,
        responses: std::collections::VecDeque<JsonRpcMessage>,
        alive: bool,
    }

    impl TestTransport {
        fn new() -> Self {
            Self { sent: Vec::new(), responses: std::collections::VecDeque::new(), alive: true }
        }

        fn queue_response(&mut self, msg: JsonRpcMessage) {
            self.responses.push_back(msg);
        }
    }

    impl McpTransportConnection for TestTransport {
        fn send(&mut self, json: &str) -> Result<(), String> {
            self.sent.push(json.to_string());
            Ok(())
        }

        fn receive(&mut self) -> Result<JsonRpcMessage, String> {
            self.responses.pop_front().ok_or_else(|| "no more messages".into())
        }

        fn is_alive(&self) -> bool {
            self.alive
        }

        fn close(&mut self) -> Result<(), String> {
            self.alive = false;
            Ok(())
        }
    }

    struct TestFactory {
        transport: std::sync::Mutex<Option<TestTransport>>,
    }

    impl TestFactory {
        fn new() -> Self {
            Self { transport: std::sync::Mutex::new(None) }
        }

        fn set_transport(&self, t: TestTransport) {
            *self.transport.lock().unwrap() = Some(t);
        }
    }

    impl McpTransportFactory for TestFactory {
        fn connect(
            &self,
            _transport: &McpTransport,
        ) -> Result<Box<dyn McpTransportConnection>, String> {
            let t = self.transport.lock().unwrap().take().ok_or("no test transport")?;
            Ok(Box::new(t))
        }
    }

    #[test]
    fn test_transport_send_receive() {
        use crate::mcp_jsonrpc::{JsonRpcRequest, JsonRpcResponse};
        let mut t = TestTransport::new();
        t.queue_response(JsonRpcMessage::Response(JsonRpcResponse {
            jsonrpc: "2.0".into(),
            id: JsonRpcRequest::new(1.into(), "test", None).id,
            result: Some(serde_json::Value::String("ok".into())),
        }));

        assert!(t.is_alive());
        t.send("test").unwrap();
        assert_eq!(t.sent.len(), 1);

        let msg = t.receive().unwrap();
        assert!(matches!(msg, JsonRpcMessage::Response(_)));

        t.close().unwrap();
        assert!(!t.is_alive());
    }
}
