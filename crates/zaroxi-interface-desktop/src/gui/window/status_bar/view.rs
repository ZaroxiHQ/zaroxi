//! Status bar view/layout layer.
//!
//! Assembles the panel fragments into left/right zones and produces the final
//! `UiBlock` for the shell to render. Span colouring and inter-segment spacing
//! reuse the engine's `format_status_bar_spans` chrome primitive (the same
//! approach used by the other shell panels), keeping layout consistent and the
//! spacing predictable.

use crate::gui::ShellRegion;
use zaroxi_core_engine_render::{Rect, UiBlock};
use zaroxi_core_engine_style::StyleTokens;
use zaroxi_core_engine_ui::chrome::{StatusBarZones, format_status_bar_spans};

use super::model::StatusModel;
use super::panels::{editor_position, file_format, workspace};
use super::style::StatusStyle;

/// Renders the status bar region from a [`StatusModel`].
pub struct StatusView;

impl StatusView {
    /// Build the status bar `UiBlock` for its shell region.
    pub fn build_block(r: &ShellRegion, tokens: &StyleTokens, model: &StatusModel) -> UiBlock {
        let style = StatusStyle::from_tokens(tokens);

        // Left = primary/global state; right = contextual file/editor state.
        let left_segments = workspace::segments(model);
        let mut right_segments = editor_position::segments(model);
        right_segments.extend(file_format::segments(model));

        let zones = StatusBarZones { left_segments, right_segments };
        let spans = format_status_bar_spans(&zones, tokens);
        let content: String = spans.iter().map(|(text, _)| text.as_str()).collect();

        let rect = Rect {
            x: r.rect.x as f32,
            y: r.rect.y as f32,
            w: r.rect.width as f32,
            h: r.rect.height as f32,
        };

        UiBlock {
            id: r.id.to_string(),
            title: String::new(),
            content,
            visible: true,
            rect,
            header_color: Some(style.background),
            content_color: Some(style.background),
            corner_radius: 0.0,
            border_color: None,
            border_width: 0.0,
            header_only: false,
            content_spans: Some(spans),
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
