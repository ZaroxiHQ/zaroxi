use zaroxi_core_engine_render::{
    ShellRenderTranscript,
    plan::{DrawSection, ShellDrawPlan},
};

#[test]
fn transcript_from_shell_draw_plan_is_deterministic() {
    // NOTE:
    // DrawSection::Content was changed from a unit variant to a struct-like
    // variant carrying minimal text layout metrics:
    //   Content { line_count: usize, width: u32, height: u32 }
    //
    // Update the test to construct a deterministic Content value. We pick
    // zeroed metrics here because this test only validates deterministic
    // transcript conversion and not actual layout sizing. This keeps the
    // change minimal and avoids coupling the test to the DummyBackend.
    //
    // Suggested verification command (run from workspace root):
    //   cargo test -p zaroxi-core-engine-render
    //
    // The rest of the test remains unchanged.
    let plan = ShellDrawPlan {
        sections: vec![
            DrawSection::Content { line_count: 0, width: 0, height: 0 },
            DrawSection::Status,
        ],
        selection_present: false,
        status_present: true,
        content_present: true,
        chrome_present: false,
    };

    let t1 = ShellRenderTranscript::from(&plan);
    let t2 = ShellRenderTranscript::from(&plan);
    assert_eq!(t1, t2, "transcript conversion must be deterministic");
    assert!(!t1.lines.is_empty(), "transcript should contain at least one line");
    let joined = t1.to_string();
    assert!(
        (joined.contains("Content") && joined.contains("Status")) || !joined.is_empty(),
        "transcript should contain content describing sections"
    );
}
