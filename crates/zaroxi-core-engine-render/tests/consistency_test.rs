use zaroxi_core_engine_render::{ShellDrawPlan, analyze};

#[test]
fn default_plan_is_consistent_between_transcript_and_renderer() {
    // Default plan is intentionally deterministic and should match between
    // ShellRenderTranscript and ShellTextRenderer under the seam rules.
    let plan = ShellDrawPlan::default();
    let report = analyze(&plan);

    assert!(report.aligned, "Consistency check failed: {:?}", report.mismatches);
}
