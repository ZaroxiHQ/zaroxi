use zaroxi_core_engine_view::{EngineShellViewInput, EngineSelection};
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
        cursor_column: Some(5),
        selection: Some(EngineSelection {
            start_line: 1,
            start_column: 0,
            end_line: 2,
            end_column: 3,
        }),
        viewport_summary: Some("1-3/3".to_string()),
        status_text: Some("OK".to_string()),
        shell_chrome: Some("Shell".to_string()),
        last_command: Some("last".to_string()),
        ai_present: true,
    };

    let scene: ShellSceneModel = input.into();

    assert_eq!(
        scene.text_lines,
        vec!["one".to_string(), "two".to_string(), "three".to_string()]
    );
    assert_eq!(scene.viewport_top_line, 1);
    assert_eq!(scene.viewport_total_lines, 3);
    assert_eq!(scene.viewport_summary, Some("1-3/3".to_string()));
    assert_eq!(scene.cursor_line, Some(2));
    assert_eq!(scene.cursor_column, Some(5));
    assert!(scene.selection_present);
    // Phase-50 maps semantic text blocks directly where present.
    assert_eq!(scene.status_text, Some("OK".to_string()));
    assert_eq!(scene.chrome_text, Some("Shell".to_string()));
    assert_eq!(scene.last_command, Some("last".to_string()));
    assert!(scene.ai_status_present);
}
