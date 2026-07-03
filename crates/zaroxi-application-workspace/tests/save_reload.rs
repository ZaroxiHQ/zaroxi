use std::env::temp_dir;
use std::fs;
use std::path::PathBuf;
use zaroxi_application_workspace::editor_service::{EditorService, ReloadResult};

// Process-wide counter + pid guarantee unique temp paths across parallel test
// binaries/threads even on hosts with coarse clock resolution (Windows CI VMs),
// where millisecond timestamps alone can collide and let tests clobber each
// other's files.
static UNIQUE_COUNTER: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);

fn unique_path(name: &str) -> PathBuf {
    let mut p = temp_dir();
    let ts = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos();
    let n = UNIQUE_COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    p.push(format!("zaroxi_test_save_{}_{}_{}_{}.txt", name, std::process::id(), ts, n));
    p
}

#[test]
fn save_all_across_multiple_dirty_buffers() -> std::io::Result<()> {
    let p1 = unique_path("a");
    let p2 = unique_path("b");
    fs::write(&p1, "one\n")?;
    fs::write(&p2, "two\n")?;
    let svc = EditorService::new_from_file(&p1)?;
    svc.open_file(&p2)?;
    // active is p2; make p2 dirty
    svc.type_text("X");
    assert!(svc.snapshot().dirty);
    // activate p1 and make it dirty
    svc.open_file(&p1)?;
    svc.type_text("Y");
    assert!(svc.snapshot().dirty);
    // save all
    let saved = svc.save_all_buffers()?;
    assert_eq!(saved, 2);
    // disk should contain changes
    let d1 = fs::read_to_string(&p1)?;
    let d2 = fs::read_to_string(&p2)?;
    assert!(d1.contains("Y"));
    assert!(d2.contains("X"));
    // cleanup
    let _ = fs::remove_file(&p1);
    let _ = fs::remove_file(&p2);
    Ok(())
}

#[test]
fn reload_on_clean_buffer_updates() -> std::io::Result<()> {
    let p = unique_path("reload_clean");
    fs::write(&p, "orig\n")?;
    let svc = EditorService::new_from_file(&p)?;
    // externally modify disk
    fs::write(&p, "updated\n")?;
    let r = svc.reload_buffer(&p);
    match r {
        ReloadResult::Reloaded => {}
        other => panic!("expected Reloaded, got {:?}", other),
    }
    assert_eq!(svc.get_text(), fs::read_to_string(&p)?);
    let _ = fs::remove_file(&p);
    Ok(())
}

#[test]
fn reload_on_dirty_buffer_blocked() -> std::io::Result<()> {
    let p = unique_path("reload_dirty");
    fs::write(&p, "base\n")?;
    let svc = EditorService::new_from_file(&p)?;
    svc.type_text("M");
    assert!(svc.snapshot().dirty);
    // externally change disk
    fs::write(&p, "on_disk\n")?;
    let r = svc.reload_buffer(&p);
    match r {
        ReloadResult::BlockedByDirty => {}
        other => panic!("expected BlockedByDirty, got {:?}", other),
    }
    // buffer should still contain unsaved edits (not replaced)
    assert!(svc.snapshot().dirty);
    let _ = fs::remove_file(&p);
    Ok(())
}

#[test]
fn resolve_reload_discard_updates_buffer() -> std::io::Result<()> {
    let p = unique_path("resolve_reload_discard");
    fs::write(&p, "orig\n")?;
    let svc = EditorService::new_from_file(&p)?;
    svc.type_text("Z");
    assert!(svc.snapshot().dirty);
    // externally change disk
    fs::write(&p, "disk_changed\n")?;
    let r = svc.resolve_reload_discard(&p);
    match r {
        ReloadResult::Reloaded => {}
        other => panic!("expected Reloaded, got {:?}", other),
    }
    assert!(!svc.snapshot().dirty);
    assert_eq!(svc.get_text(), fs::read_to_string(&p)?);
    let _ = fs::remove_file(&p);
    Ok(())
}
