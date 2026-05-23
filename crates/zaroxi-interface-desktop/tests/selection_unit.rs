use zaroxi_application_workspace::editor_service::EditorService;
use crate::clipboard::InMemoryClipboard;

#[test]
fn shift_arrow_expands_and_shrinks_selection() {
    let svc = EditorService::new_with_text("hello\nworld");
    // place cursor after 'hel' on line 1 (0-based)
    svc.buffer.lock().unwrap().set_cursor(0, 3, false);
    // shift+right x2 => selection from 3->5
    svc.arrow_right(true);
    svc.arrow_right(true);
    let sel = svc.get_selection_normalized().unwrap();
    assert_eq!(sel, (0, 3, 0, 5)); // selected "lo"
    // shift+left once => shrink selection to 4
    svc.arrow_left(true);
    let sel2 = svc.get_selection_normalized().unwrap();
    assert_eq!(sel2, (0, 3, 0, 4)); // selected "l"
}

#[test]
fn typing_replaces_selected_text_and_clears_selection() {
    let svc = EditorService::new_with_text("abcde");
    // select "bc" (1..3)
    {
        let mut b = svc.buffer.lock().unwrap();
        b.selection = Some(zaroxi_core_editor_buffer::buffer::Selection { anchor_line: 0, anchor_col: 1, active_line: 0, active_col: 3 });
    }
    svc.type_text("XY");
    assert_eq!(svc.get_text(), "aXYde");
    assert!(svc.get_selection_normalized().is_none());
}

#[test]
fn backspace_delete_removes_selected_text() {
    let svc = EditorService::new_with_text("ab\ncd");
    {
        let mut b = svc.buffer.lock().unwrap();
        b.cursor_line = 0;
        b.cursor_col = 2;
        b.selection = Some(zaroxi_core_editor_buffer::buffer::Selection { anchor_line: 0, anchor_col: 1, active_line: 1, active_col: 1 }); // selects "b\nc"
    }
    svc.backspace();
    assert_eq!(svc.get_text(), "a d");
}

#[test]
fn copy_cut_paste_via_inmemory_clipboard() {
    let svc = EditorService::new_with_text("line1\nline2\nline3");
    let clipboard = InMemoryClipboard::new();
    // select "ne2" from line2 (characters 2..5)
    {
        let mut b = svc.buffer.lock().unwrap();
        b.selection = Some(zaroxi_core_editor_buffer::buffer::Selection { anchor_line: 1, anchor_col: 1, active_line: 1, active_col: 4 });
    }
    // copy
    if let Some(t) = svc.copy_selection() {
        clipboard.set(t);
    } else {
        panic!("expected selection");
    }
    // paste at end of buffer
    {
        let mut b = svc.buffer.lock().unwrap();
        b.set_cursor(2, 5, false);
    }
    // paste
    if let Some(t) = clipboard.get() {
        svc.paste_text(&t);
    }
    assert_eq!(svc.get_text(), "line1\nline2\nline3ne2");
    // now select and cut
    {
        let mut b = svc.buffer.lock().unwrap();
        b.selection = Some(zaroxi_core_editor_buffer::buffer::Selection { anchor_line: 0, anchor_col: 0, active_line: 0, active_col: 5 });
    }
    // cut: copy then delete
    if let Some(t) = svc.copy_selection() {
        clipboard.set(t);
    }
    svc.delete_selection();
    assert_eq!(svc.get_text(), "line2\nline3ne2");
}

#[test]
fn paste_over_selection_replaces_selected_text() {
    let svc = EditorService::new_with_text("hello world");
    let clipboard = InMemoryClipboard::new();
    clipboard.set("X");
    {
        let mut b = svc.buffer.lock().unwrap();
        // select "world"
        b.selection = Some(zaroxi_core_editor_buffer::buffer::Selection { anchor_line: 0, anchor_col: 6, active_line: 0, active_col: 11 });
    }
    // paste over selection
    if let Some(text) = clipboard.get() {
        svc.paste_text(&text);
    }
    assert_eq!(svc.get_text(), "hello X");
}
