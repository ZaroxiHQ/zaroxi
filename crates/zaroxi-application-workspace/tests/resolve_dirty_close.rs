use std::fs;
use std::path::PathBuf;
use std::env::temp_dir;
use zaroxi_application_workspace::editor_service::{EditorService, ResolveDirtyCloseResult, CloseResult};

fn unique_path(name: &str) -> PathBuf {
    let mut p = temp_dir();
    let ts = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_millis();
    p.push(format!("zaroxi_test_{}_{}.txt", name, ts));
    p
}

#[test]
fn dirty_close_blocked_and_resolve_save_closes() -> std::io::Result<()> {
    let path = unique_path("resolve_save_close");
    fs::write(&path, "line1\nline2")?;
    let svc = EditorService::new_from_file(&path)?;
    // make dirty
    svc.type_text("X");
    assert!(svc.snapshot().dirty);
    // attempt to close should be blocked
    let res = svc.close_active();
    assert_eq!(res, CloseResult::BlockedByDirty);
    // resolve by saving
    let r = svc.resolve_dirty_close_save(&path);
    match r {
        ResolveDirtyCloseResult::ClosedAfterSave => {}
        other => panic!("expected ClosedAfterSave, got {:?}", other),
    }
    // buffer should be closed
    assert_eq!(svc.opened_paths().len(), 0);
    // disk should contain the saved change
    let disk = fs::read_to_string(&path)?;
    assert!(disk.contains("X"));
    let _ = fs::remove_file(&path);
    Ok(())
}

#[test]
fn dirty_close_blocked_and_resolve_discard_closes_and_discards() -> std::io::Result<()> {
    let path = unique_path("resolve_discard_close");
    fs::write(&path, "orig\ncontent")?;
    let svc = EditorService::new_from_file(&path)?;
    // make dirty
    svc.type_text("Y");
    assert!(svc.snapshot().dirty);
    // attempt close blocked
    let res = svc.close_active();
    assert_eq!(res, CloseResult::BlockedByDirty);
    // resolve by discarding
    let r = svc.resolve_dirty_close_discard(&path);
    match r {
        ResolveDirtyCloseResult::ClosedAfterDiscard => {}
        other => panic!("expected ClosedAfterDiscard, got {:?}", other),
    }
    // buffer closed
    assert_eq!(svc.opened_paths().len(), 0);
    // disk should be unchanged (discard prevented save)
    let disk = fs::read_to_string(&path)?;
    assert_eq!(disk, "orig\ncontent");
    let _ = fs::remove_file(&path);
    Ok(())
}

#[test]
fn discard_drops_unsaved_edits_on_reopen() -> std::io::Result<()> {
    let path = unique_path("discard_drops_edits");
    fs::write(&path, "first\nsecond")?;
    let svc = EditorService::new_from_file(&path)?;
    svc.type_text("Z");
    assert!(svc.snapshot().dirty);
    let r = svc.resolve_dirty_close_discard(&path);
    match r {
        ResolveDirtyCloseResult::ClosedAfterDiscard => {}
        other => panic!("expected ClosedAfterDiscard, got {:?}", other),
    }
    // reopen from disk to ensure edits were discarded
    let svc2 = EditorService::new_from_file(&path)?;
    assert_eq!(svc2.get_text(), fs::read_to_string(&path)?);
    let _ = fs::remove_file(&path);
    Ok(())
}

#[test]
fn save_failure_leaves_buffer_open_and_reports_failure() -> std::io::Result<()> {
    use std::os::unix::fs::PermissionsExt;

    let path = unique_path("save_failure");
    fs::write(&path, "alpha\nbeta")?;
    // make file read-only to force write failure on save
    let mut perms = fs::metadata(&path)?.permissions();
    perms.set_mode(0o444);
    fs::set_permissions(&path, perms)?;

    let svc = EditorService::new_from_file(&path)?;
    svc.type_text("Q");
    assert!(svc.snapshot().dirty);
    let r = svc.resolve_dirty_close_save(&path);
    match r {
        ResolveDirtyCloseResult::SaveFailed(_) => {}
        other => panic!("expected SaveFailed, got {:?}", other),
    }
    // buffer should still be open and dirty
    assert_eq!(svc.opened_paths().len(), 1);
    assert!(svc.snapshot().dirty);

    // restore permissions so cleanup can remove file
    let mut perms = fs::metadata(&path)?.permissions();
    perms.set_mode(0o644);
    fs::set_permissions(&path, perms)?;
    let _ = fs::remove_file(&path);
    Ok(())
}

#[test]
fn resolve_discard_on_non_active_buffer_updates_state_deterministically() -> std::io::Result<()> {
    let p1 = unique_path("r1");
    let p2 = unique_path("r2");
    let p3 = unique_path("r3");
    fs::write(&p1, "A")?;
    fs::write(&p2, "B")?;
    fs::write(&p3, "C")?;
    let svc = EditorService::new_from_file(&p1)?;
    svc.open_file(&p2)?;
    svc.open_file(&p3)?;
    // active is p3 (index 2)
    assert_eq!(svc.active_index(), Some(2));
    // make middle buffer dirty (p2)
    {
        // open p2 to make edits then return to p3 active to mimic non-active dirty buffer
        svc.open_file(&p2)?; // open_file activates p2
        svc.type_text("M");
        assert!(svc.snapshot().dirty);
        // reopen p3 to make it active again
        svc.open_file(&p3)?;
    }
    // now try to close p2 (non-active)
    let res = svc.close_buffer(&p2);
    assert_eq!(res, CloseResult::BlockedByDirty);
    // resolve by discarding p2
    let r = svc.resolve_dirty_close_discard(&p2);
    match r {
        ResolveDirtyCloseResult::ClosedAfterDiscard => {}
        other => panic!("expected ClosedAfterDiscard, got {:?}", other),
    }
    // opened should be two and active index should be adjusted deterministically
    assert_eq!(svc.opened_paths().len(), 2);
    assert_eq!(svc.active_index(), Some(1));
    let opened = svc.opened_paths();
    assert_eq!(opened[0].as_ref().unwrap(), &p1);
    assert_eq!(opened[1].as_ref().unwrap(), &p3);
    let _ = fs::remove_file(&p1);
    let _ = fs::remove_file(&p2);
    let _ = fs::remove_file(&p3);
    Ok(())
}
