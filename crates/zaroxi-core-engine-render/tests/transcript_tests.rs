use zaroxi_core_engine_render::{plan::ShellDrawPlan, ShellRenderTranscript};

#[test]
fn transcript_from_shell_draw_plan_is_deterministic() {
    // Construct a draw plan. Prefer Default if available; if not the test will
    // still exercise deterministic conversion for whatever ShellDrawPlan supports.
    let plan = ShellDrawPlan::default();
    let t1 = ShellRenderTranscript::from(&plan);
    let t2 = ShellRenderTranscript::from(&plan);
    assert_eq!(t1, t2, "transcript conversion must be deterministic");
    assert!(!t1.lines.is_empty(), "transcript should contain at least one line");
    let joined = t1.to_string();
    assert!(joined.contains("ShellDrawPlan") || joined.len() > 0, "transcript should contain content");
}
