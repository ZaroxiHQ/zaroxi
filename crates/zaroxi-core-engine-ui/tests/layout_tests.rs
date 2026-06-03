#[cfg(test)]
mod tests {
    use zaroxi_core_engine_layout::build_shell_ui;
    use zaroxi_core_engine_style::test_utils::test_tokens_dark;

    #[test]
    fn basic_shell_rects_count_and_top_height() {
        let tokens = test_tokens_dark();
        let rects = build_shell_ui(800, 600, &tokens);
        // paint order: background + top + sidebar + editor + status
        assert!(rects.len() >= 5, "expected at least 5 rects, got {}", rects.len());

        // background is first
        let bg = &rects[0];
        assert_eq!(bg.x, 0.0);
        assert_eq!(bg.y, 0.0);
        assert_eq!(bg.width, 800.0);
        assert_eq!(bg.height, 600.0);

        // top bar height should match the design constant (30.0)
        let top = &rects[1];
        assert_eq!(top.height as i32, 30, "top bar height mismatch: {}", top.height);
    }
}
