use std::fs;
use std::path::PathBuf;
use std::env::temp_dir;
use zaroxi_application_workspace::editor_service::EditorService;

fn unique_path(name: &str) -> PathBuf {
    let mut p = temp_dir();
    let ts = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_millis();
    p.push(format!("zaroxi_test_{}_{}.txt", name, ts));
    p
}

#[test]
fn edit_marks_dirty_and_save_clears_dirty_and_writes_file() -> std::io::Result<()> {
    let path = unique_path("save_clears_dirty");
    fs::write(&path, "line1\nline2")?;
    let svc = EditorService::new_from_file(&path)?;
    let snap = svc.snapshot();
    assert_eq!(snap.dirty, false, "fresh from-file should be clean");
    svc.type_text("X");
    let snap2 = svc.snapshot();
    assert!(snap2.dirty, "after edit buffer should be dirty");
    svc.save(&path)?;
    let snap3 = svc.snapshot();
    assert_eq!(snap3.dirty, false, "after save buffer should be clean");
    let disk = fs::read_to_string(&path)?;
    assert_eq!(disk, svc.get_text());
    // cleanup
    let _ = fs::remove_file(&path);
    Ok(())
}

#[test]
fn reload_reflects_disk_content() -> std::io::Result<()> {
    let path = unique_path("reload_reflects_disk");
    fs::write(&path, "original\ncontent")?;
    let svc = EditorService::new_from_file(&path)?;
    svc.type_text("X");
    svc.save(&path)?;
    // external change
    fs::write(&path, "external\nchanged")?;
    svc.reload(&path)?;
    assert_eq!(svc.get_text(), fs::read_to_string(&path)?);
    let snap = svc.snapshot();
    assert_eq!(snap.dirty, false, "after reload buffer should be clean and reflect disk");
    let _ = fs::remove_file(&path);
    Ok(())
}

#[test]
fn undo_after_save_can_clear_dirty() -> std::io::Result<()> {
    let path = unique_path("undo_after_save");
    fs::write(&path, "a\nb")?;
    let svc = EditorService::new_from_file(&path)?;
    svc.type_text("Z");
    assert!(svc.snapshot().dirty);
    svc.save(&path)?;
    assert!(!svc.snapshot().dirty);
    svc.type_text("Y");
    assert!(svc.snapshot().dirty);
    svc.undo();
    // undo should restore to saved state and clear dirty
    assert!(!svc.snapshot().dirty);
    let _ = fs::remove_file(&path);
    Ok(())
}
