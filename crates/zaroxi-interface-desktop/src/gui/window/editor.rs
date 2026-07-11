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

#[derive(Clone, Default)]
pub struct EditorContentData {
    pub tab_entries: Vec<TabEntry>,
    pub breadcrumb_label: String,
    pub editor_body_text: String,
    pub editor_spans: Option<Vec<(String, [f32; 4])>>,
    pub cursor_line: usize,
    pub cursor_col: usize,
    pub body_title: String,
    pub total_lines: usize,
    pub visible_line_range: Option<(usize, usize)>,
    /// When soft-wrap is active, maps visual row index (within the viewport
    /// window) → logical line index.  Index 0 is the first VISUAL row in the
    /// window.  Used by hit-testing, gutter, and cursor projection.
    pub visual_to_logical: Vec<usize>,
    /// Total visual row count within the viewport window (after wrapping).
    /// Used as `content_line_offset` upper bound and for scroll metrics.
    pub total_visual_lines: usize,
    /// Characters per visual row for the current wrap width.  Zero means
    /// no wrapping (full logical lines).  Derived from the visible editor
    /// text width ÷ monospace character advance.
    pub chars_per_row: usize,
    /// When wrapping is active, the visual row index within the wrapped
    /// content where the first visible logical line (scroll_top) begins.
    /// Used to set `content_line_offset` on the render block so the
    /// renderer can skip overscan rows directly.
    pub wrap_visual_offset: usize,
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

    pub fn build_breadcrumb_block(r: &ShellRegion, tokens: &StyleTokens) -> UiBlock {
        // The breadcrumb row currently shows the parent directory path, which
        // looks like cluttered `/home/.../crates/...` text above the code.
        // Suppress it entirely: the workbench tab strip already carries the
        // file identity (basename-first with disambiguation).  Future iterations
        // can reintroduce a lightweight breadcrumb when it is used for file
        // navigation rather than static display.
        UiBlock {
            id: r.id.to_string(),
            title: String::new(),
            rect: r.into(),
            header_color: Some(tokens.tab_strip_background.to_array()),
            content_color: Some(tokens.tab_strip_background.to_array()),
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

        let content_line_offset = if data.chars_per_row > 0 {
            None
        } else {
            data.visible_line_range.map(|(start, _)| start)
        };

        UiBlock {
            id: r.id.to_string(),
            title: String::new(),
            content: data.editor_body_text.clone(),
            rect: r.into(),
            header_color: Some(tokens.editor_content_background.to_array()),
            content_color: Some(tokens.editor_content_background.to_array()),
            content_spans: data.editor_spans.clone(),
            cursor_line: Some(data.cursor_line),
            cursor_col: Some(data.cursor_col),
            highlight_active_line: true,
            content_line_offset,
            content_offset_y: 0.0,
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
        visual_to_logical: &[usize],
        total_visual_lines: usize,
    ) -> UiBlock {
        if !dest.is_explorer() {
            return UiBlock {
                id: r.id.to_string(),
                rect: r.into(),
                header_color: Some(tokens.panel_background.to_array()),
                content_color: Some(tokens.panel_background.to_array()),
                ..Default::default()
            };
        }
        if total_visual_lines > 0 && !visual_to_logical.is_empty() {
            zaroxi_core_engine_ui::blocks::make_gutter_block_windowed(
                r.rect.x as f32,
                r.rect.y as f32,
                r.rect.width as f32,
                r.rect.height as f32,
                total_lines,
                0,
                total_visual_lines,
                tokens,
                visual_to_logical,
            )
        } else if let Some((start, end)) = visible_range {
            zaroxi_core_engine_ui::blocks::make_gutter_block_windowed(
                r.rect.x as f32,
                r.rect.y as f32,
                r.rect.width as f32,
                r.rect.height as f32,
                total_lines,
                start,
                end,
                tokens,
                &[],
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

    /// Build the bottom-panel base block. The body/title are authoritative only
    /// as a fallback: the live content (terminal viewport or Problems/Output
    /// placeholder) is projected on top each frame by
    /// `TerminalController::apply_bottom_panel`. No static "Ready" text is
    /// emitted here — the terminal owns this surface.
    pub fn build_bottom_panel_block(
        r: &ShellRegion,
        tokens: &StyleTokens,
        terminal_tabs: Option<&[String]>,
    ) -> UiBlock {
        let title = terminal_tabs
            .filter(|t| !t.is_empty())
            .map(|tabs| tabs.join(" \u{2022} "))
            .unwrap_or_else(|| "Terminal \u{2022} Problems \u{2022} Output".to_string());

        UiBlock {
            id: r.id.to_string(),
            title,
            content: String::new(),
            rect: r.into(),
            header_color: Some(tokens.bottom_panel_header_background.to_array()),
            content_color: Some(tokens.bottom_panel_background.to_array()),
            ..Default::default()
        }
    }
}
