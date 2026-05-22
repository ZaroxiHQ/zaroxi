#![allow(dead_code)]
#![allow(unused_imports)]
// Editor viewport crate: deterministic visible-line math and line positioning.
// Responsibilities:
// - Calculate which lines are visible for a given scroll offset.
// - Provide line -> y positioning relative to the content top.
// - Keep math stable and dependency-light so presenter/layout code can consume it.

mod viewport;
pub use viewport::EditorViewport;

#[cfg(test)]
mod tests {
    use super::EditorViewport;

    #[test]
    fn visible_range_basic() {
        let ev = EditorViewport::new(800, 200, 20.0, 48);
        let (first, last) = ev.visible_line_range(0.0);
        // At zero scroll the first visible line is 0 and last >= first.
        assert_eq!(first, 0);
        assert!(last >= first);
    }

    #[test]
    fn line_to_y_and_positions() {
        let ev = EditorViewport::new(800, 100, 10.0, 40);
        // Height 100 with line height 10 => 10 lines fit.
        assert_eq!(ev.total_lines_fit(), 10);
        // line 3 at scroll 0 should be at y = 3*10
        assert_eq!(ev.line_to_y(3, 0.0), 30.0);
        // visible positions should map to consecutive y offsets
        let positions = ev.visible_line_positions(5.5, 100);
        assert!(!positions.is_empty());
    }
}
