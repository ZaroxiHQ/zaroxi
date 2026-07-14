//! MCP (Model Context Protocol) foundation — domain types for server registration,
//! configuration, lifecycle management, and tool capability listing.
//!
//! Phase 1 establishes the config model and connection lifecycle. Actual
//! transport (stdio/SSE) and JSON-RPC message handling belong in the
//! infrastructure layer in a later phase.

use serde::{Deserialize, Serialize};

/// Identifier for an MCP server instance.
pub type McpServerId = String;

/// Transport mechanism for MCP server communication.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum McpTransport {
    /// Spawn a child process and communicate over stdin/stdout (JSON-RPC).
    Stdio { command: String, args: Vec<String>, env: Vec<(String, String)> },
    /// Connect to a running server via SSE + HTTP POST.
    Sse { url: String, headers: Vec<(String, String)> },
}

/// Configuration for a single MCP server.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct McpServerConfig {
    /// Unique stable identifier for this server (user-chosen slug).
    pub id: McpServerId,
    /// Human-readable name.
    pub display_name: String,
    /// Short description of what the server provides.
    pub description: String,
    /// Whether the server is enabled (should be started on app launch).
    pub enabled: bool,
    /// Transport configuration.
    pub transport: McpTransport,
    /// Auto-start on application launch.
    pub auto_start: bool,
}

/// The connection/health state of an MCP server.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum McpConnectionState {
    /// Never attempted or configured.
    Disconnected,
    /// Initialization / handshake in progress.
    Connecting,
    /// Connected and ready to serve tools.
    Connected,
    /// Connection failed (carries a soft error description).
    Error,
    /// Explicitly disabled by the user.
    Disabled,
}

/// A single tool capability exposed by an MCP server.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct McpTool {
    /// Unique tool name (as reported by the server).
    pub name: String,
    /// Human-readable description.
    pub description: String,
    /// JSON Schema for the tool's input parameters (stringified).
    pub input_schema: Option<String>,
}

/// Runtime state for a single MCP server.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct McpServerState {
    pub config: McpServerConfig,
    pub connection_state: McpConnectionState,
    /// Tools reported by the server after initialization.
    pub tools: Vec<McpTool>,
    /// Error message, if connection_state is Error.
    pub error_message: Option<String>,
}

impl McpServerState {
    pub fn new(config: McpServerConfig) -> Self {
        Self {
            config,
            connection_state: McpConnectionState::Disconnected,
            tools: Vec::new(),
            error_message: None,
        }
    }

    pub fn is_ready(&self) -> bool {
        self.connection_state == McpConnectionState::Connected && !self.tools.is_empty()
    }
}

/// Aggregate MCP configuration — all registered servers.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct McpSettings {
    /// Registered MCP servers.
    pub servers: Vec<McpServerConfig>,
}

impl McpSettings {
    pub fn add_server(&mut self, config: McpServerConfig) {
        self.servers.retain(|s| s.id != config.id);
        self.servers.push(config);
    }

    pub fn remove_server(&mut self, id: &str) {
        self.servers.retain(|s| s.id != id);
    }

    pub fn get_server(&self, id: &str) -> Option<&McpServerConfig> {
        self.servers.iter().find(|s| s.id == id)
    }

    pub fn get_server_mut(&mut self, id: &str) -> Option<&mut McpServerConfig> {
        self.servers.iter_mut().find(|s| s.id == id)
    }

    pub fn enabled_servers(&self) -> Vec<&McpServerConfig> {
        self.servers.iter().filter(|s| s.enabled).collect()
    }
}

/// Well-known MCP server presets for phase-1 scaffolding.
pub mod presets {
    use super::*;

    /// Filesystem MCP server — allows the AI to read/write within the workspace.
    pub fn filesystem_server(workspace_root: &str) -> McpServerConfig {
        McpServerConfig {
            id: "filesystem".into(),
            display_name: "Filesystem".into(),
            description: "Read, write, and list files in the workspace".into(),
            enabled: false,
            auto_start: false,
            transport: McpTransport::Stdio {
                command: "npx".into(),
                args: vec![
                    "-y".into(),
                    "@anthropic-ai/mcp-server-filesystem".into(),
                    workspace_root.to_string(),
                ],
                env: Vec::new(),
            },
        }
    }

    /// Git MCP server — allows the AI to inspect git state.
    pub fn git_server(workspace_root: &str) -> McpServerConfig {
        McpServerConfig {
            id: "git".into(),
            display_name: "Git".into(),
            description: "Read git history, diffs, and branch info".into(),
            enabled: false,
            auto_start: false,
            transport: McpTransport::Stdio {
                command: "npx".into(),
                args: vec![
                    "-y".into(),
                    "@anthropic-ai/mcp-server-git".into(),
                    "--repository".into(),
                    workspace_root.to_string(),
                ],
                env: Vec::new(),
            },
        }
    }

    /// Browser MCP server — allows the AI to fetch web content.
    pub fn fetch_server() -> McpServerConfig {
        McpServerConfig {
            id: "fetch".into(),
            display_name: "Web Fetch".into(),
            description: "Fetch URLs and parse web content".into(),
            enabled: false,
            auto_start: false,
            transport: McpTransport::Stdio {
                command: "npx".into(),
                args: vec!["-y".into(), "@anthropic-ai/mcp-server-fetch".into()],
                env: Vec::new(),
            },
        }
    }

    /// All available presets.
    pub fn all_presets(workspace_root: &str) -> Vec<McpServerConfig> {
        vec![filesystem_server(workspace_root), git_server(workspace_root), fetch_server()]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mcp_server_state_defaults_disconnected() {
        let cfg = McpServerConfig {
            id: "test".into(),
            display_name: "Test".into(),
            description: "A test server".into(),
            enabled: true,
            auto_start: false,
            transport: McpTransport::Stdio {
                command: "echo".into(),
                args: vec!["hello".into()],
                env: Vec::new(),
            },
        };
        let state = McpServerState::new(cfg.clone());
        assert_eq!(state.connection_state, McpConnectionState::Disconnected);
        assert!(state.tools.is_empty());
        assert!(!state.is_ready());
    }

    #[test]
    fn mcp_settings_add_remove_server() {
        let mut s = McpSettings::default();
        let cfg = McpServerConfig {
            id: "srv1".into(),
            display_name: "Server 1".into(),
            description: "First".into(),
            enabled: true,
            auto_start: false,
            transport: McpTransport::Stdio { command: "cmd".into(), args: vec![], env: vec![] },
        };
        s.add_server(cfg.clone());
        assert_eq!(s.servers.len(), 1);
        assert!(s.get_server("srv1").is_some());

        s.remove_server("srv1");
        assert!(s.servers.is_empty());
        assert!(s.get_server("srv1").is_none());
    }

    #[test]
    fn mcp_settings_enabled_filter() {
        let mut s = McpSettings::default();
        let enabled = McpServerConfig {
            id: "e".into(),
            display_name: "Enabled".into(),
            description: "".into(),
            enabled: true,
            auto_start: false,
            transport: McpTransport::Stdio { command: "cmd".into(), args: vec![], env: vec![] },
        };
        let disabled = McpServerConfig { id: "d".into(), enabled: false, ..enabled.clone() };
        s.add_server(enabled.clone());
        s.add_server(disabled);
        assert_eq!(s.enabled_servers().len(), 1);
        assert_eq!(s.enabled_servers()[0].id, "e");
    }
}
