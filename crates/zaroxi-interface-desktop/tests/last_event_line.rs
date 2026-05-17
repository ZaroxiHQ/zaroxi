#[cfg(test)]
mod tests {
    use zaroxi_interface_desktop::projections::last_event_line::{summarize_event_kind, summarize_last_event};
    use zaroxi_interface_desktop::ports::BufferId;
    use zaroxi_interface_desktop::ports::WorkspaceEventKind;
    use std::path::PathBuf;

    #[test]
    fn no_events_returns_no_events() {
        let le = summarize_last_event(None);
        assert_eq!(le.text, "No events");
    }

    #[test]
    fn buffer_opened_shows_path() {
        // Construct a BufferOpened kind using a dummy path. We don't rely on buffer id formatting here.
        let kind = WorkspaceEventKind::BufferOpened {
            buffer_id: BufferId("buf:1".to_string()), // harmless dummy id; path is sufficient for the summary
            path: PathBuf::from("src/main.rs"),
        };
        let s = summarize_event_kind(&kind);
        assert_eq!(s, "BufferOpened: src/main.rs");
    }
}
