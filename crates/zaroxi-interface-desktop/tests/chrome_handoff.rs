use zaroxi_core_engine_scene::scene::{ShellChrome, Tab as SceneTab};
use zaroxi_core_engine_render::intent::ChromePrimitive;

#[test]
fn handoff_no_tabs_produces_empty_chrome_primitive() {
    let chrome = ShellChrome {
        chrome_label: None,
        tabs: vec![],
        active_tab_index: None,
        focus_slot: None,
        status_text: None,
        ai_indicator: None,
        content_preview: None,
    };

    let prim = ChromePrimitive::from(chrome);
    assert!(prim.tabs.is_empty());
    assert!(prim.chrome_label.is_none());
    assert!(prim.active_tab_index.is_none());
}

#[test]
fn handoff_one_tab_preserves_label_and_active() {
    let scene_tab = SceneTab {
        index: 1,
        id: "tab1".to_string(),
        label: "main".to_string(),
        active: true,
    };

    let chrome = ShellChrome {
        chrome_label: Some("Project".to_string()),
        tabs: vec![scene_tab],
        active_tab_index: Some(0),
        focus_slot: Some("editor".to_string()),
        status_text: Some("ok".to_string()),
        ai_indicator: None,
        content_preview: None,
    };

    let prim = ChromePrimitive::from(chrome);
    assert_eq!(prim.chrome_label.as_deref(), Some("Project"));
    assert_eq!(prim.active_tab_index, Some(0));
    assert_eq!(prim.tabs.len(), 1);
    let t = &prim.tabs[0];
    assert_eq!(t.index, 1);
    assert_eq!(t.id, "tab1");
    assert_eq!(t.label, "main");
    assert!(t.active);
}

#[test]
fn handoff_multiple_tabs_preserves_order_and_active() {
    let tabs = vec![
        SceneTab { index: 1, id: "a".into(), label: "A".into(), active: false },
        SceneTab { index: 2, id: "b".into(), label: "B".into(), active: true },
        SceneTab { index: 3, id: "c".into(), label: "C".into(), active: false },
    ];

    let chrome = ShellChrome {
        chrome_label: None,
        tabs: tabs.clone(),
        active_tab_index: Some(1),
        focus_slot: None,
        status_text: None,
        ai_indicator: None,
        content_preview: None,
    };

    let prim = ChromePrimitive::from(chrome);
    assert_eq!(prim.tabs.len(), 3);
    // order preserved
    assert_eq!(prim.tabs[0].id, "a");
    assert_eq!(prim.tabs[1].id, "b");
    assert_eq!(prim.tabs[2].id, "c");
    // active semantics preserved
    assert_eq!(prim.tabs[1].active, true);
    assert_eq!(prim.active_tab_index, Some(1));
}
