use zaroxi_core_engine_render::intent::RenderSection;
use zaroxi_core_engine_scene::scene::{ShellChrome, Tab as SceneTab};

#[test]
fn chrome_to_render_section_no_tabs_is_safe() {
    let chrome = ShellChrome {
        chrome_label: None,
        tabs: vec![],
        active_tab_index: None,
        active_panel_id: None,
        status_text: None,
    };

    let sec = RenderSection::from(chrome);
    match sec {
        RenderSection::Chrome { chrome } => {
            assert!(chrome.tabs.is_empty());
            assert!(chrome.chrome_label.is_none());
            assert!(chrome.active_tab_index.is_none());
        }
        _ => panic!("expected chrome render section"),
    }
}

#[test]
fn chrome_to_render_section_one_tab_preserves_label_and_active() {
    let scene_tab =
        SceneTab { index: 1, id: "tab1".to_string(), label: "main".to_string(), active: true };

    let chrome = ShellChrome {
        chrome_label: Some("Project".to_string()),
        tabs: vec![scene_tab],
        active_tab_index: Some(0),
        active_panel_id: Some("editor".to_string()),
        status_text: Some("ok".to_string()),
    };

    let sec = RenderSection::from(chrome);
    match sec {
        RenderSection::Chrome { chrome } => {
            assert_eq!(chrome.chrome_label.as_deref(), Some("Project"));
            assert_eq!(chrome.active_tab_index, Some(0));
            assert_eq!(chrome.tabs.len(), 1);
            let t = &chrome.tabs[0];
            assert_eq!(t.index, 1);
            assert_eq!(t.id, "tab1");
            assert_eq!(t.label, "main");
            assert!(t.active);
        }
        _ => panic!("expected chrome render section"),
    }
}

#[test]
fn chrome_to_render_section_multiple_tabs_preserves_order_and_active() {
    let tabs = vec![
        SceneTab { index: 1, id: "a".into(), label: "A".into(), active: false },
        SceneTab { index: 2, id: "b".into(), label: "B".into(), active: true },
        SceneTab { index: 3, id: "c".into(), label: "C".into(), active: false },
    ];

    let chrome = ShellChrome {
        chrome_label: None,
        tabs: tabs.clone(),
        active_tab_index: Some(1),
        active_panel_id: None,
        status_text: None,
    };

    let sec = RenderSection::from(chrome);
    match sec {
        RenderSection::Chrome { chrome } => {
            assert_eq!(chrome.tabs.len(), 3);
            assert_eq!(chrome.tabs[0].id, "a");
            assert_eq!(chrome.tabs[1].id, "b");
            assert_eq!(chrome.tabs[2].id, "c");
            assert_eq!(chrome.tabs[1].active, true);
            assert_eq!(chrome.active_tab_index, Some(1));
        }
        _ => panic!("expected chrome render section"),
    }
}
