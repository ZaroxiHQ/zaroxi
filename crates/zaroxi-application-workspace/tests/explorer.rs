use std::collections::HashSet;
use std::env;
use std::fs;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use zaroxi_application_workspace::WorkspaceExplorer;

#[test]
fn explorer_load_expand_select_open() -> std::io::Result<()> {
    // Create temporary workspace layout under OS temp dir:
    // tmp/<unique>/workspace/
    //   dir_a/
    //     file_a.txt
    //   file_root.txt
    let base = env::temp_dir();
    let uniq = format!(
        "zaroxi_test_{}_{}",
        std::process::id(),
        SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos()
    );
    let tmp = base.join(uniq);
    let root = tmp.join("workspace");
    fs::create_dir_all(root.join("dir_a"))?;
    fs::write(root.join("dir_a").join("file_a.txt"), "hello-a")?;
    fs::write(root.join("file_root.txt"), "hello-root")?;

    // Use the application-side explorer surface.
    let mut explorer = WorkspaceExplorer::new();
    explorer.load_workspace(&PathBuf::from(&root))?;

    // Root should be present and contain at least the two entries we created.
    assert!(explorer.tree.is_some());
    let tree = explorer.tree.as_ref().unwrap();
    // children may be in arbitrary order; assert presence by name.
    let mut seen_dir = false;
    let mut seen_file = false;
    for c in &tree.root.children {
        if c.name == "dir_a" {
            seen_dir = true;
        }
        if c.name == "file_root.txt" {
            seen_file = true;
        }
    }
    assert!(seen_dir, "expected dir_a present");
    assert!(seen_file, "expected file_root.txt present");

    // Expand dir_a
    let dir_id = tree
        .root
        .children
        .iter()
        .find(|c| c.name == "dir_a")
        .map(|c| c.id.clone())
        .expect("dir_a should exist");
    let toggled = explorer.toggle_expand(&dir_id);
    assert!(toggled, "toggle_expand should succeed for directories");

    // Selecting file inside dir should be possible.
    // After toggling we can find the child entry id.
    let file_id_opt = {
        let tree_ref = explorer.tree.as_ref().unwrap();
        let dir_node = tree_ref.root.children.iter().find(|c| c.name == "dir_a").unwrap();
        let file_child = dir_node
            .children
            .iter()
            .find(|c| c.name == "file_a.txt")
            .expect("file_a.txt should exist");
        file_child.id.clone()
    };

    let selected_ok = explorer.select(&file_id_opt);
    assert!(selected_ok, "selection should succeed");

    // Open selected file and verify contents.
    let opened = explorer.open_selected()?;
    assert_eq!(opened, Some("hello-a".to_string()));

    // Cleanup the temporary directory we created. Ignore errors during cleanup
    // to avoid hiding test failures.
    let _ = fs::remove_dir_all(&tmp);

    Ok(())
}

#[test]
fn explorer_visible_items_shows_top_level_without_root() -> std::io::Result<()> {
    let base = env::temp_dir();
    let uniq = format!(
        "zaroxi_vis_{}_{}",
        std::process::id(),
        SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos()
    );
    let tmp = base.join(uniq);
    let root = tmp.join("workspace");
    fs::create_dir_all(root.join("sub"))?;
    fs::write(root.join("top.txt"), "top")?;
    fs::write(root.join("sub").join("nest.txt"), "nest")?;

    let mut explorer = WorkspaceExplorer::new();
    explorer.load_workspace(&PathBuf::from(&root))?;

    let items = explorer.visible_items(&HashSet::new(), None);
    // Should see top.txt and sub/ at depth 0 (root itself is excluded).
    let files: Vec<_> = items.iter().map(|i| (&i.name, i.depth, i.is_dir)).collect();
    assert!(files.iter().any(|(n, d, _)| *n == "top.txt" && *d == 0));
    assert!(files.iter().any(|(n, d, d2)| *n == "sub" && *d == 0 && *d2));

    let _ = fs::remove_dir_all(&tmp);
    Ok(())
}

#[test]
fn explorer_toggle_expand_reveals_children() -> std::io::Result<()> {
    let base = env::temp_dir();
    let uniq = format!(
        "zaroxi_exp_{}_{}",
        std::process::id(),
        SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos()
    );
    let tmp = base.join(uniq);
    let root = tmp.join("workspace");
    fs::create_dir_all(root.join("sub"))?;
    fs::write(root.join("sub").join("nest.txt"), "nest")?;

    let mut explorer = WorkspaceExplorer::new();
    explorer.load_workspace(&PathBuf::from(&root))?;

    let items_before = explorer.visible_items(&HashSet::new(), None);
    assert!(items_before.iter().any(|i| i.name == "sub"));
    assert!(!items_before.iter().any(|i| i.name == "nest.txt"));

    let sub_id = items_before.iter().find(|i| i.name == "sub").map(|i| i.id.clone()).unwrap();
    let toggled = explorer.toggle_expand(&sub_id);
    assert!(toggled);

    let items_after = explorer.visible_items(&HashSet::new(), None);
    assert!(items_after.iter().any(|i| i.name == "nest.txt" && i.depth == 1));

    let _ = fs::remove_dir_all(&tmp);
    Ok(())
}

#[test]
fn explorer_visible_items_marks_open_and_active() -> std::io::Result<()> {
    let base = env::temp_dir();
    let uniq = format!(
        "zaroxi_act_{}_{}",
        std::process::id(),
        SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos()
    );
    let tmp = base.join(uniq);
    let root = tmp.join("workspace");
    fs::create_dir_all(&root)?;
    fs::write(root.join("target.txt"), "target")?;

    let mut explorer = WorkspaceExplorer::new();
    explorer.load_workspace(&PathBuf::from(&root))?;

    let target_path = root.join("target.txt");
    let target_str = target_path.to_string_lossy().to_string();

    let mut opened = HashSet::new();
    opened.insert(target_str.clone());

    let items = explorer.visible_items(&opened, Some(&target_str));
    let target_item = items.iter().find(|i| i.name == "target.txt").unwrap();
    assert!(target_item.is_open);
    assert!(target_item.is_active);

    let _ = fs::remove_dir_all(&tmp);
    Ok(())
}

#[test]
fn explorer_is_dir_and_get_entry_path() -> std::io::Result<()> {
    let base = env::temp_dir();
    let uniq = format!(
        "zaroxi_dir_{}_{}",
        std::process::id(),
        SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos()
    );
    let tmp = base.join(uniq);
    let root = tmp.join("workspace");
    fs::create_dir_all(root.join("mydir"))?;
    fs::write(root.join("myfile.txt"), "hello")?;

    let explorer = {
        let mut e = WorkspaceExplorer::new();
        e.load_workspace(&PathBuf::from(&root))?;
        e
    };

    let items = explorer.visible_items(&HashSet::new(), None);
    let dir_item = items.iter().find(|i| i.name == "mydir").unwrap();
    let file_item = items.iter().find(|i| i.name == "myfile.txt").unwrap();

    assert!(explorer.is_dir(&dir_item.id));
    assert!(!explorer.is_dir(&file_item.id));

    let file_path = explorer.get_entry_path(&file_item.id).unwrap();
    assert!(file_path.ends_with("myfile.txt"));

    let _ = fs::remove_dir_all(&tmp);
    Ok(())
}
