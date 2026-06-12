use zaroxi_core_engine_text::{TextLabel, new_backend};

#[test]
fn backend_constructs_via_seam() {
    let backend = new_backend();
    let label = TextLabel::from("hello");
    let layout = backend.layout_label(&label, None);
    assert_eq!(layout.lines, vec!["hello".to_string()]);
    assert_eq!(layout.height, 16);
}
