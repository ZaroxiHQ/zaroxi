use zaroxi_core_editor_buffer::buffer::Buffer;

#[test]
fn buffer_editing_sets_dirty_and_saved_state_clears_it() {
    let mut b = Buffer::from_text("hello");
    // from_text sets saved_text and marks clean
    assert_eq!(b.dirty, false);
    b.replace_selection_or_insert("X");
    assert!(b.dirty, "buffer should be dirty after insert");
    b.set_saved_state();
    assert_eq!(b.dirty, false, "buffer should be clean after setting saved state");
    b.backspace();
    assert!(b.dirty, "buffer should be dirty after backspace");
}
