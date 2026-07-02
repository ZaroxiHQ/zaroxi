// Status-line and transient status message derivation.
// This function centralizes the tiny shell-facing status line logic that the
// main DesktopComposition.latest_status_bar_line previously contained.

pub(crate) fn latest_status_bar_line(
    comp: &super::DesktopComposition,
) -> Option<super::StatusBarLine> {
    // Pending-close banner takes precedence.
    if let Some(pc) = comp.pending_close.as_ref() {
        // Local copy of the earlier logic.
        let mut hint = "[S]ave [D]iscard [C]ancel".to_string();
        if let super::PendingClose::SessionClose { .. } = pc {
            hint = "[S]ave all [D]iscard all [C]ancel".to_string();
        }
        let text = format!("{}  {}", pc.render_summary(), hint);
        return Some(super::StatusBarLine { text, sticky: Some("pending-close".to_string()) });
    }

    // Transient last-command-line status takes next precedence.
    if let Some(m) = comp.metadata.as_ref()
        && let Some(ref last) = m.last_command_line
    {
        let text = last.clone();
        return Some(super::StatusBarLine { text, sticky: Some("status-message".to_string()) });
    }

    // Fallback to the pre-existing presenter-driven status_bar helper.
    super::status_bar::latest_status_bar_line(comp)
}
