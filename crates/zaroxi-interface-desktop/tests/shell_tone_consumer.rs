use zaroxi_interface_desktop::presenters::gpu_shell::{GpuShellPresenter, GpuShellView, SlotName};

/// Focused test proving that the presenter's new debug consumer surface
/// deterministically exposes the derived `shell_tone`.
#[test]
fn debug_summary_includes_shell_tone() {
    let width: u32 = 120;
    let height: u32 = 80;
    let chrome: u32 = 10;
    let status: u32 = 6;

    let base_regions = GpuShellPresenter::map_regions(width, height, chrome, status);

    // Neutral (default)
    let view_neutral = GpuShellView::from_shell_regions(&base_regions);
    assert_eq!(GpuShellPresenter::debug_summary(&view_neutral), "shell_tone=neutral".to_string());

    // Ai case -> ai tone
    let mut r_ai = base_regions.clone();
    r_ai.ai_indicator = Some("ai:available".to_string());
    let view_ai = GpuShellView::from_shell_regions(&r_ai);
    assert_eq!(GpuShellPresenter::debug_summary(&view_ai), "shell_tone=ai".to_string());

    // Attention case -> attention tone
    let mut r_att = base_regions.clone();
    r_att.status_text = Some("error".to_string());
    let view_att = GpuShellView::from_shell_regions(&r_att);
    assert_eq!(GpuShellPresenter::debug_summary(&view_att), "shell_tone=attention".to_string());

    // Focused case -> focused tone (explicit focus_slot)
    let mut r_f = base_regions.clone();
    r_f.focus_slot = Some(SlotName::ContentMain);
    let view_f = GpuShellView::from_shell_regions(&r_f);
    assert_eq!(GpuShellPresenter::debug_summary(&view_f), "shell_tone=focused".to_string());
}
