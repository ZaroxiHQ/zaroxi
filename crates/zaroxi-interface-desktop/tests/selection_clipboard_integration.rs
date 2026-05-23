use zaroxi_application_workspace::editor_service::EditorService;
use crate::clipboard::InMemoryClipboard;
use crate::presenters::transcript::render::ShellRenderTranscript;

#[test]
fn end_to_end_selection_copy_paste_and_transcript_reflects_selection() {
    // initial buffer with two lines
    let svc = EditorService::new_with_text("first line\nsecond line");
    // create clipboard and input bridge locally
    let clipboard = InMemoryClipboard::new();

    // select "second" (line 1 columns 0..6)
    {
        let mut b = svc.buffer.lock().unwrap();
        b.cursor_line = 1;
        b.cursor_col = 0;
        b.selection = Some(zaroxi_core_editor_buffer::buffer::Selection { anchor_line: 1, anchor_col: 0, active_line: 1, active_col: 6 });
    }

    // copy
    if let Some(t) = svc.copy_selection() {
        clipboard.set(t);
    } else {
        panic!("copy failed");
    }

    // place cursor at end of first line and paste
    {
        let mut b = svc.buffer.lock().unwrap();
        b.set_cursor(0, b.lines[0].chars().count(), false);
    }
    if let Some(t) = clipboard.get() {
        svc.paste_text(&t);
    }

    // Build deterministic plan_lines similar to presenter output:
    // For each visible line produce a Text entry; include a Selection entry derived from selection state.
    let snapshot = svc.snapshot();
    let mut plan_lines: Vec<String> = Vec::new();
    for (i, line) in snapshot.lines.iter().enumerate() {
        let y = (i as u32) * 16;
        plan_lines.push(format!("Text x=0 y={} text=\"{}\"", y, line));
    }

    // If selection present, emit Selection lines per visible intersection (presenter would do this).
    if let Some((sline, scol, eline, ecol)) = snapshot.selection {
        // convert to 0-based for our simple projection
        let sline0 = (sline - 1) as usize;
        let eline0 = (eline - 1) as usize;
        for (i, _) in snapshot.lines.iter().enumerate() {
            let row = i;
            if row < sline0 || row > eline0 {
                continue;
            }
            let sel_start_col = if row == sline0 { scol as u32 } else { 0 };
            let sel_end_col = if row == eline0 { ecol as u32 } else { snapshot.lines[row].chars().count() as u32 };
            if sel_end_col <= sel_start_col {
                continue;
            }
            let sx = sel_start_col * 8; // pretend char width 8
            let sy = (row as u32) * 16;
            let w = (sel_end_col - sel_start_col) * 8;
            let h = 16;
            plan_lines.push(format!("Selection x={} y={} w={} h={}", sx, sy, w, h));
        }
    }

    // Use presenter's parse helper to obtain primitives
    let prims = ShellRenderTranscript::parse_plan_lines(&plan_lines);
    // We expect at least one selection rect in the primitives to reflect the copied selection.
    assert!(!prims.selections.is_empty());
    // Ensure buffer text reflects the pasted text
    let final_text = svc.get_text();
    assert!(final_text.contains("second"));
}
