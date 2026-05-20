use zaroxi_core_engine_text::{new_backend, TextLabel};

#[test]
fn backend_constructs_via_seam() {
    // Ensure the public seam constructs a backend and can layout a label.
    let backend = new_backend();
    let label = TextLabel::from("hello");
    let layout = backend.layout_label(&label, None);
    assert_eq!(layout.lines, vec!["hello".to_string()]);
    assert_eq!(layout.height, 16);
}

#[cfg(feature = "glyphon_backend")]
#[test]
fn glyphon_backend_via_seam() {
    // When the glyphon_backend feature is enabled the seam should still return a
    // Box<dyn TextBackend> and be usable without exposing Glyphon types.
    let backend = new_backend();
    let label = TextLabel::from("hello glyphon");
    let layout = backend.layout_label(&label, None);
    assert!(!layout.lines.is_empty());
}
