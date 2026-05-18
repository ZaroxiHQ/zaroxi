use zaroxi_core_engine_layout::{LayoutBlock, SelectionBlock, ShellLayoutInput, StatusBlock, TextBlock};
use zaroxi_core_engine_scene::ShellSceneModel;

#[test]
fn translation_preserves_structure_and_ordering() {
    // Construct a minimal ShellSceneModel (struct fields are authoritative per repo summaries).
    let scene = ShellSceneModel {
        text_lines: vec!["line1".into(), "line2".into()],
        viewport_top_line: 1,
        viewport_total_lines: 200,
        viewport_summary: Some("ok".into()),
        cursor_line: Some(2),
        cursor_column: Some(5),
        // Fields required by the ShellSceneModel struct:
        selection_present: true,
        status_text: Some("status".into()),
        chrome_text: Some("chrome".into()),
        last_command: Some("cmd".into()),
        ai_status_present: false,
    };

    let layout = ShellLayoutInput::from(scene);

    // Expect blocks: Text, Selection, Status (in that order)
    assert_eq!(layout.blocks.len(), 3);

    match &layout.blocks[0] {
        LayoutBlock::Text(tb) => {
            assert_eq!(*tb, TextBlock { lines: vec!["line1".into(), "line2".into()] });
        }
        other => panic!("expected Text block, got {:?}", other),
    }

    match &layout.blocks[1] {
        LayoutBlock::Selection(sel) => {
            assert_eq!(*sel, SelectionBlock { line: 2, column: 5 });
        }
        other => panic!("expected Selection block, got {:?}", other),
    }

    match &layout.blocks[2] {
        LayoutBlock::Status(s) => {
            assert_eq!(*s, StatusBlock { summary: "ok".into() });
        }
        other => panic!("expected Status block, got {:?}", other),
    }

    // viewport facts preserved
    assert_eq!(layout.viewport.top_line, 1);
    assert_eq!(layout.viewport.total_lines, 200);
    assert_eq!(layout.viewport.summary, Some("ok".into()));
    assert_eq!(layout.viewport.cursor_line, Some(2));
    assert_eq!(layout.viewport.cursor_column, Some(5));
}
