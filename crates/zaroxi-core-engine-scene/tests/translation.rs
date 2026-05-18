use zaroxi_core_engine_view::EngineShellViewInput;
use zaroxi_core_engine_scene::ShellSceneModel;

/// Semantic translation test:
/// Ensure a populated EngineShellViewInput produces a ShellSceneModel that
/// preserves the textual and viewport semantics (no layout info).
#[test]
fn translation_preserves_semantics() {
    let input = EngineShellViewInput {
        top_line: 1,
        total_lines: 3,
        lines: vec!["one".to_string(), "two".to_string(), "three".to_string()],
        cursor_line: Some(2),
    };

    let scene: ShellSceneModel = input.into();

    assert_eq!(
        scene.text_lines,
        vec!["one".to_string(), "two".to_string(), "three".to_string()]
    );
    assert_eq!(scene.viewport_top_line, 1);
    assert_eq!(scene.viewport_total_lines, 3);
    assert_eq!(scene.cursor_line, Some(2));
    assert!(scene.selection_present);
    // Phase-50 defaults are false for presence flags we don't yet compute.
    assert!(!scene.status_present);
    assert!(!scene.chrome_present);
    assert!(!scene.ai_status_present);
}
