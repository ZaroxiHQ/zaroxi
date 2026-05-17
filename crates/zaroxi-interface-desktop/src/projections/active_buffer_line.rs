/// Tiny, read-only shell-facing projection that answers "what buffer am I currently on?"
///
/// Lifecycle rule implemented (Phase 43):
/// "ActiveBufferLine is absent before the first refresh and present after the first
/// refresh if there is an active buffer."
///
/// This module intentionally remains adapter-local and lightweight. It composes only
/// from the ShellSnapshot's shell context (revision and active buffer/display).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ActiveBufferLine {
    /// Active buffer identifier (kernel/workspace buffer id).
    pub buffer_id: String,
    /// Optional display name or path (when available).
    pub display: Option<String>,
}

impl ActiveBufferLine {
    /// Create a new ActiveBufferLine from raw parts.
    /// This helper is intentionally public to enable tests to exercise the lifecycle rule
    /// without constructing a full ShellSnapshot.
    pub fn from_parts(latest_revision: u64, active_buffer: Option<String>, active_display: Option<String>) -> Option<Self> {
        // Lifecycle rule: absent before the first refresh (revision == 0).
        if latest_revision == 0 {
            return None;
        }

        let buf = active_buffer?;
        Some(Self {
            buffer_id: buf,
            display: active_display,
        })
    }

    /// Compose from the authoritative shell-facing snapshot.
    /// Respects the same lifecycle rule: returns None if the snapshot indicates no refresh yet
    /// or there is no active buffer.
    pub fn from_shell_snapshot(snapshot: &crate::ShellSnapshot) -> Option<Self> {
        // Map into the minimal parts we need. This keeps the projection local and
        // avoids depending on multiple other projections.
        let rev = snapshot.context.latest_revision;
        // Convert the active buffer identifier into an owned String.
        // Many buffer id types in the codebase are tuple structs like `BufferId(pub String)`,
        // so extract the inner string when available. Clone to avoid borrowing the snapshot.
        let active = snapshot
            .context
            .active_buffer
            .as_ref()
            .map(|b| b.0.clone());
        let display = snapshot.context.active_display.clone();
        Self::from_parts(rev, active, display)
    }

    /// Render a concise shell-friendly line. Uses "<none>" for absent display.
    /// Example: "active_buffer=buf-123 display=main.rs"
    pub fn render(&self) -> String {
        let disp = self.display.as_deref().unwrap_or("<none>");
        format!("active_buffer={} display={}", self.buffer_id, disp)
    }

    /// Convenience: is the projection empty? (shouldn't be if constructed via from_parts)
    pub fn is_empty(&self) -> bool {
        self.buffer_id.is_empty()
    }
}
