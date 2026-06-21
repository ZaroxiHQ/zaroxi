//! Status bar view/layout layer.
//!
//! Assembles the panel fragments into left/right zones and produces the final
//! `UiBlock` for the shell to render.
//!
//! Rendering note: the engine treats the status bar as a **header-only** region
//! (a thin single-line strip with no body content area). The renderer paints
//! such regions' text from `UiBlock::title` across the full strip height and
//! culls anything placed in the body `content`/`content_spans` (the body area
//! collapses to zero height on a 26px strip). So the assembled status text is
//! emitted as the block **title**; `format_status_bar_spans` is still used to
//! get consistent left/right spacing.

use crate::gui::ShellRegion;
use zaroxi_core_engine_render::{Rect, UiBlock};
use zaroxi_core_engine_style::StyleTokens;
use zaroxi_core_engine_ui::chrome::{StatusBarZones, format_status_bar_spans};

use super::model::StatusModel;
use super::panels::{diagnostics, document_state, editor_position, file_format, workspace};
use super::style::StatusStyle;

fn render_debug_enabled() -> bool {
    std::env::var("ZAROXI_STATUS_RENDER_DEBUG").as_deref() == Ok("1")
}

/// Renders the status bar region from a [`StatusModel`].
pub struct StatusView;

/// Assemble the left/right zones from the panels. Left = primary/global state
/// (workspace, document state, diagnostics); right = contextual file/editor
/// state (position + selection, indent, encoding, line endings, language).
fn zones(model: &StatusModel) -> StatusBarZones {
    let mut left_segments = workspace::segments(model);
    left_segments.extend(document_state::segments(model));
    left_segments.extend(diagnostics::segments(model));

    let mut right_segments = editor_position::segments(model);
    right_segments.extend(file_format::segments(model));

    StatusBarZones { left_segments, right_segments }
}

impl StatusView {
    /// Build the status bar `UiBlock` for its shell region.
    pub fn build_block(r: &ShellRegion, tokens: &StyleTokens, model: &StatusModel) -> UiBlock {
        let style = StatusStyle::from_tokens(tokens);

        let zones = zones(model);
        let spans = format_status_bar_spans(&zones, tokens);
        // Concatenated status text. Emitted as the block title because the engine
        // renders the (header-only) status strip's text from `title`.
        let text: String = spans.iter().map(|(t, _)| t.as_str()).collect();

        let rect = Rect {
            x: r.rect.x as f32,
            y: r.rect.y as f32,
            w: r.rect.width as f32,
            h: r.rect.height as f32,
        };

        if render_debug_enabled() {
            eprintln!(
                "ZAROXI_STATUS_RENDER_DEBUG: left={:?} right={:?}",
                zones.left_segments, zones.right_segments
            );
            eprintln!(
                "ZAROXI_STATUS_RENDER_DEBUG: title_text={:?} rect=(x={:.0} y={:.0} w={:.0} h={:.0})",
                text, rect.x, rect.y, rect.w, rect.h
            );
        }

        UiBlock {
            id: r.id.to_string(),
            title: text,
            content: String::new(),
            visible: true,
            rect,
            header_color: Some(style.background),
            content_color: None,
            corner_radius: 0.0,
            border_color: None,
            border_width: 0.0,
            header_only: true,
            content_spans: None,
            cursor_line: None,
            cursor_col: None,
            highlight_active_line: false,
            selection_range: None,
            text_color: Some(style.primary_text),
            clip_rect: None,
            content_offset_x: 0.0,
            content_offset_y: 0.0,
            content_line_offset: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::super::model::{DiagnosticCounts, SelectionInfo, StatusInputs, StatusModel};
    use super::zones;

    fn rich_model() -> StatusModel {
        StatusModel::from_inputs(&StatusInputs {
            file_label: Some("src/main.rs"),
            workspace_name: Some("zaroxi"),
            cursor_line: 0,
            cursor_col: 0,
            text_sample: Some("fn main() {\n    let x = 1;\n}"),
            modified: true,
            parsing: false,
            readonly: false,
            selection: Some(SelectionInfo { chars: 5, lines: 1 }),
            diagnostics: Some(DiagnosticCounts { errors: 2, warnings: 1, ..Default::default() }),
        })
    }

    /// The visible content the bar renders (the same zones `build_block` formats)
    /// must actually contain the live fields when a file is open.
    #[test]
    fn open_file_renders_rich_visible_zones() {
        let z = zones(&rich_model());
        assert!(!z.left_segments.is_empty(), "left zone must render");
        assert!(!z.right_segments.is_empty(), "right zone must render");

        let rendered = format!("{} || {}", z.left_segments.join("  "), z.right_segments.join("  "));
        for expected in [
            "zaroxi",
            "Modified",
            "E 2 W 1",
            "Ln 1, Col 1",
            "Sel 5",
            "Spaces: 4",
            "UTF-8",
            "LF",
            "Rust",
        ] {
            assert!(
                rendered.contains(expected),
                "visible status bar missing {expected:?}; rendered: {rendered:?}"
            );
        }
    }

    /// With no file open the bar stays quiet: workspace state only, no editor
    /// position or file-format noise.
    #[test]
    fn no_file_renders_quiet_zones() {
        let z = zones(&StatusModel::default());
        assert_eq!(z.left_segments, vec!["No Workspace".to_string()]);
        assert!(z.right_segments.is_empty(), "no-file bar must not show editor/format fields");
    }

    /// The final `UiBlock` the renderer paints must carry the rich status text in
    /// the slot the engine actually renders for the header-only status strip
    /// (`title`), not in the culled body (`content`/`content_spans`).
    #[test]
    fn build_block_emits_text_in_rendered_title_slot() {
        let region = crate::gui::ShellRegion {
            id: "status_bar",
            name: "status_bar",
            rect: crate::gui::Rect { x: 0, y: 800, width: 1200, height: 26 },
        };
        let tokens = zaroxi_core_engine_style::test_utils::test_tokens_dark();

        let block = super::StatusView::build_block(&region, &tokens, &rich_model());

        assert!(block.header_only, "status strip must render as a header-only region");
        assert!(block.content.is_empty(), "body content is culled for the strip; must be empty");
        assert!(block.content_spans.is_none(), "body spans are culled; must be None");
        for expected in [
            "zaroxi",
            "Modified",
            "E 2 W 1",
            "Ln 1, Col 1",
            "Sel 5",
            "Spaces: 4",
            "UTF-8",
            "LF",
            "Rust",
        ] {
            assert!(
                block.title.contains(expected),
                "rendered status title missing {expected:?}; title: {:?}",
                block.title
            );
        }
    }
}
