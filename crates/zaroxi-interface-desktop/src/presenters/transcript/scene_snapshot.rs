/// Small helpers to derive deterministic multi-line summaries from the
/// engine-owned ShellSceneModel. Factored out to keep the transcript render
/// implementation focused and small.
pub fn engine_scene_summary() -> String {
    use zaroxi_core_engine_scene::get_current_scene;

    let s = get_current_scene();
    let mut out: Vec<String> = Vec::new();
    out.push(format!("engine_total_lines: {}", s.viewport_total_lines));
    out.push(format!("engine_top_line: {}", s.viewport_top_line));
    out.push(format!("engine_cursor: {:?}:{:?}", s.cursor_line, s.cursor_column));
    // Emit up to the first 10 lines starting at top_line for visibility.
    let start = (s.viewport_top_line.max(1) - 1) as usize;
    for (i, line) in s.text_lines.iter().enumerate().skip(start).take(10) {
        out.push(format!("engine_line {}: {}", i + 1, line));
    }
    out.join("\n")
}
