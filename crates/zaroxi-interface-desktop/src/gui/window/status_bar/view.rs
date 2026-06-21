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
use zaroxi_core_engine_ui::chrome::StatusBarZones;

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
        // Left group → block title (left-aligned). Joined for display; the engine
        // renders the (header-only) strip's title text.
        let left_text = zones.left_segments.join("  ");

        // Right group → priority-ordered segments carried in `content_spans`. The
        // renderer right-aligns them and drops the lowest-priority (trailing) ones
        // first when the strip is narrow. Order: position/selection, then the
        // glanceable language, then the remaining format fields.
        let mut right_segments = editor_position::segments(model);
        let mut fmt = file_format::segments(model);
        if !fmt.is_empty() {
            // file_format yields [indent, encoding, EOL, language]; move language
            // ahead of the lower-priority format fields.
            fmt.rotate_right(1);
        }
        right_segments.extend(fmt);
        let right_spans: Vec<(String, [f32; 4])> =
            right_segments.iter().map(|s| (s.clone(), style.primary_text)).collect();
        let content_spans = if right_spans.is_empty() { None } else { Some(right_spans) };

        let rect = Rect {
            x: r.rect.x as f32,
            y: r.rect.y as f32,
            w: r.rect.width as f32,
            h: r.rect.height as f32,
        };

        if render_debug_enabled() {
            eprintln!(
                "ZAROXI_STATUS_RENDER_DEBUG: left={:?} right={:?}",
                zones.left_segments, right_segments
            );
            eprintln!(
                "ZAROXI_STATUS_RENDER_DEBUG: left_title={:?} rect=(x={:.0} y={:.0} w={:.0} h={:.0})",
                left_text, rect.x, rect.y, rect.w, rect.h
            );
        }

        UiBlock {
            id: r.id.to_string(),
            title: left_text,
            content: String::new(),
            visible: true,
            rect,
            header_color: Some(style.background),
            content_color: None,
            corner_radius: 0.0,
            border_color: None,
            border_width: 0.0,
            header_only: true,
            content_spans,
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

    /// The final `UiBlock` carries the LEFT group in `title` (left-aligned slot)
    /// and the priority-ordered RIGHT segments in `content_spans` (right-aligned
    /// by the renderer). Both live in the header slot the engine paints — not the
    /// culled body.
    #[test]
    fn build_block_splits_left_title_and_right_segments() {
        let region = crate::gui::ShellRegion {
            id: "status_bar",
            name: "status_bar",
            rect: crate::gui::Rect { x: 0, y: 800, width: 1200, height: 26 },
        };
        let tokens = zaroxi_core_engine_style::test_utils::test_tokens_dark();

        let block = super::StatusView::build_block(&region, &tokens, &rich_model());

        assert!(block.header_only, "status strip must render as a header-only region");
        assert!(block.content.is_empty(), "body content is culled for the strip; must be empty");

        // Left group lives in the title.
        for expected in ["zaroxi", "Modified", "E 2 W 1"] {
            assert!(
                block.title.contains(expected),
                "left title missing {expected:?}; title: {:?}",
                block.title
            );
        }
        assert!(!block.title.contains("Ln 1, Col 1"), "right fields must not be in the left title");

        // Right group lives in content_spans (priority-ordered, right-aligned).
        let segs: Vec<String> = block
            .content_spans
            .as_ref()
            .expect("right segments must be present when a file is open")
            .iter()
            .map(|(t, _)| t.clone())
            .collect();
        let joined = segs.join(" ");
        for expected in ["Ln 1, Col 1", "Sel 5", "Spaces: 4", "UTF-8", "LF", "Rust"] {
            assert!(
                joined.contains(expected),
                "right segment missing {expected:?}; right: {:?}",
                joined
            );
        }
        // Language is prioritized ahead of the lower-priority format fields.
        let lang_idx = segs.iter().position(|s| s == "Rust").unwrap();
        let enc_idx = segs.iter().position(|s| s == "UTF-8").unwrap();
        assert!(lang_idx < enc_idx, "language keeps priority over encoding; segs: {segs:?}");
    }
}
