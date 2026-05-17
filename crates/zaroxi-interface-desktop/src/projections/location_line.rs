/// Tiny, read-only shell-facing projection that answers "where is the cursor?"
///
/// Lifecycle rule: absent before the first refresh and present after the first refresh
/// when an active document cursor exists.
///
/// This projection is adapter-local and composes from the ShellSnapshot (already provided
/// by the DesktopComposition). It intentionally does not introduce any framework or
/// abstraction — a single tiny accessor and renderer only.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LocationLine {
    /// 1-based line number of the cursor (kept as u32 to match other projections).
    pub line: u32,
    /// 0-based column index in characters.
    pub column: u32,
    /// Optional display name for the active buffer (when available).
    pub display: Option<String>,
}

impl LocationLine {
    /// Create a LocationLine from the minimal raw parts.
    /// Returns None when the lifecycle rule indicates absence:
    /// - absent before the first refresh (latest_revision == 0)
    /// - absent when cursor/document info is missing
    pub fn from_parts(latest_revision: u64, cursor_line: Option<u32>, cursor_column: Option<u32>, display: Option<String>) -> Option<Self> {
        // Lifecycle: absent before first refresh.
        if latest_revision == 0 {
            return None;
        }

        // Require both cursor line and column to consider the projection present.
        let line = cursor_line?;
        let column = cursor_column?;
        Some(Self {
            line,
            column,
            display,
        })
    }

    /// Compose the projection from the authoritative ShellSnapshot.
    /// Uses snapshot.context.latest_revision for the lifecycle decision and reads
    /// cursor coordinates from snapshot.active_document. The active buffer display
    /// is taken from the shell context (snapshot.context.active_display) when available.
    pub fn from_shell_snapshot(snapshot: &crate::ShellSnapshot) -> Option<Self> {
        let rev = snapshot.context.latest_revision;
        let line = snapshot.active_document.as_ref().and_then(|d| d.cursor_line);
        let column = snapshot.active_document.as_ref().and_then(|d| d.cursor_column);
        let display = snapshot.context.active_display.clone();
        Self::from_parts(rev, line, column, display)
    }

    /// Render a concise shell-friendly line.
    /// Examples:
    /// - "cursor=10:5 display=main.rs"
    /// - "cursor=10:5"
    pub fn render(&self) -> String {
        if let Some(ref d) = self.display {
            format!("cursor={}:{} display={}", self.line, self.column, d)
        } else {
            format!("cursor={}:{}", self.line, self.column)
        }
    }

    /// Convenience: is the projection empty? (shouldn't be if constructed via from_parts)
    pub fn is_empty(&self) -> bool {
        false
    }
}
