use std::fs;
use std::path::PathBuf;
use std::env::temp_dir;
use zaroxi_application_workspace::editor_service::{
    EditorService, AttemptCloseSessionResult, ResolveCloseSessionResult,
};

fn unique_path(name: &str) -> PathBuf {
    let mut p = temp_dir();
    let ts = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_millis();
    p.push(format!("zaroxi_test_{}_{}.txt", name, ts));
    p
}

#[test]
fn close_session_succeeds_when_all_clean() -> std::io::Result<()> {
    let p1 = unique_path("clean1");
    let p2 = unique_path("clean2");
    fs::write(&p1, "one\n")?;
    fs::write(&p2, "two\n")?;
    let svc = EditorService::new_from_file(&p1)?;
    svc.open_file(&p2)?;
    // nothing dirty initially
    let res = svc.attempt_close_session();
    assert_eq!(res, AttemptCloseSessionResult::Closed);
    assert_eq!(svc.opened_paths().len(), 0);
    assert_eq!(svc.active_index(), None);
    let snap = svc.snapshot();
    assert!(snap.lines.is_empty());
    let _ = fs::remove_file(&p1);
    let _ = fs::remove_file(&p2);
    Ok(())
}

#[test]
fn close_session_blocked_when_any_dirty() -> std::io::Result<()> {
    let p = unique_path("blocked");
    fs::write(&p, "alpha\nbeta")?;
    let svc = EditorService::new_from_file(&p)?;
    svc.type_text("X");
    let res = svc.attempt_close_session();
    match res {
        AttemptCloseSessionResult::BlockedByDirty { dirty_buffers } => {
            assert!(dirty_buffers.len() >= 1);
            // ensure original buffer still present
            assert_eq!(svc.opened_paths().len(), 1);
        }
        other => panic!("expected BlockedByDirty, got {:?}", other),
    }
    let _ = fs::remove_file(&p);
    Ok(())
}

#[test]
fn close_session_reports_multiple_dirty_buffers() -> std::io::Result<()> {
    let p1 = unique_path("m1");
    let p2 = unique_path("m2");
    fs::write(&p1, "A")?;
    fs::write(&p2, "B")?;
    let svc = EditorService::new_from_file(&p1)?;
    svc.open_file(&p2)?;
    // make both dirty
    svc.type_text("z");
    // switch to first and dirty it too
    svc.open_file(&p1)?;
    svc.type_text("y");
    // Now attempt close
    let res = svc.attempt_close_session();
    match res {
        AttemptCloseSessionResult::BlockedByDirty { dirty_buffers } => {
            assert!(dirty_buffers.len() >= 2);
            // ensure session still open
            assert_eq!(svc.opened_paths().len(), 2);
        }
        other => panic!("expected BlockedByDirty, got {:?}", other),
    }
    let _ = fs::remove_file(&p1);
    let _ = fs::remove_file(&p2);
    Ok(())
}

#[test]
fn resolve_save_all_succeeds_and_closes() -> std::io::Result<()> {
    let p1 = unique_path("save_all_1");
    let p2 = unique_path("save_all_2");
    fs::write(&p1, "orig1")?;
    fs::write(&p2, "orig2")?;
    let svc = EditorService::new_from_file(&p1)?;
    svc.open_file(&p2)?;
    // dirty both
    svc.type_text("X");
    svc.open_file(&p1)?;
    svc.type_text("Y");
    // save-all and close
    let r = svc.resolve_close_session_save_all();
    assert!(matches!(r, ResolveCloseSessionResult::ClosedAfterSaveAll));
    // buffers should be removed
    assert_eq!(svc.opened_paths().len(), 0);
    // disk should reflect saved changes
    let d1 = fs::read_to_string(&p1)?;
    let d2 = fs::read_to_string(&p2)?;
    assert!(d1.contains("Y") || d1.contains("X"));
    assert!(d2.contains("X") || d2.contains("Y"));
    let _ = fs::remove_file(&p1);
    let _ = fs::remove_file(&p2);
    Ok(())
}

#[test]
fn resolve_discard_all_succeeds_and_discards_edits() -> std::io::Result<()> {
    let p = unique_path("discard_all");
    fs::write(&p, "disk\ncontent")?;
    let svc = EditorService::new_from_file(&p)?;
    svc.type_text("Z");
    assert!(svc.snapshot().dirty);
    let r = svc.resolve_close_session_discard_all();
    assert!(matches!(r, ResolveCloseSessionResult::ClosedAfterDiscardAll));
    // reopen should reflect disk (edits discarded)
    let svc2 = EditorService::new_from_file(&p)?;
    assert_eq!(svc2.get_text(), fs::read_to_string(&p)?);
    let _ = fs::remove_file(&p);
    Ok(())
}

#[test]
fn save_all_failure_keeps_session_open_and_reports() -> std::io::Result<()> {
    use std::os::unix::fs::PermissionsExt;
    let p = unique_path("save_fail");
    fs::write(&p, "willfail")?;
    // make write fail by making file read-only (on unix)
    let mut perms = fs::metadata(&p)?.permissions();
    perms.set_mode(0o444);
    fs::set_permissions(&p, perms)?;
    let svc = EditorService::new_from_file(&p)?;
    svc.type_text("Q");
    let r = svc.resolve_close_session_save_all();
    match r {
        ResolveCloseSessionResult::SaveAllFailed { failed_buffers } => {
            assert!(!failed_buffers.is_empty());
            // session must still be open
            assert!(svc.opened_paths().len() >= 1);
        }
        other => panic!("expected SaveAllFailed, got {:?}", other),
    }
    // restore permissions for cleanup
    let mut perms = fs::metadata(&p)?.permissions();
    perms.set_mode(0o644);
    fs::set_permissions(&p, perms)?;
    let _ = fs::remove_file(&p);
    Ok(())
}
