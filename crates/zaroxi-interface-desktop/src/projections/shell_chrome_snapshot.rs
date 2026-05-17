#![allow(clippy::missing_const_for_fn)]

// Tiny, explicit shell-facing aggregate that composes existing shell lines only.
//
// Lifecycle rule (strict): present only when all mandatory composed lines
// (SessionIdentityLine, ActiveBufferLine, LocationLine, StatusBarLine)
// are present; otherwise no snapshot is produced.

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ShellChromeSnapshot {
    /// Rendered session identity (stable, shell-facing string).
    pub session: String,
    /// Rendered active buffer line (stable, shell-facing string).
    pub active_buffer: String,
    /// Rendered location (e.g. "12:3").
    pub location: String,
    /// Rendered status bar text.
    pub status: String,
    /// Optional last command line rendered text (may be None).
    pub last_command: Option<String>,
}

impl ShellChromeSnapshot {
    /// Compose a ShellChromeSnapshot from already-rendered shell line strings.
    ///
    /// Returns Some(snapshot) only when the four mandatory components are present:
    /// session, active_buffer, location, and status. last_command is optional.
    pub fn compose(
        session_rendered: Option<String>,
        active_rendered: Option<String>,
        location_rendered: Option<String>,
        status_rendered: Option<String>,
        last_command_rendered: Option<String>,
    ) -> Option<Self> {
        match (session_rendered, active_rendered, location_rendered, status_rendered) {
            (Some(session), Some(active), Some(location), Some(status)) => Some(Self {
                session,
                active_buffer: active,
                location,
                status,
                last_command: last_command_rendered,
            }),
            _ => None,
        }
    }

    /// Render a compact one-line representation suitable for tiny shell output.
    pub fn render(&self) -> String {
        if let Some(ref lc) = self.last_command {
            format!("{} │ {} │ {} │ {} │ last: {}", self.session, self.active_buffer, self.location, self.status, lc)
        } else {
            format!("{} │ {} │ {} │ {}", self.session, self.active_buffer, self.location, self.status)
        }
    }
}
