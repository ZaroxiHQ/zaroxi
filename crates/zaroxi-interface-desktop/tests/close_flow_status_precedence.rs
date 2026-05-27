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
