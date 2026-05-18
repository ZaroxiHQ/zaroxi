use zaroxi_core_engine_render::{ShellTextRenderer, ShellDrawPlan, ShellRenderTranscript};

#[test]
fn renders_default_shell_draw_plan_stably() {
    // Build a minimal ShellDrawPlan using Default (the plan type is part of
    // the existing conversion chain; Default is used here to keep the test
    // tiny and focused on renderer stability).
    let renderer = ShellTextRenderer::new();
    let plan = ShellDrawPlan::default();
    let transcript = renderer.render(&plan);

    // Expected content is deterministic: header + pretty Debug of the plan.
    let expected_lines = vec![
        "ShellDrawPlan:".to_string(),
        format!("{:#?}", plan),
    ];

    assert_eq!(transcript.lines, expected_lines, "Renderer produced unstable or unexpected output");
}
