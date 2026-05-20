use zaroxi_core_engine_render::text_seam::layout_label_for_render;

#[test]
fn text_seam_produces_dummy_layout() {
    let layout = layout_label_for_render("hello", None);
    assert_eq!(layout.lines, vec!["hello".to_string()]);
    assert_eq!(layout.height, 16);
    assert_eq!(layout.width, 5u32.saturating_mul(8));
}

#[cfg(feature = "glyphon_backend")]
#[test]
fn text_seam_constructs_with_glyphon_when_enabled() {
    // When the workspace is built with the glyphon_backend feature enabled for
    // zaroxi-core-engine-text this still exercises the seam without leaking types.
    let layout = layout_label_for_render("hello glyphon", None);
    assert!(!layout.lines.is_empty());
}
