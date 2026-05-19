use zaroxi_interface_desktop::presenters::gpu_shell::{GpuShellPresenter, GpuShellView, ShellRenderTranscript, ShellRegions, SlotName, ShellTone};

#[test]
fn transcript_includes_derived_shell_tone() {
    // Neutral (default)
    let width: u32 = 120;
    let height: u32 = 80;
    let chrome: u32 = 10;
    let status: u32 = 6;

    let regions = GpuShellPresenter::map_regions(width, height, chrome, status);
    let view = GpuShellView::from_shell_regions(&regions);
    let plan = crate::presenters::gpu_shell::GpuPaintPlan::from_view(&view);
    let transcript = ShellRenderTranscript::from_view_and_plan(width, height, &view, &plan);
    let txt = transcript.to_string();
    assert!(txt.contains("shell_tone: neutral"));

    // Ai case: ai_indicator present -> ai tone
    let mut r_ai = regions.clone();
    r_ai.ai_indicator = Some("ai:available".to_string());
    let view_ai = GpuShellView::from_shell_regions(&r_ai);
    let plan_ai = crate::presenters::gpu_shell::GpuPaintPlan::from_view(&view_ai);
    let transcript_ai = ShellRenderTranscript::from_view_and_plan(width, height, &view_ai, &plan_ai);
    let txt_ai = transcript_ai.to_string();
    assert!(txt_ai.contains("shell_tone: ai"));

    // Attention case: status_text present but no ai_indicator -> attention
    let mut r_att = regions.clone();
    r_att.status_text = Some("error".to_string());
    let view_att = GpuShellView::from_shell_regions(&r_att);
    let plan_att = crate::presenters::gpu_shell::GpuPaintPlan::from_view(&view_att);
    let transcript_att = ShellRenderTranscript::from_view_and_plan(width, height, &view_att, &plan_att);
    let txt_att = transcript_att.to_string();
    assert!(txt_att.contains("shell_tone: attention"));

    // Focused case: explicit focus_slot or content activity -> focused
    let mut r_f = regions.clone();
    r_f.focus_slot = Some(SlotName::ContentMain);
    let view_f = GpuShellView::from_shell_regions(&r_f);
    let plan_f = crate::presenters::gpu_shell::GpuPaintPlan::from_view(&view_f);
    let transcript_f = ShellRenderTranscript::from_view_and_plan(width, height, &view_f, &plan_f);
    let txt_f = transcript_f.to_string();
    assert!(txt_f.contains("shell_tone: focused"));
}
