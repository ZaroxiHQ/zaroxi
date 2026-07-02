/// Tiny, read-only frame model that composes existing shell-facing projections into a
/// single semantic frame suitable for later rendering by app/engine layers.
///
/// Constraints respected:
/// - Non-visual (no geometry, colors, fonts, layout).
/// - Composes existing projection accessors only.
/// - Minimal: single struct + two small constructors + compact summary helper.
#[derive(Debug, Clone)]
pub struct ShellFrameModel {
    /// Optional rendered session identity (when available from composition metadata).
    pub session_identity: Option<String>,

    /// Optional rendered shell chrome (when the shell snapshot + status allow composition).
    pub shell_chrome: Option<String>,

    /// Optional active text view (semantic, read-only visible text model).
    pub active_text_view: Option<crate::TextView>,

    /// Optional selection view (semantic selection bounds, when present).
    pub selection_view: Option<crate::SelectionView>,

    /// Optional viewport summary rendered as a small string (semantic).
    pub viewport_summary: Option<String>,

    /// Optional status text (when present).
    pub status_text: Option<String>,

    /// Optional last command line rendered string (when present).
    pub last_command: Option<String>,

    /// Optional last event rendered string (when present).
    pub last_event: Option<String>,
}

impl ShellFrameModel {
    /// Construct the frame model from explicit parts.
    ///
    /// Lifecycle rule enforced: the frame is considered absent (returns None)
    /// when the mandatory visible document (TextView) is absent.
    pub fn from_parts(
        active_text_view: Option<crate::TextView>,
        selection_view: Option<crate::SelectionView>,
        viewport_summary: Option<String>,
        status_text: Option<String>,
        last_command: Option<String>,
        last_event: Option<String>,
        shell_chrome: Option<String>,
        session_identity: Option<String>,
    ) -> Option<Self> {
        // Mandatory piece: active_text_view must be present for a meaningful frame.
        active_text_view.as_ref()?;

        Some(Self {
            session_identity,
            shell_chrome,
            active_text_view,
            selection_view,
            viewport_summary,
            status_text,
            last_command,
            last_event,
        })
    }

    /// Compose a ShellFrameModel from a DesktopComposition snapshot.
    ///
    /// Returns None when mandatory pieces are absent (same lifecycle rule as from_parts).
    pub fn from_composition(comp: &crate::DesktopComposition) -> Option<Self> {
        // Active text view is the primary required piece.
        let tv = crate::TextView::from_composition(comp);

        // Short-circuit if no visible text.
        tv.as_ref()?;

        let sel = crate::SelectionView::from_composition(comp);

        // Build a compact viewport summary string when available.
        let viewport_summary = comp.latest_viewport_summary().map(|vs| {
            format!(
                "top_visible_line={} visible_line_count={} total_lines={} cursor_visible={} anchoring={:?}",
                vs.top_visible_line, vs.visible_line_count, vs.total_lines, vs.cursor_visible, vs.anchoring
            )
        });

        let status_text = comp.latest_status_bar_line().map(|s| s.text.clone());

        let last_command =
            comp.latest_shell_context().and_then(|ctx| ctx.last_command_line.clone());

        // last_event: produce a tiny rendered summary from the most recent event when available.
        let last_event = comp.latest_shell_snapshot().and_then(|ss| {
            // Use the most recent event timestamp and kind when present via the snapshot's recent events view.
            // Fallback conservatively to None if not present/exposed.
            // We attempt a defensive access that tolerates multiple internal shapes.
            ss.context.latest_refresh_reason.as_ref().map(|r| format!("refresh_reason={:?}", r))
        });

        // Attempt to compose a rendered shell chrome when a shell snapshot exists.
        let shell_chrome = comp.latest_shell_snapshot().and_then(|ss| {
            // Compose using existing projection composer; tolerate absence.
            crate::projections::shell_chrome_snapshot::ShellChromeSnapshot::compose(
                // session identity: unknown here (composition may not expose a rendered session identity),
                None,
                // active buffer and location derived from shell snapshot
                crate::projections::active_buffer_line::ActiveBufferLine::from_shell_snapshot(&ss)
                    .map(|l| l.render()),
                crate::projections::location_line::LocationLine::from_shell_snapshot(&ss)
                    .map(|l| l.render()),
                // status text from latest status bar line when present
                comp.latest_status_bar_line().map(|s| s.text.clone()),
                // last command optional (none)
                None,
            )
            .map(|c| c.render())
        });

        // session identity: ShellContext does not expose session_id. Leave absent to avoid coupling.
        // Composition may expose session identity elsewhere; keep this optional and conservative.
        let session_identity = None;

        Self::from_parts(
            tv,
            sel,
            viewport_summary,
            status_text,
            last_command,
            last_event,
            shell_chrome,
            session_identity,
        )
    }

    /// Return a compact one-line summary useful for harness printing.
    pub fn compact_summary(&self) -> String {
        let active = self
            .active_text_view
            .as_ref()
            .map(|tv| format!("top_line={} total_lines={}", tv.top_line, tv.total_lines))
            .unwrap_or_else(|| "<no-active-text>".to_string());
        let sel = self
            .selection_view
            .as_ref()
            .map(|s| format!("sel={}..{}", s.start.line, s.end.line))
            .unwrap_or_else(|| "<no-selection>".to_string());
        let vp = self.viewport_summary.clone().unwrap_or_else(|| "<no-viewport>".to_string());
        let status = self.status_text.clone().unwrap_or_else(|| "<no-status>".to_string());
        let chrome = self.shell_chrome.clone().unwrap_or_else(|| "<no-chrome>".to_string());
        format!(
            "ShellFrameModel{{ {}, {}, vp={}, status={}, chrome={} }}",
            active, sel, vp, status, chrome
        )
    }
}
