use zaroxi_core_engine_render::{plan::ShellDrawPlan, ShellRenderTranscript};

#[test]
fn transcript_from_shell_draw_plan_is_deterministic() {
    // Construct a draw plan without relying on Default (not all versions of
    // ShellDrawPlan implement Default). We create a zeroed instance unsafely
    // to avoid depending on the concrete fields; the goal of this test is to
    // ensure the conversion is deterministic and stable for the same input.
    let plan: ShellDrawPlan = unsafe { std::mem::zeroed() };

    let t1 = ShellRenderTranscript::from(&plan);
    let t2 = ShellRenderTranscript::from(&plan);
    assert_eq!(t1, t2, "transcript conversion must be deterministic");
    assert!(!t1.lines.is_empty(), "transcript should contain at least one line");
    let joined = t1.to_string();
    assert!(joined.len() > 0, "transcript should contain content");
}
