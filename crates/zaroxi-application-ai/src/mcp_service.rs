//! MCP service — manages the lifecycle of multiple MCP servers.
//!
//! Handles connection, initialization, tool discovery, health monitoring,
//! and tool invocation for all registered MCP servers.
//!
//! Phase 3: full connection lifecycle, tool discovery, tool-call routing.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use zaroxi_domain_ai::mcp::{McpConnectionState, McpServerState, McpSettings, McpTool};
use zaroxi_domain_ai::mcp_stdio::StdioTransport;

use crate::mcp_client::McpClient;

/// State of all managed MCP servers.
#[derive(Default)]
struct McpServiceState {
    servers: HashMap<String, McpServerState>,
    clients: HashMap<String, McpClient>,
}

/// Manages MCP server connections, tool discovery, and tool calls.
pub struct McpService {
    state: Arc<Mutex<McpServiceState>>,
}

impl Default for McpService {
    fn default() -> Self {
        Self::new()
    }
}

impl McpService {
    pub fn new() -> Self {
        Self { state: Arc::new(Mutex::new(McpServiceState::default())) }
    }

    pub fn register_servers(&self, settings: &McpSettings) {
        let mut state = self.state.lock().unwrap();
        for config in &settings.servers {
            let srv = McpServerState::new(config.clone());
            state.servers.insert(config.id.clone(), srv);
        }
    }

    /// Connect to a server by id, performing the full MCP handshake.
    pub fn connect(&self, server_id: &str) -> Result<(), String> {
        let mut state = self.state.lock().unwrap();
        let srv = state.servers.get_mut(server_id).ok_or("server not found")?;

        if !srv.config.enabled {
            return Err("server is disabled".into());
        }

        srv.connection_state = McpConnectionState::Connecting;
        let config = srv.config.clone();
        drop(state);

        let transport = StdioTransport::spawn(&config.transport).map_err(|e| {
            let mut s = self.state.lock().unwrap();
            if let Some(srv) = s.servers.get_mut(server_id) {
                srv.connection_state = McpConnectionState::Error;
                srv.error_message = Some(e.clone());
            }
            format!("connection failed: {e}")
        })?;

        let mut client = McpClient::new(Box::new(transport));
        client.initialize().map_err(|e| {
            let mut s = self.state.lock().unwrap();
            if let Some(srv) = s.servers.get_mut(server_id) {
                srv.connection_state = McpConnectionState::Error;
                srv.error_message = Some(e.clone());
            }
            client.disconnect();
            format!("handshake failed: {e}")
        })?;

        let mut state = self.state.lock().unwrap();
        if let Some(srv) = state.servers.get_mut(server_id) {
            srv.connection_state = McpConnectionState::Connected;
            srv.tools = client.tools().to_vec();
            srv.error_message = None;
        }
        state.clients.insert(server_id.to_string(), client);

        Ok(())
    }

    /// Disconnect from a server.
    pub fn disconnect(&self, server_id: &str) {
        let mut state = self.state.lock().unwrap();
        if let Some(mut client) = state.clients.remove(server_id) {
            client.disconnect();
        }
        if let Some(srv) = state.servers.get_mut(server_id) {
            srv.connection_state = McpConnectionState::Disconnected;
        }
    }

    /// Get the current state of a server.
    pub fn server_state(&self, server_id: &str) -> Option<McpServerState> {
        self.state.lock().unwrap().servers.get(server_id).cloned()
    }

    /// Get all server states.
    pub fn all_server_states(&self) -> Vec<McpServerState> {
        self.state.lock().unwrap().servers.values().cloned().collect()
    }

    /// Get all tools from all connected servers.
    pub fn all_tools(&self) -> Vec<McpTool> {
        self.state
            .lock()
            .unwrap()
            .servers
            .values()
            .filter(|s| s.is_ready())
            .flat_map(|s| s.tools.clone())
            .collect()
    }

    /// Call a tool on a connected server.
    pub fn call_tool(
        &self,
        server_id: &str,
        tool_name: &str,
        arguments: Option<serde_json::Value>,
    ) -> Result<String, String> {
        let mut state = self.state.lock().unwrap();
        let client = state.clients.get_mut(server_id).ok_or("not connected")?;

        let result = client.call_tool(tool_name, arguments).map_err(|e| e.to_string())?;

        let output = result.content.iter().map(|b| b.text.clone()).collect::<Vec<_>>().join("\n");
        Ok(output)
    }

    /// Toggle a server on or off.
    pub fn set_enabled(&self, server_id: &str, enabled: bool) -> Result<(), String> {
        let mut state = self.state.lock().unwrap();
        let srv = state.servers.get_mut(server_id).ok_or("server not found")?;
        srv.config.enabled = enabled;
        if !enabled {
            srv.connection_state = McpConnectionState::Disabled;
        }
        if !enabled && let Some(mut client) = state.clients.remove(server_id) {
            client.disconnect();
        }
        Ok(())
    }

    /// Summary status for the AI panel.
    pub fn status_summary(&self) -> McpStatusSummary {
        let state = self.state.lock().unwrap();
        let connected = state.servers.values().filter(|s| s.is_ready()).count();
        let total = state.servers.len();
        let tool_count: usize =
            state.servers.values().filter(|s| s.is_ready()).map(|s| s.tools.len()).sum();
        McpStatusSummary { connected, total, tool_count }
    }
}

/// A compact status summary for UI display.
#[derive(Debug, Clone)]
pub struct McpStatusSummary {
    pub connected: usize,
    pub total: usize,
    pub tool_count: usize,
}

impl McpStatusSummary {
    pub fn is_any_connected(&self) -> bool {
        self.connected > 0
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use zaroxi_domain_ai::mcp::McpServerConfig;
    use zaroxi_domain_ai::mcp::McpTransport;

    #[test]
    fn service_starts_with_no_servers() {
        let svc = McpService::new();
        assert_eq!(svc.all_server_states().len(), 0);
        assert_eq!(svc.all_tools().len(), 0);
    }

    #[test]
    fn register_and_query_servers() {
        let svc = McpService::new();
        let mut settings = McpSettings::default();
        settings.add_server(McpServerConfig {
            id: "test1".into(),
            display_name: "Test".into(),
            description: "A test".into(),
            enabled: true,
            auto_start: false,
            transport: McpTransport::Stdio { command: "echo".into(), args: vec![], env: vec![] },
        });
        svc.register_servers(&settings);
        assert_eq!(svc.all_server_states().len(), 1);
        let s = svc.server_state("test1").unwrap();
        assert_eq!(s.connection_state, McpConnectionState::Disconnected);
    }

    #[test]
    fn set_enabled_toggles_state() {
        let svc = McpService::new();
        let mut settings = McpSettings::default();
        settings.add_server(McpServerConfig {
            id: "test".into(),
            display_name: "Test".into(),
            description: "".into(),
            enabled: true,
            auto_start: false,
            transport: McpTransport::Stdio { command: "echo".into(), args: vec![], env: vec![] },
        });
        svc.register_servers(&settings);
        svc.set_enabled("test", false).unwrap();
        let s = svc.server_state("test").unwrap();
        assert_eq!(s.connection_state, McpConnectionState::Disabled);
    }

    #[test]
    fn status_summary_counts_connected() {
        let svc = McpService::new();
        let mut settings = McpSettings::default();
        settings.add_server(McpServerConfig {
            id: "s1".into(),
            display_name: "S1".into(),
            description: "".into(),
            enabled: true,
            auto_start: false,
            transport: McpTransport::Stdio { command: "cmd".into(), args: vec![], env: vec![] },
        });
        svc.register_servers(&settings);
        let summary = svc.status_summary();
        assert_eq!(summary.total, 1);
        assert_eq!(summary.connected, 0);
        assert!(!summary.is_any_connected());
    }
}
