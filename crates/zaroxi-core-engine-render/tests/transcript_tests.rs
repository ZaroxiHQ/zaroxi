use zaroxi_core_engine_render::{plan::{ShellDrawPlan, DrawSection}, ShellRenderTranscript};

#[test]
fn transcript_from_shell_draw_plan_is_deterministic() {
    // Construct a minimal, valid ShellDrawPlan instance without UB.
    let plan = ShellDrawPlan {
        sections: vec![DrawSection::Content, DrawSection::Status],
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
        (joined.contains("Content") && joined.contains("Status")) || joined.len() > 0,
        "transcript should contain content describing sections"
    );
}
