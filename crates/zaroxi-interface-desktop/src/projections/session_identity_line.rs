/// Tiny, read-only shell-facing projection that summarizes the current
/// session/workspace identity in a single concise line suitable for shells.
///
/// This module intentionally avoids introducing any framework or broad
/// abstraction: it's a local adapter concern that formats existing ids/paths.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SessionIdentityLine {
    /// Session identifier (e.g. kernel/session id).
    pub session_id: Option<String>,
    /// Workspace identifier (if any).
    pub workspace_id: Option<String>,
    /// Optional workspace path or display name (when available).
    pub workspace_path: Option<String>,
}

impl SessionIdentityLine {
    /// Create a new identity line from optional string parts.
    /// Accepts owned Strings so callers (e.g. harness) can pass readily available values.
    pub fn new(
        session_id: Option<String>,
        workspace_id: Option<String>,
        workspace_path: Option<String>,
    ) -> Self {
        Self { session_id, workspace_id, workspace_path }
    }

    /// Whether the projection contains no identity information.
    pub fn is_empty(&self) -> bool {
        self.session_id.is_none() && self.workspace_id.is_none() && self.workspace_path.is_none()
    }

    /// Render a concise shell-friendly line. Uses "<none>" for absent parts.
    /// Example: "session=sess-123 workspace=ws-1 path=/path/to/ws"
    pub fn render(&self) -> String {
        let sid = self.session_id.as_deref().unwrap_or("<none>");
        let wid = self.workspace_id.as_deref().unwrap_or("<none>");
        let path = self.workspace_path.as_deref().unwrap_or("<none>");
        format!("session={} workspace={} path={}", sid, wid, path)
    }
}
