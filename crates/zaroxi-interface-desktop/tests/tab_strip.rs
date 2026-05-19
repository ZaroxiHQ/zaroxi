use zaroxi_interface_desktop::presenters::gpu_shell::TabStrip;

#[test]
fn no_buffers_empty_tab_strip() {
    let opened: Vec<(String, String)> = vec![];
    let ts = TabStrip::from_opened_and_active(&opened, None);
    assert!(ts.tabs.is_empty());
}

#[test]
fn one_buffer_active() {
    let opened = vec![("buf1".to_string(), "main.rs".to_string())];
    let ts = TabStrip::from_opened_and_active(&opened, Some("buf1"));
    assert_eq!(ts.tabs.len(), 1);
    let t = &ts.tabs[0];
    assert_eq!(t.id, "buf1");
    assert_eq!(t.display, "main.rs");
    assert!(t.active);
    assert_eq!(t.index, 0);
}

#[test]
fn multiple_buffers_order_preserved_and_single_active() {
    let opened = vec![
        ("a".to_string(), "A".to_string()),
        ("b".to_string(), "B".to_string()),
        ("c".to_string(), "C".to_string()),
    ];
    let ts = TabStrip::from_opened_and_active(&opened, Some("b"));
    assert_eq!(ts.tabs.len(), 3);
    assert_eq!(ts.tabs[0].id, "a");
    assert_eq!(ts.tabs[1].id, "b");
    assert_eq!(ts.tabs[2].id, "c");
    let active_count = ts.tabs.iter().filter(|t| t.active).count();
    assert_eq!(active_count, 1);
    assert_eq!(ts.tabs[1].display, "B");
}

#[test]
fn active_none_when_no_match() {
    let opened = vec![
        ("x".to_string(), "X".to_string()),
        ("y".to_string(), "Y".to_string()),
    ];
    let ts = TabStrip::from_opened_and_active(&opened, Some("missing"));
    assert!(ts.tabs.iter().all(|t| !t.active));
}
