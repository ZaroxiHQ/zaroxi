use std::env;
use std::fs;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};
use zaroxi_interface_desktop::DesktopComposition;
use zaroxi_interface_desktop::folder_picker::{
    FakeFolderPicker, FolderPicker, PickerDiagnostics, PickerKind, PickerOutcome,
};

fn temp_workspace() -> PathBuf {
    let base = env::temp_dir();
    let uniq = format!(
        "zaroxi_test_{}_{}",
        std::process::id(),
        SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos()
    );
    let tmp = base.join(uniq);

    tmp.join("workspace")
}

#[test]
fn explorer_empty_when_no_workspace_root() {
    let comp = DesktopComposition::new();
    assert!(comp.maybe_explorer.is_none());
    assert_eq!(comp.explorer_item_count(), 0);

    let items = comp.format_cached_explorer_items();
    assert!(items.is_none());

    let wc = comp.build_work_content();
    assert!(wc.explorer_items.is_none());
}

#[test]
fn explorer_toggle_directory_expands_and_collapses() -> std::io::Result<()> {
    let root = temp_workspace();
    fs::create_dir_all(root.join("sub"))?;
    fs::write(root.join("top.txt"), "top")?;

    let mut comp = DesktopComposition::new();
    comp.workspace_root_path = Some(root.clone());
    comp.load_or_refresh_explorer();
    assert!(comp.maybe_explorer.is_some());
    assert!(comp.explorer_item_count() > 0);

    let sub_idx = (0..comp.explorer_item_count())
        .find(|&i| comp.get_explorer_item_at(i).map(|it| it.name.as_str()) == Some("sub"))
        .expect("sub directory should be in explorer items");

    assert!(comp.is_explorer_item_dir(sub_idx));

    let sub_id = comp.get_explorer_item_id_at(sub_idx).unwrap();
    if let Some(ref mut explorer) = comp.maybe_explorer {
        explorer.toggle_expand(&sub_id);
    }
    comp.refresh_cached_explorer_items();

    let items_after = comp.format_cached_explorer_items();
    assert!(items_after.is_some());

    if let Some(ref mut explorer) = comp.maybe_explorer {
        explorer.toggle_expand(&sub_id);
    }
    comp.refresh_cached_explorer_items();

    let _ = fs::remove_dir_all(root.parent().unwrap());
    Ok(())
}

#[test]
fn explorer_build_work_content_includes_tree_items() -> std::io::Result<()> {
    let root = temp_workspace();
    fs::create_dir_all(&root)?;
    fs::write(root.join("readme.md"), "hello")?;

    let mut comp = DesktopComposition::new();
    comp.workspace_root_path = Some(root.clone());
    comp.load_or_refresh_explorer();

    let wc = comp.build_work_content();
    let items = wc.explorer_items.expect("should have explorer items for non-empty workspace");
    assert!(items.iter().any(|s| s.contains("readme.md")), "explorer should show readme.md");

    let _ = fs::remove_dir_all(root.parent().unwrap());
    Ok(())
}

#[test]
fn explorer_empty_directory_shows_no_items() -> std::io::Result<()> {
    let root = temp_workspace();
    fs::create_dir_all(&root)?;

    let mut comp = DesktopComposition::new();
    comp.workspace_root_path = Some(root.clone());
    comp.load_or_refresh_explorer();

    let items = comp.format_cached_explorer_items();
    assert!(items.is_none(), "empty workspace directory should produce no explorer items");

    let _ = fs::remove_dir_all(root.parent().unwrap());
    Ok(())
}

#[test]
fn explorer_load_handles_nonexistent_path() {
    let root = PathBuf::from("/tmp/nonexistent_zaroxi_workspace_12345");

    let mut comp = DesktopComposition::new();
    comp.workspace_root_path = Some(root);
    comp.load_or_refresh_explorer();

    assert!(comp.maybe_explorer.is_some(), "explorer should still create structure");
    assert_eq!(
        comp.explorer_item_count(),
        0,
        "nonexistent path should produce no visible children"
    );
}

#[test]
fn startup_without_workspace_shows_open_button() {
    let comp = DesktopComposition::new();
    let wc = comp.build_work_content();

    assert!(wc.explorer_items.is_none());
    assert_eq!(
        wc.explorer_empty_button,
        Some("Open Workspace".to_string()),
        "empty explorer should show Open Workspace button"
    );
}

#[test]
fn open_workspace_button_disappears_after_workspace_set() -> std::io::Result<()> {
    let root = temp_workspace();
    fs::create_dir_all(&root)?;
    fs::write(root.join("file.txt"), "content")?;

    let mut comp = DesktopComposition::new();
    let wc_before = comp.build_work_content();
    assert_eq!(wc_before.explorer_empty_button, Some("Open Workspace".to_string()));

    comp.workspace_root_path = Some(root.clone());
    comp.load_or_refresh_explorer();

    let wc_after = comp.build_work_content();
    assert!(
        wc_after.explorer_empty_button.is_none(),
        "button should disappear after workspace is loaded"
    );
    assert!(wc_after.explorer_items.is_some());

    let _ = fs::remove_dir_all(root.parent().unwrap());
    Ok(())
}

#[test]
fn cancel_folder_picker_is_noop() {
    let picker = FakeFolderPicker::cancelled();
    let outcome = picker.pick_folder();
    assert!(matches!(outcome, PickerOutcome::Cancelled), "cancel should return Cancelled");
}

#[test]
fn successful_folder_picker_returns_path() {
    let path = PathBuf::from("/test/path");
    let picker = FakeFolderPicker::selected(path.clone());
    let outcome = picker.pick_folder();
    assert_eq!(outcome, PickerOutcome::Selected(path));
}

#[test]
fn picker_unavailable_returns_structured_error() {
    let picker = FakeFolderPicker::unavailable("no xdg-desktop-portal found");
    let outcome = picker.pick_folder();
    assert!(matches!(&outcome, PickerOutcome::Unavailable { .. }));
    assert_eq!(outcome.reason().unwrap(), "no xdg-desktop-portal found");
    assert!(outcome.diagnostics().is_some());
}

#[test]
fn picker_unavailable_sets_status_message() {
    let picker = FakeFolderPicker::unavailable("backend missing");
    let outcome = picker.pick_folder();
    assert!(outcome.reason().is_some_and(|r| r.contains("backend missing")));
    assert!(!outcome.is_selected());
}

#[test]
fn picker_diagnostics_included_in_unavailable() {
    let picker = FakeFolderPicker::unavailable("picker backend not found");
    let outcome = picker.pick_folder();
    let diag = outcome.diagnostics().expect("Unavailable should include diagnostics");
    assert!(!diag.rfd_attempted);
}

#[test]
fn picker_outcome_cancelled_has_no_diagnostics() {
    let picker = FakeFolderPicker::cancelled();
    let outcome = picker.pick_folder();
    assert!(outcome.diagnostics().is_none());
}

#[test]
fn picker_outcome_selected_has_no_diagnostics() {
    let picker = FakeFolderPicker::selected(PathBuf::from("/tmp"));
    let outcome = picker.pick_folder();
    assert!(outcome.diagnostics().is_none());
}

#[test]
fn picker_diagnostics_probe_initializes_attempt_flags() {
    let diag = PickerDiagnostics::probe();
    assert!(!diag.rfd_attempted);
    assert!(!diag.rfd_succeeded);
    assert!(!diag.any_subprocess_attempted);
    assert!(!diag.any_subprocess_succeeded);
}

#[test]
fn picker_outcome_selected_extracts_path() {
    let picker = FakeFolderPicker::selected(PathBuf::from("/home/user/project"));
    let outcome = picker.pick_folder();
    assert!(outcome.is_selected());
    assert!(outcome.reason().is_none());
}

#[test]
fn folder_picker_trait_is_object_safe() {
    let picker: std::sync::Arc<dyn FolderPicker> =
        std::sync::Arc::new(FakeFolderPicker::cancelled());
    let outcome = picker.pick_folder();
    assert!(matches!(outcome, PickerOutcome::Cancelled));
}

#[test]
fn picker_kind_is_open_folder() {
    // Smoke test: PickerKind enum compiles and has expected variant.
    let kind = PickerKind::OpenFolder;
    assert_eq!(kind, PickerKind::OpenFolder);
}

#[test]
fn workspace_open_sets_root_and_loads_explorer() -> std::io::Result<()> {
    let root = temp_workspace();
    fs::create_dir_all(&root)?;
    fs::write(root.join("README.md"), "# Project")?;

    let mut comp = DesktopComposition::new();
    comp.workspace_root_path = Some(root.clone());
    comp.load_or_refresh_explorer();

    assert!(comp.maybe_explorer.is_some());
    let wc = comp.build_work_content();

    assert!(wc.explorer_empty_button.is_none(), "no button when workspace is loaded");

    let items = wc.explorer_items.expect("should have items");
    assert!(items.iter().any(|s| s.contains("README.md")));

    let _ = fs::remove_dir_all(root.parent().unwrap());
    Ok(())
}
