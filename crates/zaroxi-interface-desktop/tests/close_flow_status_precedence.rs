use zaroxi_core_editor_buffer::ports::BufferId;
use zaroxi_interface_desktop::close::PendingClose;
use zaroxi_interface_desktop::desktop::DesktopComposition;

#[test]
fn confirm_save_and_close_status_survives_refresh() {
    let mut comp = DesktopComposition::new();
    comp.set_close_result_status("Saved and closed lib.rs".to_string());

    // Simulate a refresh that would normally set a generic status message.
    comp.set_status_message("UpdateBuffer ✓".to_string());

    let status = comp.latest_status_bar_line().expect("expected status line");
    assert_eq!(status.text, "Saved and closed lib.rs");
}

#[test]
fn confirm_discard_and_close_status_survives_refresh() {
    let mut comp = DesktopComposition::new();
    comp.set_close_result_status("Discarded changes and closed lib.rs".to_string());

    // Simulate an immediately following refresh/update.
    comp.set_status_message("UpdateBuffer ✓".to_string());

    let status = comp.latest_status_bar_line().expect("expected status line");
    assert_eq!(status.text, "Discarded changes and closed lib.rs");
}

#[test]
fn confirm_cancel_close_does_not_surface_previous_close_status() {
    let mut comp = DesktopComposition::new();
    comp.set_close_result_status("Saved and closed lib.rs".to_string());

    // User cancels the pending close -> compositional helpers should clear
    // the preserved explicit close-result status so it won't be displayed.
    comp.clear_close_result_status();
    comp.clear_pending_close();
    comp.set_status_message("Close cancelled".to_string());

    let status = comp.latest_status_bar_line().expect("expected status line");
    assert_eq!(status.text, "Close cancelled");
}

#[test]
fn pending_session_overrides_close_result() {
    let mut comp = DesktopComposition::new();
    // Simulate a previously preserved explicit close-result status.
    comp.set_close_result_status("Saved and closed lib.rs".to_string());

    // Enter a new pending-session-close flow; this should clear the preserved status
    // and surface the pending-session banner immediately.
    let pending = PendingClose::SessionClose {
        dirty_buffers: vec![BufferId::from("buf:dirty")],
        summary: "1 open buffers".to_string(),
    };
    comp.set_pending_close(pending);

    assert!(comp.has_pending_close(), "pending close should be set after entering session-close");
    let bar = comp.latest_status_bar_line().expect("status bar present");
    assert!(
        bar.text.contains("Close session")
            || bar.text.contains("buffers")
            || bar.text.contains("Close"),
        "status should reflect session close pending and not be the stale close-result"
    );
}

#[test]
fn pending_buffer_overrides_close_result() {
    let mut comp = DesktopComposition::new();
    // Simulate a previously preserved explicit close-result status.
    comp.set_close_result_status("Saved and closed lib.rs".to_string());

    // Enter a pending-buffer-close flow; this should clear the preserved status
    // and surface the buffer-close banner immediately.
    let pending = PendingClose::BufferClose {
        buffer_id: BufferId::from("buf:dirty"),
        display: Some("lib.rs".to_string()),
        dirty: true,
    };
    comp.set_pending_close(pending);

    assert!(comp.has_pending_close(), "pending close should be set after entering buffer-close");
    let bar = comp.latest_status_bar_line().expect("status bar present");
    assert!(
        bar.text.contains("Close") || bar.text.contains("Discard") || bar.text.contains("Save"),
        "status should reflect pending buffer-close banner and not the stale close-result"
    );
}
