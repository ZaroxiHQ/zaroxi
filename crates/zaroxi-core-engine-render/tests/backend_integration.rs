use zaroxi_core_engine_text::{TextLabel, new_backend};

#[test]
fn engine_render_can_construct_text_backend() {
    // Verify that the render crate (same "core" layer) can consume the text backend
    // purely through the Zaroxi-owned abstraction without touching Glyphon types.
    let backend = new_backend();
    let label = TextLabel::from("from-render");
    let layout = backend.layout_label(&label, None);
    assert_eq!(layout.lines[0], "from-render");
}
