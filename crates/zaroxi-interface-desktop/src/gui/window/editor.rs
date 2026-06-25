/*!
Editor panel: tab strip, breadcrumb, content area, minimap, bottom panel.

Phase 50: panel-owned UiBlock construction.
Phase 73: tab strip uses chrome formatters with content_spans.
*/

use crate::gui::ShellRegion;
use zaroxi_core_engine_render::UiBlock;
use zaroxi_core_engine_style::StyleTokens;
use zaroxi_core_engine_ui::chrome::TabEntry;

use super::destination::WorkbenchDestination;

#[derive(Clone)]
pub struct EditorContentData {
    pub tab_entries: Vec<TabEntry>,
    pub breadcrumb_label: String,
    /// Viewport window of visible lines joined by '\n' (plus overscan above/below).
    /// For small files this may be the full document; for large files it is
    /// restricted to `visible_line_start..visible_line_end` to avoid O(file_size)
    /// string materialization on the render hot path.
    pub editor_body_text: String,
    pub editor_spans: Option<Vec<(String, [f32; 4])>>,
    pub cursor_line: usize,
    pub cursor_col: usize,
    pub body_title: String,
    /// Total logical line count (0-based count of lines in the document).
    /// Used for gutter numbering; avoids O(N) line-counting from `editor_body_text`.
    pub total_lines: usize,
    /// When set, `editor_body_text` contains only lines in [start, end) (with
    /// overscan applied).  `content_line_offset` on the render block is set
    /// to `start` so the renderer can compute the correct absolute
    /// screen position for each visible line.
    pub visible_line_range: Option<(usize, usize)>,
}

impl Default for EditorContentData {
    fn default() -> Self {
        Self {
            tab_entries: Vec::new(),
            breadcrumb_label: String::new(),
            editor_body_text: String::new(),
            editor_spans: None,
            cursor_line: 0,
            cursor_col: 0,
            body_title: String::new(),
            total_lines: 0,
            visible_line_range: None,
        }
    }
}

pub struct EditorPanel;

impl EditorPanel {
    pub fn build_tab_strip_block(
        r: &ShellRegion,
        tokens: &StyleTokens,
        _data: &EditorContentData,
        _dest: WorkbenchDestination,
    ) -> UiBlock {
        // Background only: the unified tab strip (file tabs + non-file workbench
        // tabs) is rendered by the cockpit `WorkbenchTabStrip` (overlay accents +
        // text labels), so the shape pass emits just the strip surface here.
        UiBlock {
            id: r.id.to_string(),
            rect: r.into(),
            header_color: Some(tokens.tab_strip_background.to_array()),
            content_color: Some(tokens.tab_strip_background.to_array()),
            ..Default::default()
        }
    }

    pub fn build_breadcrumb_block(
        r: &ShellRegion,
        tokens: &StyleTokens,
        data: &EditorContentData,
        dest: WorkbenchDestination,
    ) -> UiBlock {
        // Non-file destinations: the tab strip already carries the title, so the
        // breadcrumb row is redundant. Suppress it to keep non-file pages clean.
        if !dest.is_explorer() {
            return UiBlock {
                id: r.id.to_string(),
                title: String::new(),
                rect: r.into(),
                header_color: Some(tokens.tab_strip_background.to_array()),
                content_color: Some(tokens.tab_strip_background.to_array()),
                ..Default::default()
            };
        }
        // For file tabs: show only the parent directory (not the full path) to
        // avoid duplicating the basename already shown in the tab label.
        let dir_only = std::path::Path::new(&data.breadcrumb_label)
            .parent()
            .and_then(|p| p.to_str())
            .filter(|s| !s.is_empty())
            .map(|s| s.to_string())
            .unwrap_or_default();
        UiBlock {
            id: r.id.to_string(),
            title: dir_only,
            rect: r.into(),
            header_color: Some(tokens.editor_breadcrumb_background.to_array()),
            header_only: true,
            text_color: Some(tokens.text_muted.to_array()),
            ..Default::default()
        }
    }

    pub fn build_content_area_block(
        r: &ShellRegion,
        tokens: &StyleTokens,
        data: &EditorContentData,
        dest: WorkbenchDestination,
    ) -> UiBlock {
        // Non-Explorer destinations replace the file editor with a clean,
        // opaque page surface. The cockpit page (Settings / Extensions /
        // placeholder) draws its labels + accents on top in the text + overlay
        // passes. Suppressing the file body here is what makes the destination
        // visibly take over the main content region.
        if !dest.is_explorer() {
            return UiBlock {
                id: r.id.to_string(),
                rect: r.into(),
                header_color: Some(tokens.panel_background.to_array()),
                content_color: Some(tokens.panel_background.to_array()),
                ..Default::default()
            };
        }

        // When viewport-windowed, content_line_offset is the absolute
        // line number of the first line in the content String.  The
        // renderer uses this to compute the correct screen position.
        let content_line_offset = data.visible_line_range.map(|(start, _)| start);

        UiBlock {
            id: r.id.to_string(),
            title: data.body_title.clone(),
            content: data.editor_body_text.clone(),
            rect: r.into(),
            header_color: Some(tokens.editor_content_background.to_array()),
            content_color: Some(tokens.editor_content_background.to_array()),
            content_spans: data.editor_spans.clone(),
            cursor_line: Some(data.cursor_line),
            cursor_col: Some(data.cursor_col),
            highlight_active_line: true,
            content_line_offset,
            ..Default::default()
        }
    }

    /// Build a gutter block. When `visible_range` is set, only the
    /// line numbers within that window are materialised, and the block
    /// carries `content_line_offset` for absolute y-positioning so the
    /// renderer does not iterate through the full document.
    pub fn build_gutter_block(
        r: &ShellRegion,
        tokens: &StyleTokens,
        total_lines: usize,
        visible_range: Option<(usize, usize)>,
        dest: WorkbenchDestination,
    ) -> UiBlock {
        // Non-Explorer destinations have no file, so the gutter shows no line
        // numbers — just its background — to avoid phantom numbers beside the
        // settings / extensions / placeholder page.
        if !dest.is_explorer() {
            return UiBlock {
                id: r.id.to_string(),
                rect: r.into(),
                header_color: Some(tokens.editor_gutter_bg.to_array()),
                content_color: Some(tokens.editor_gutter_bg.to_array()),
                ..Default::default()
            };
        }
        if let Some((start, end)) = visible_range {
            zaroxi_core_engine_ui::blocks::make_gutter_block_windowed(
                r.rect.x as f32,
                r.rect.y as f32,
                r.rect.width as f32,
                r.rect.height as f32,
                total_lines,
                start,
                end,
                tokens,
            )
        } else {
            zaroxi_core_engine_ui::blocks::make_gutter_block(
                r.rect.x as f32,
                r.rect.y as f32,
                r.rect.width as f32,
                r.rect.height as f32,
                total_lines.max(1),
                tokens,
            )
        }
    }

    pub fn build_bottom_panel_block(
        r: &ShellRegion,
        tokens: &StyleTokens,
        terminal_tabs: Option<&[String]>,
    ) -> UiBlock {
        let title = terminal_tabs
            .filter(|t| !t.is_empty())
            .map(|tabs| tabs.join(" \u{2022} "))
            .unwrap_or_else(|| "Terminal \u{2022} Problems \u{2022} Output".to_string());

        let content = terminal_tabs
            .filter(|t| !t.is_empty())
            .map(|_tabs| "Ready".to_string())
            .unwrap_or_else(|| "No terminal session".to_string());

        UiBlock {
            id: r.id.to_string(),
            title,
            content,
            rect: r.into(),
            header_color: Some(tokens.bottom_panel_header_background.to_array()),
            content_color: Some(tokens.bottom_panel_background.to_array()),
            ..Default::default()
        }
    }
}
