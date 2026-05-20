use zaroxi_core_engine_layout::{
    ShellLayoutInput, LayoutBlock, TextBlock, SelectionBlock, StatusBlock, ViewportFacts,
};
use zaroxi_core_engine_render::{ShellRenderIntent, plan::DrawSection, ShellDrawPlan};

#[test]
fn plan_from_intent_preserves_order_and_presence() {
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

    // Convert into the semantic render intent, then into the draw plan.
    let intent = ShellRenderIntent::from(layout);
    let plan = ShellDrawPlan::from(intent);

    // Presence flags preserved
    assert!(plan.selection_present, "selection_present must be true");
    assert!(plan.status_present, "status_present must be true");
    assert!(plan.content_present, "content_present must be true");

    // Ordering preserved and section kinds mapped
    assert_eq!(plan.sections.len(), 3);

    match &plan.sections[0] {
        DrawSection::Content { line_count: _, width: _, height: _ } => {}
        _ => panic!("expected first section to be Content"),
    }

    match &plan.sections[1] {
        DrawSection::Selection => {}
        _ => panic!("expected second section to be Selection"),
    }

    match &plan.sections[2] {
        DrawSection::Status => {}
        _ => panic!("expected third section to be Status"),
    }
}
