use zaroxi_core_editor_buffer::buffer::{Buffer, Selection};

#[test]
fn undo_grouped_typing_and_redo() {
    let mut b = Buffer::from_text("");
    // Type 'a' then 'b' as two single-char inserts; grouping merges them.
    b.replace_selection_or_insert("a");
    b.replace_selection_or_insert("b");
    assert_eq!(b.to_text(), "ab");
    // Undo should revert both grouped typing edits.
    assert!(b.undo());
    assert_eq!(b.to_text(), "");
    // Redo restores.
    assert!(b.redo());
    assert_eq!(b.to_text(), "ab");
}

#[test]
fn undo_restores_replaced_selected_text() {
    let mut b = Buffer::from_text("hello");
    // select "hel"
    b.selection = Some(Selection { anchor_line: 0, anchor_col: 0, active_line: 0, active_col: 3 });
    b.replace_selection_or_insert("J");
    assert_eq!(b.to_text(), "Jlo");
    assert!(b.undo());
    assert_eq!(b.to_text(), "hello");
}

#[test]
fn undo_after_delete_over_selection_restores_text_and_cursor() {
    let mut b = Buffer::from_text("one\ntwo\nthree");
    // select "two\n"
    b.selection = Some(Selection { anchor_line: 1, anchor_col: 0, active_line: 1, active_col: 3 });
    // perform delete (record undo)
    assert!(b.delete_selection_and_return_cursor_at_start(true));
    assert_eq!(b.to_text(), "one\n\nthree");
    // undo
    assert!(b.undo());
    assert_eq!(b.to_text(), "one\ntwo\nthree");
}

#[test]
fn undo_redo_paste_over_selection() {
    let mut b = Buffer::from_text("abc");
    // select "b"
    b.selection = Some(Selection { anchor_line: 0, anchor_col: 1, active_line: 0, active_col: 2 });
    b.replace_selection_or_insert("Z"); // paste-like replace
    assert_eq!(b.to_text(), "aZc");
    assert!(b.undo());
    assert_eq!(b.to_text(), "abc");
    assert!(b.redo());
    assert_eq!(b.to_text(), "aZc");
}

#[test]
fn cursor_and_selection_restored_after_undo_redo() {
    let mut b = Buffer::from_text("line");
    // place cursor after 'l'
    b.set_cursor(0, 1, false);
    // anchor and select next two chars
    b.anchor_selection_here();
    b.update_selection_active(0, 3);
    // replace selection
    b.replace_selection_or_insert("X");
    // after replace cursor at end of 'X' (col 1)
    assert_eq!(b.cursor_col, 1);
    // Undo should restore original selection and cursor
    assert!(b.undo());
    assert_eq!(b.selection.as_ref().unwrap().normalized(), (0, 1, 0, 3));
    // Redo should reapply edit and clear selection
    assert!(b.redo());
    assert!(b.selection.is_none());
    assert_eq!(b.to_text(), "lXe");
}
