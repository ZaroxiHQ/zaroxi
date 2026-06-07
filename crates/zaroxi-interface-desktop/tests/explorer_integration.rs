use std::env;
use std::fs;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};
use zaroxi_interface_desktop::DesktopComposition;

fn temp_workspace() -> PathBuf {
    let base = env::temp_dir();
    let uniq = format!(
        "zaroxi_test_{}_{}",
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    );
    let tmp = base.join(uniq);
    let root = tmp.join("workspace");
    root
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
    assert!(
        items.iter().any(|s| s.contains("readme.md")),
        "explorer should show readme.md"
    );

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
    assert!(
        items.is_none(),
        "empty workspace directory should produce no explorer items"
    );

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
