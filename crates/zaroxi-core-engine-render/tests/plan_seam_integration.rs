use zaroxi_core_engine_render::{ShellRenderIntent, RenderSection, ShellDrawPlan, DrawSection};

#[test]
fn intent_to_draw_plan_consumes_text_seam() {
    // Build a minimal ShellRenderIntent containing a Text section. Converting
    // this intent into a ShellDrawPlan should exercise the engine text seam
    // (dummy backend path) as part of the real render-path conversion.
    let intent = ShellRenderIntent {
        sections: vec![RenderSection::Text { lines: vec!["hello".to_string()] }],
        selection_present: false,
        status_present: false,
    };

    let plan = ShellDrawPlan::from(intent);

    assert!(plan.content_present);
    assert_eq!(
        plan.sections,
        vec![DrawSection::Content {
            line_count: 1,
            width: 5u32.saturating_mul(8),
            height: 16
        }]
    );
}
