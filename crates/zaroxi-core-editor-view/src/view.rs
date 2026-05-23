use zaroxi_core_editor_gutter::GutterModel;
use zaroxi_core_editor_viewport::EditorViewport;
use zaroxi_kernel_math::Rect;

/// Lightweight result returned by EditorView::layout.
///
/// Extended to carry minimal caret & selection view-state so presenters can
/// deterministically project caret/selection plan entries without recomputing
/// editor geometry. Fields are intentionally small and simple: cursor_line is
/// 1-based when present, cursor_column is 0-based, and selection is a simple
/// (start_line,start_col,end_line,end_col) tuple using 1-based lines.
#[derive(Clone, Debug)]
pub struct EditorViewLayout {
    /// Gutter area (absolute window-space).
    pub gutter_rect: Rect,
    /// Content/text area (absolute window-space).
    pub content_rect: Rect,
    /// Inclusive visible line range (0-based).
    pub visible_range: (u32, u32),
    /// Absolute y positions (window-space) for each visible line in order.
    pub line_positions: Vec<(u32, f32)>,

    /// Optional cursor line (1-based) when present.
    pub cursor_line: Option<u32>,
    /// Optional cursor column (0-based) when present.
    pub cursor_column: Option<u32>,
    /// Optional selection (start_line, start_col, end_line, end_col) with
    /// 1-based lines and 0-based columns when present.
    pub selection: Option<(u32, u32, u32, u32)>,
}

impl EditorViewLayout {
    /// Return the visible text lines (cloned) from the provided document slice.
    ///
    /// This helper preserves the top-to-bottom order of visible rows and will
    /// produce an output entry for each visible row. If the document is shorter
    /// than the visible range the missing rows are represented by empty strings
    /// so downstream consumers observe a stable row count.
    pub fn visible_texts(&self, doc: &[String]) -> Vec<String> {
        let (first, last) = self.visible_range;
        if doc.is_empty() {
            return Vec::new();
        }
        let mut out: Vec<String> = Vec::new();
        for idx in first..=last {
            match doc.get(idx as usize) {
                Some(s) => out.push(s.clone()),
                None => out.push(String::new()),
            }
        }
        out
    }
}

/// Optional lightweight state supplied when computing a layout that includes
/// cursor/selection information.
#[derive(Clone, Debug)]
pub struct EditorViewState {
    /// Cursor line (1-based).
    pub cursor_line: Option<u32>,
    /// Cursor column (0-based).
    pub cursor_column: Option<u32>,
    /// Selection range (start_line, start_col, end_line, end_col).
    pub selection: Option<(u32, u32, u32, u32)>,
}

/// EditorView composes gutter + viewport math and projects visible line positions
/// into absolute window coordinates suitable for presenters/renderers.
#[derive(Clone, Debug)]
pub struct EditorView {
    pub viewport: EditorViewport,
    pub gutter: GutterModel,
}

impl EditorView {
    pub fn new(viewport: EditorViewport, gutter: GutterModel) -> Self {
        EditorView { viewport, gutter }
    }

    /// Compute layout with optional editor state (cursor/selection).
    pub fn layout_with_state(
        &self,
        editor_rect: Rect,
        scroll_y: f32,
        total_lines: u32,
        state: Option<&EditorViewState>,
    ) -> EditorViewLayout {
        let gutter_rect = self.gutter.gutter_rect(editor_rect);
        let content_rect = self.gutter.content_rect(editor_rect);

        let (first, last) = self.viewport.visible_line_range(scroll_y);
        let last = std::cmp::min(last, total_lines.saturating_sub(1));
        let visible_range = if total_lines == 0 {
            (0, 0)
        } else {
            (first, last)
        };

        let mut line_positions: Vec<(u32, f32)> = Vec::new();
        for (li, y_rel) in self.viewport.visible_line_positions(scroll_y, total_lines) {
            // Convert relative content-top y to absolute window-space y.
            let abs_y = content_rect.y + y_rel;
            line_positions.push((li, abs_y));
        }

        let mut layout = EditorViewLayout {
            gutter_rect,
            content_rect,
            visible_range,
            line_positions,
            cursor_line: None,
            cursor_column: None,
            selection: None,
        };

        if let Some(s) = state {
            layout.cursor_line = s.cursor_line;
            layout.cursor_column = s.cursor_column;
            layout.selection = s.selection;
        }

        layout
    }

    /// Backwards-compatible layout() that does not include cursor/selection state.
    pub fn layout(&self, editor_rect: Rect, scroll_y: f32, total_lines: u32) -> EditorViewLayout {
        self.layout_with_state(editor_rect, scroll_y, total_lines, None)
    }
}
