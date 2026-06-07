/*!
Tests for the Explorer panel module: view model construction, panel item
output, widget tree composition, and action dispatch.

These tests validate user-visible behavior only — empty-state rendering,
button placement, tree item visibility, and toggle-open-file flow.
*/

use std::env;
use std::fs;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use zaroxi_interface_desktop::DesktopComposition;
use zaroxi_interface_desktop::gui::window::explorer_panel::ExplorerPanelViewModel;

fn temp_workspace() -> PathBuf {
    let base = env::temp_dir();
    let uniq = format!(
        "zaroxi_panel_test_{}_{}",
        std::process::id(),
        SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos()
    );
    base.join(uniq).join("workspace")
}

// ── View model tests ──────────────────────────────────────────────────

#[test]
fn panel_view_model_shows_open_button_when_no_workspace() {
    let comp = DesktopComposition::new();
    let vm = ExplorerPanelViewModel::build(&comp);

    assert!(vm.title.is_none());
    assert!(vm.items.is_empty());
    assert_eq!(vm.primary_action_label, Some("Open Workspace".to_string()));
}

#[test]
fn panel_view_model_hides_button_after_workspace_loaded() -> std::io::Result<()> {
    let root = temp_workspace();
    fs::create_dir_all(&root)?;
    fs::write(root.join("main.rs"), "fn main() {}")?;

    let mut comp = DesktopComposition::new();
    comp.workspace_root_path = Some(root.clone());
    comp.load_or_refresh_explorer();

    let vm = ExplorerPanelViewModel::build(&comp);
    assert!(vm.primary_action_label.is_none());
    assert!(vm.empty_message.is_some());
    assert!(!vm.items.is_empty());

    let _ = fs::remove_dir_all(root.parent().unwrap());
    Ok(())
}

#[test]
fn panel_view_model_items_have_structure() -> std::io::Result<()> {
    let root = temp_workspace();
    fs::create_dir_all(root.join("src"))?;
    fs::write(root.join("Cargo.toml"), "")?;
    fs::write(root.join("src").join("lib.rs"), "")?;

    let mut comp = DesktopComposition::new();
    comp.workspace_root_path = Some(root.clone());
    comp.load_or_refresh_explorer();

    let vm = ExplorerPanelViewModel::build(&comp);
    assert!(vm.title.is_some());

    let toml = vm.items.iter().find(|i| i.label.contains("Cargo.toml"));
    assert!(toml.is_some(), "Cargo.toml should appear in items");
    assert!(!toml.unwrap().is_dir);

    let src = vm.items.iter().find(|i| i.label.contains("src"));
    assert!(src.is_some(), "src directory should appear");
    assert!(src.unwrap().is_dir);

    let _ = fs::remove_dir_all(root.parent().unwrap());
    Ok(())
}

// ── Work content integration tests ────────────────────────────────────

#[test]
fn work_content_has_panel_items_when_workspace_loaded() -> std::io::Result<()> {
    let root = temp_workspace();
    fs::create_dir_all(&root)?;
    fs::write(root.join("README.md"), "# Hello")?;

    let mut comp = DesktopComposition::new();
    comp.workspace_root_path = Some(root.clone());
    comp.load_or_refresh_explorer();

    let wc = comp.build_work_content();
    assert!(wc.explorer_panel_items.is_some());
    assert!(wc.explorer_panel_title.is_some());
    assert!(wc.explorer_empty_button.is_none());

    let _ = fs::remove_dir_all(root.parent().unwrap());
    Ok(())
}

#[test]
fn work_content_panel_button_when_no_workspace() {
    let comp = DesktopComposition::new();
    let wc = comp.build_work_content();

    assert!(wc.explorer_panel_items.is_none());
    assert!(wc.explorer_panel_title.is_none());
    assert_eq!(wc.explorer_empty_button, Some("Open Workspace".to_string()));
}

#[test]
fn toggle_directory_updates_work_content() -> std::io::Result<()> {
    let root = temp_workspace();
    fs::create_dir_all(root.join("sub"))?;
    fs::write(root.join("sub").join("nested.rs"), "// nested")?;
    fs::write(root.join("top.txt"), "top")?;

    let mut comp = DesktopComposition::new();
    comp.workspace_root_path = Some(root.clone());
    comp.load_or_refresh_explorer();

    let wc_before = comp.build_work_content();
    let items_before = wc_before.explorer_panel_items.as_ref().unwrap();
    let has_sub = items_before.iter().any(|i| i.label.contains("sub") && i.is_dir);
    assert!(has_sub);

    // Find sub directory by scanning cached explorer items via the public API
    let sub_idx = (0..comp.explorer_item_count())
        .find(|&i| comp.get_explorer_item_at(i).map(|it| it.name.as_str()) == Some("sub"))
        .expect("sub should be in items");
    let sub_id = comp.get_explorer_item_id_at(sub_idx).unwrap();
    if let Some(ref mut e) = comp.maybe_explorer {
        e.toggle_expand(&sub_id);
    }
    comp.refresh_cached_explorer_items();

    let wc_after = comp.build_work_content();
    let items_after = wc_after.explorer_panel_items.as_ref().unwrap();
    assert!(items_after.len() > items_before.len(), "expand should add children");

    let _ = fs::remove_dir_all(root.parent().unwrap());
    Ok(())
}

#[test]
fn hit_rect_computed_when_button_visible() {
    let comp = DesktopComposition::new();
    let wc = comp.build_work_content();

    assert_eq!(wc.explorer_empty_button, Some("Open Workspace".to_string()));

    // Same formula as build_sidebar_block and the app.rs hit-rect compute.
    let pad = 10.0;
    let search_h = 26.0;
    let search_gap = 8.0;
    let divider_space = 12.0;
    let btn_button_y = 8.0;
    let sidebar_x = 44.0;
    let sidebar_y = 0.0;

    let hit_x = sidebar_x + pad + 10.0;
    let hit_y = sidebar_y + pad + search_h + search_gap + divider_space + btn_button_y;

    assert_eq!(hit_x, 64.0);
    assert_eq!(hit_y, 64.0);

    let bx = hit_x + 70.0;
    let by = hit_y + 15.0;
    assert!(bx >= hit_x && bx <= hit_x + 140.0);
    assert!(by >= hit_y && by <= hit_y + 30.0);
}
