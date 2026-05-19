use zaroxi_interface_desktop::presenters::gpu_shell::{KeyEvent, handle_key_event};

#[test]
fn keymaps_ctrl_tab_to_next_and_updates_active() {
    let opened = vec![
        ("a".to_string(), "A".to_string()),
        ("b".to_string(), "B".to_string()),
    ];

    let mut active: Option<String> = Some("a".to_string());
    let current_active = active.clone();

    let res = handle_key_event(
        &KeyEvent { ctrl: true, shift: false, key: "Tab".to_string() },
        &opened,
        current_active.as_deref(),
        |id: &str| {
            active = Some(id.to_string());
        },
    );

    assert_eq!(res.as_deref(), Some("b"));
    assert_eq!(active.as_deref(), Some("b"));
}

#[test]
fn keymaps_ctrl_shift_tab_to_prev_and_updates_active() {
    let opened = vec![
        ("a".to_string(), "A".to_string()),
        ("b".to_string(), "B".to_string()),
        ("c".to_string(), "C".to_string()),
    ];

    let mut active: Option<String> = Some("b".to_string());
    let current_active = active.clone();

    let res = handle_key_event(
        &KeyEvent { ctrl: true, shift: true, key: "Tab".to_string() },
        &opened,
        current_active.as_deref(),
        |id: &str| {
            active = Some(id.to_string());
        },
    );

    assert_eq!(res.as_deref(), Some("a"));
    assert_eq!(active.as_deref(), Some("a"));
}

#[test]
fn keymaps_are_noop_on_empty_buffers() {
    let opened: Vec<(String, String)> = vec![];

    let mut applied = false;
    let res = handle_key_event(
        &KeyEvent { ctrl: true, shift: false, key: "Tab".to_string() },
        &opened,
        None,
        |_id| {
            applied = true;
        },
    );

    assert_eq!(res, None);
    assert!(!applied);
}

#[test]
fn keymaps_single_buffer_stays_same() {
    let opened = vec![("solo".to_string(), "solo.rs".to_string())];

    let mut active: Option<String> = Some("solo".to_string());
    let current_active = active.clone();

    let res = handle_key_event(
        &KeyEvent { ctrl: true, shift: false, key: "Tab".to_string() },
        &opened,
        current_active.as_deref(),
        |id: &str| {
            active = Some(id.to_string());
        },
    );

    assert_eq!(res.as_deref(), Some("solo"));
    assert_eq!(active.as_deref(), Some("solo"));
}
