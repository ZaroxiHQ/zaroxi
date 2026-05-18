use zaroxi_core_engine_layout::{
    ShellLayoutInput, LayoutBlock, TextBlock, SelectionBlock, StatusBlock, ViewportFacts,
};
use zaroxi_core_engine_render::{ShellRenderIntent, RenderSection};

#[test]
fn intent_from_layout_preserves_order_and_presence() {
    // Prepare a populated ShellLayoutInput with Text, Selection, Status in that order.
    let text_block = TextBlock {
        lines: vec!["line1".to_string(), "line2".to_string()],
    };
    let selection_block = SelectionBlock { line: 3, column: 5 };
    let status_block = StatusBlock {
        summary: "OK".to_string(),
    };

    let layout = ShellLayoutInput {
        blocks: vec![
            LayoutBlock::Text(text_block.clone()),
            LayoutBlock::Selection(selection_block.clone()),
            LayoutBlock::Status(status_block.clone()),
        ],
        viewport: ViewportFacts {
            top_line: 1,
            total_lines: 10,
            summary: Some("summary".to_string()),
            cursor_line: Some(3),
            cursor_column: Some(5),
        },
    };

    // Convert into the semantic render intent.
    let intent = ShellRenderIntent::from(layout);

    // Presence flags preserved
    assert!(intent.selection_present, "selection_present must be true");
    assert!(intent.status_present, "status_present must be true");

    // Ordering preserved and section contents copied
    assert_eq!(intent.sections.len(), 3);

    match &intent.sections[0] {
        RenderSection::Text { lines } => {
            assert_eq!(lines, &vec!["line1".to_string(), "line2".to_string()]);
        }
        _ => panic!("expected first section to be Text"),
    }

    match &intent.sections[1] {
        RenderSection::Selection { line, column } => {
            assert_eq!(*line, 3);
            assert_eq!(*column, 5);
        }
        _ => panic!("expected second section to be Selection"),
    }

    match &intent.sections[2] {
        RenderSection::Status { summary } => {
            assert_eq!(summary, "OK");
        }
        _ => panic!("expected third section to be Status"),
    }
}
