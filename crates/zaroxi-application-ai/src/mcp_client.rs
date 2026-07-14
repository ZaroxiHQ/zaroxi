//! MCP client — manages the connection lifecycle, initialization handshake,
//! tool listing, and tool invocation for a single MCP server.
//!
//! Uses the domain-layer transport trait and stdio transport implementation.

use serde_json::{Value, json};

use zaroxi_domain_ai::mcp::{McpConnectionState, McpTool};
use zaroxi_domain_ai::mcp_jsonrpc::{
    JsonRpcId, JsonRpcMessage, JsonRpcNotification, JsonRpcRequest, JsonRpcResponse,
};
use zaroxi_domain_ai::mcp_transport::McpTransportConnection;

pub struct McpToolResult {
    pub content: Vec<McpContentBlock>,
    pub is_error: bool,
}

pub struct McpContentBlock {
    pub kind: String,
    pub text: String,
}

pub struct McpClient {
    transport: Box<dyn McpTransportConnection>,
    state: McpConnectionState,
    capabilities: Option<Value>,
    server_name: Option<String>,
    server_version: Option<String>,
    tools: Vec<McpTool>,
    next_id: i64,
}

impl McpClient {
    pub fn new(transport: Box<dyn McpTransportConnection>) -> Self {
        Self {
            transport,
            state: McpConnectionState::Disconnected,
            capabilities: None,
            server_name: None,
            server_version: None,
            tools: Vec::new(),
            next_id: 1,
        }
    }

    pub fn state(&self) -> McpConnectionState {
        self.state
    }

    pub fn tools(&self) -> &[McpTool] {
        &self.tools
    }

    pub fn server_name(&self) -> Option<&str> {
        self.server_name.as_deref()
    }

    fn send_request(
        &mut self,
        method: &str,
        params: Option<Value>,
    ) -> Result<JsonRpcResponse, String> {
        let id: JsonRpcId = self.next_id.into();
        self.next_id += 1;
        let req = JsonRpcRequest::new(id.clone(), method, params);
        let json = serde_json::to_string(&req).map_err(|e| format!("serialize: {e}"))?;
        self.transport.send(&json)?;

        let timeout_ms = 30_000u128;
        let start = std::time::Instant::now();
        while start.elapsed().as_millis() < timeout_ms {
            match self.transport.receive() {
                Ok(msg) => match msg {
                    JsonRpcMessage::Response(resp) if resp.id == id => return Ok(resp),
                    JsonRpcMessage::Error(err) if err.id.as_ref() == Some(&id) => {
                        return Err(format!(
                            "MCP error [{}]: {}",
                            err.error.code, err.error.message
                        ));
                    }
                    _ => {}
                },
                Err(e) => {
                    if e.contains("timed out") {
                        continue;
                    }
                    return Err(e);
                }
            }
        }
        Err(format!("request {method} timed out"))
    }

    fn send_notification(&mut self, method: &str, params: Option<Value>) -> Result<(), String> {
        let notif = JsonRpcNotification::new(method, params);
        let json = serde_json::to_string(&notif).map_err(|e| format!("serialize: {e}"))?;
        self.transport.send(&json)
    }

    pub fn initialize(&mut self) -> Result<(), String> {
        if !self.transport.is_alive() {
            self.state = McpConnectionState::Error;
            return Err("transport not alive".into());
        }

        self.state = McpConnectionState::Connecting;

        let init_params = json!({
            "protocolVersion": "2024-11-05",
            "capabilities": {
                "roots": { "listChanged": false },
                "sampling": {}
            },
            "clientInfo": { "name": "zaroxi-studio", "version": "0.1.0" }
        });

        let resp = self.send_request("initialize", Some(init_params))?;

        let result = resp.result.ok_or("initialize returned no result")?;
        self.capabilities = result.get("capabilities").cloned();
        if let Some(info) = result.get("serverInfo") {
            self.server_name = info.get("name").and_then(|v| v.as_str()).map(String::from);
            self.server_version = info.get("version").and_then(|v| v.as_str()).map(String::from);
        }

        self.send_notification("notifications/initialized", None)?;

        let tools_resp = self.send_request("tools/list", None)?;
        if let Some(tools_json) = tools_resp.result.and_then(|r| r.get("tools").cloned()) {
            Self::parse_tools(&mut self.tools, &tools_json);
        }

        self.state = McpConnectionState::Connected;
        tracing::info!(
            "MCP client connected: {:?} v{:?}, {} tools",
            self.server_name,
            self.server_version,
            self.tools.len()
        );
        Ok(())
    }

    pub fn call_tool(
        &mut self,
        tool_name: &str,
        arguments: Option<Value>,
    ) -> Result<McpToolResult, String> {
        if self.state != McpConnectionState::Connected {
            return Err("not connected".into());
        }

        let params = json!({
            "name": tool_name,
            "arguments": arguments.unwrap_or(Value::Object(serde_json::Map::new()))
        });
        let resp = self.send_request("tools/call", Some(params))?;
        let result = resp.result.ok_or("tools/call returned no result")?;
        let is_error = result.get("isError").and_then(|v| v.as_bool()).unwrap_or(false);
        let content = result
            .get("content")
            .and_then(|c| c.as_array())
            .map(|arr| {
                arr.iter()
                    .map(|block| McpContentBlock {
                        kind: block.get("type").and_then(|v| v.as_str()).unwrap_or("text").into(),
                        text: block.get("text").and_then(|v| v.as_str()).unwrap_or("").into(),
                    })
                    .collect()
            })
            .unwrap_or_default();

        Ok(McpToolResult { content, is_error })
    }

    pub fn disconnect(&mut self) {
        let _ = self.transport.close();
        self.state = McpConnectionState::Disconnected;
    }

    fn parse_tools(tools: &mut Vec<McpTool>, json: &Value) {
        if let Some(arr) = json.as_array() {
            for entry in arr {
                let name = entry.get("name").and_then(|n| n.as_str()).unwrap_or("").to_string();
                let description =
                    entry.get("description").and_then(|d| d.as_str()).unwrap_or("").to_string();
                let input_schema =
                    entry.get("inputSchema").map(|s| serde_json::to_string(s).unwrap_or_default());
                if !name.is_empty() {
                    tools.push(McpTool { name, description, input_schema });
                }
            }
        }
    }
}

impl Drop for McpClient {
    fn drop(&mut self) {
        self.disconnect();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_client_starts_disconnected() {
        let fake: Box<dyn McpTransportConnection> = Box::new(FakeTransport::new());
        let client = McpClient::new(fake);
        assert_eq!(client.state(), McpConnectionState::Disconnected);
    }

    #[test]
    fn parse_tools_from_json() {
        let json: Value = serde_json::from_str(
            r#"[
                {"name": "read_file", "description": "Read a file", "inputSchema": {"type":"object"}},
                {"name": "list_dir", "description": "List directory"}
            ]"#,
        )
        .unwrap();
        let mut tools = Vec::new();
        McpClient::parse_tools(&mut tools, &json);
        assert_eq!(tools.len(), 2);
        assert_eq!(tools[0].name, "read_file");
    }

    struct FakeTransport {
        alive: bool,
    }

    impl FakeTransport {
        fn new() -> Self {
            Self { alive: true }
        }
    }

    impl McpTransportConnection for FakeTransport {
        fn send(&mut self, _json: &str) -> Result<(), String> {
            Ok(())
        }
        fn receive(&mut self) -> Result<JsonRpcMessage, String> {
            Ok(JsonRpcMessage::Response(JsonRpcResponse {
                jsonrpc: "2.0".into(),
                id: 1.into(),
                result: Some(json!({"ok": true})),
            }))
        }
        fn is_alive(&self) -> bool {
            self.alive
        }
        fn close(&mut self) -> Result<(), String> {
            self.alive = false;
            Ok(())
        }
    }
}
