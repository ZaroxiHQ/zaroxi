use std::env::temp_dir;
use std::fs;
use std::path::PathBuf;
use zaroxi_application_workspace::editor_service::{CloseResult, EditorService};

// Process-wide counter + pid guarantee unique temp paths across parallel test
// binaries/threads even on hosts with coarse clock resolution (Windows CI VMs),
// where millisecond timestamps alone can collide and let tests clobber each
// other's files.
static UNIQUE_COUNTER: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);

fn unique_path(name: &str) -> PathBuf {
    let mut p = temp_dir();
    let ts = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos();
    let n = UNIQUE_COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    p.push(format!("zaroxi_test_{}_{}_{}_{}.txt", name, std::process::id(), ts, n));
    p
}

#[test]
fn closing_clean_buffer_removes_it_and_updates_active() -> std::io::Result<()> {
    let p1 = unique_path("close_clean_1");
    let p2 = unique_path("close_clean_2");
    fs::write(&p1, "first\nline")?;
    fs::write(&p2, "second\nline")?;
    let svc = EditorService::new_from_file(&p1)?;
    // open second file (will become active by design)
    svc.open_file(&p2)?;
    let opened = svc.opened_paths();
    assert_eq!(opened.len(), 2);
    // active should be the second (index 1)
    assert_eq!(svc.active_index(), Some(1));
    // close the active (clean) buffer
    let res = svc.close_buffer(&p2);
    assert_eq!(res, CloseResult::Closed);
    let opened2 = svc.opened_paths();
    assert_eq!(opened2.len(), 1);
    // active should now be the previous neighbor (index 0)
    assert_eq!(svc.active_index(), Some(0));
    // text should reflect first file
    assert_eq!(svc.get_text(), fs::read_to_string(&p1)?);
    let _ = fs::remove_file(&p1);
    let _ = fs::remove_file(&p2);
    Ok(())
}

#[test]
fn closing_last_buffer_leaves_no_active_buffer() -> std::io::Result<()> {
    let p = unique_path("close_last");
    fs::write(&p, "only\none")?;
    let svc = EditorService::new_from_file(&p)?;
    // close active (only) buffer
    let res = svc.close_active();
    assert_eq!(res, CloseResult::Closed);
    // no opened buffers and no active index
    assert_eq!(svc.opened_paths().len(), 0);
    assert_eq!(svc.active_index(), None);
    // snapshot is empty
    let snap = svc.snapshot();
    assert!(snap.lines.is_empty());
    let _ = fs::remove_file(&p);
    Ok(())
}

#[test]
fn closing_dirty_buffer_is_blocked_until_saved() -> std::io::Result<()> {
    let p = unique_path("close_dirty");
    fs::write(&p, "dirty\nstart")?;
    let svc = EditorService::new_from_file(&p)?;
    // make dirty
    svc.type_text("X");
    assert!(svc.snapshot().dirty);
    // attempt close should be blocked
    let res = svc.close_active();
    assert_eq!(res, CloseResult::BlockedByDirty);
    // save then close should succeed
    svc.save(&p)?;
    assert!(!svc.snapshot().dirty);
    let res2 = svc.close_active();
    assert_eq!(res2, CloseResult::Closed);
    let _ = fs::remove_file(&p);
    Ok(())
}

#[test]
fn opened_buffer_summaries_coherent_after_close() -> std::io::Result<()> {
    let p1 = unique_path("sum_1");
    let p2 = unique_path("sum_2");
    fs::write(&p1, "one")?;
    fs::write(&p2, "two")?;
    let svc = EditorService::new_from_file(&p1)?;
    svc.open_file(&p2)?;
    // close first (non-active)
    let res = svc.close_buffer(&p1);
    assert_eq!(res, CloseResult::Closed);
    // opened paths length should be 1 and active should point to remaining
    let opened = svc.opened_paths();
    assert_eq!(opened.len(), 1);
    assert_eq!(svc.active_index(), Some(0));
    // remaining path should be p2
    assert_eq!(opened[0].as_ref().unwrap(), &p2);
    let _ = fs::remove_file(&p1);
    let _ = fs::remove_file(&p2);
    Ok(())
}

#[test]
fn integration_flow_open_multiple_close_one_updates_state() -> std::io::Result<()> {
    // open three files, close middle, ensure neighbor selection deterministic
    let p1 = unique_path("int_1");
    let p2 = unique_path("int_2");
    let p3 = unique_path("int_3");
    fs::write(&p1, "A")?;
    fs::write(&p2, "B")?;
    fs::write(&p3, "C")?;
    let svc = EditorService::new_from_file(&p1)?;
    svc.open_file(&p2)?;
    svc.open_file(&p3)?;
    // currently active should be p3 (last opened)
    assert_eq!(svc.opened_paths().len(), 3);
    assert_eq!(svc.active_index(), Some(2));
    // close middle (p2)
    let res = svc.close_buffer(&p2);
    assert_eq!(res, CloseResult::Closed);
    // opened count should be 2
    assert_eq!(svc.opened_paths().len(), 2);
    // active index should have adjusted (since active was index 2 and we removed index1, active becomes 1)
    assert_eq!(svc.active_index(), Some(1));
    // opened paths should be [p1, p3]
    let opened = svc.opened_paths();
    assert_eq!(opened[0].as_ref().unwrap(), &p1);
    assert_eq!(opened[1].as_ref().unwrap(), &p3);
    let _ = fs::remove_file(&p1);
    let _ = fs::remove_file(&p2);
    let _ = fs::remove_file(&p3);
    Ok(())
}
