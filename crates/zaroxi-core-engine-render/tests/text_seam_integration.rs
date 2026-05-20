use zaroxi_core_engine_render::text_seam::layout_label_for_render;

#[test]
fn text_seam_produces_dummy_layout() {
    let layout = layout_label_for_render("hello", None);
    assert_eq!(layout.lines, vec!["hello".to_string()]);
    assert_eq!(layout.height, 16);
    assert_eq!(layout.width, 5u32.saturating_mul(8));
}

#[test]
fn text_seam_constructs_with_backend_when_enabled() {
    // Exercise the seam; regardless of which concrete backend is selected
    // (DummyBackend or GlyphonBackend) the returned layout should contain at
    // least one line. We avoid cfg(feature = "glyphon_backend") here because
    // crate-local cfg checks cannot reference features of other crates.
    let layout = layout_label_for_render("hello glyphon", None);
    assert!(!layout.lines.is_empty());
}
