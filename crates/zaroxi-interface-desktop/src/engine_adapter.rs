//! Engine adapter seam: maps Zaroxi-specific shell state into app-neutral
//! engine primitives (DocumentViewport, CaretModel, SelectionModel, ScrollModel).
//!
//! This adapter is the single canonical point where IDE/Shell concepts are
//! translated into the generic engine contract. Engine crates consume only
//! the app-neutral types defined in `zaroxi-core-engine-view::document_viewport`
//! and `zaroxi-core-engine-scene::text_span`.
//!
//! Design rules:
//! - This module may reference IDE-specific concepts (tabs, workspace, AI, etc.)
//! - It MUST NOT leak those concepts into engine types
//! - Each mapping function produces only app-neutral types
//! - All Shell/IDE knowledge stays within this adapter boundary

use zaroxi_core_engine_view::{
    CaretModel, DocumentViewport, RenderedDocument, ScrollModel, SelectionModel,
};

/// Build a generic `DocumentViewport` from visible lines and optional metadata.
///
/// This is the primary mapping from "what the shell is currently showing" into
/// the engine's text-view contract.
pub fn build_document_viewport(
    visible_lines: &[String],
    top_line: u32,
    total_lines: u32,
    summary: Option<&str>,
) -> DocumentViewport {
    DocumentViewport::new(
        top_line,
        total_lines,
        visible_lines.to_vec(),
        summary.map(|s| s.to_string()),
    )
}

/// Build a `CaretModel` from absolute document coordinates.
pub fn build_caret(line: u32, column: u32) -> CaretModel {
    CaretModel::new(line, column)
}

/// Build a `SelectionModel` from start/end document coordinates.
pub fn build_selection(
    start_line: u32,
    start_column: u32,
    end_line: u32,
    end_column: u32,
) -> SelectionModel {
    SelectionModel::new(start_line, start_column, end_line, end_column)
}

/// Build a `ScrollModel` from the current viewport position.
pub fn build_scroll(top_line: u32, total_lines: u32, visible_line_count: u32) -> ScrollModel {
    ScrollModel::new(top_line, total_lines, visible_line_count)
}

/// Build a complete `RenderedDocument` from shell text, caret, and scroll state.
///
/// This is the convenience entry point that produces the full engine hand-off.
pub fn build_rendered_document(
    visible_lines: &[String],
    top_line: u32,
    total_lines: u32,
    summary: Option<&str>,
    caret: Option<(u32, u32)>,
    selection: Option<(u32, u32, u32, u32)>,
    visible_line_count: u32,
) -> RenderedDocument {
    let viewport = build_document_viewport(visible_lines, top_line, total_lines, summary);

    let caret = caret.map(|(line, col)| build_caret(line, col));

    let selection =
        selection.map(|(sline, scol, eline, ecol)| build_selection(sline, scol, eline, ecol));

    let scroll = Some(build_scroll(top_line, total_lines, visible_line_count));

    RenderedDocument { viewport, caret, selection, scroll }
}

/// Build an absent/empty `RenderedDocument` (no active text view).
pub fn build_absent_document() -> RenderedDocument {
    RenderedDocument::absent()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn build_document_viewport_basic() {
        let lines = vec!["hello".to_string(), "world".to_string()];
        let vp = build_document_viewport(&lines, 1, 2, Some("Ln 1/2"));
        assert_eq!(vp.visible_lines, lines);
        assert_eq!(vp.top_line, 1);
        assert_eq!(vp.total_lines, 2);
        assert_eq!(vp.summary.as_deref(), Some("Ln 1/2"));
    }

    #[test]
    fn build_rendered_document_all_present() {
        let lines = vec!["fn main() {".to_string(), "}".to_string()];
        let doc = build_rendered_document(
            &lines,
            1,
            2,
            Some("Ln 0/2"),
            Some((1, 3)),
            Some((1, 0, 1, 5)),
            2,
        );
        assert_eq!(doc.viewport.visible_lines.len(), 2);
        assert_eq!(doc.caret.as_ref().unwrap().line, 1);
        assert_eq!(doc.caret.as_ref().unwrap().column, 3);
        assert_eq!(doc.selection.as_ref().unwrap().start_column, 0);
        assert_eq!(doc.selection.as_ref().unwrap().end_column, 5);
        let scroll = doc.scroll.unwrap();
        assert_eq!(scroll.top_line, 1);
        assert_eq!(scroll.total_lines, 2);
    }

    #[test]
    fn build_absent_document_is_empty() {
        let doc = build_absent_document();
        assert!(doc.viewport.visible_lines.is_empty());
        assert!(doc.caret.is_none());
        assert!(doc.selection.is_none());
    }

    #[test]
    fn scroll_proportions_are_valid() {
        let scroll = build_scroll(50, 100, 30);
        let ratio = scroll.viewport_ratio();
        let prop = scroll.scroll_proportion();
        assert!(ratio > 0.0 && ratio <= 1.0);
        assert!(prop >= 0.0 && prop <= 1.0);
    }
}
