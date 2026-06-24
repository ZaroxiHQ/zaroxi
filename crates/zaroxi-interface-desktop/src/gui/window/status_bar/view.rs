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

use zaroxi_interface_widgets::{ContextBand, MarkerKind, MetaChips, StatusMarker};

use super::model::{DocumentState, StatusModel};
use super::panels::{diagnostics, document_state, editor_position, file_format, workspace};
use super::style::StatusStyle;

fn render_debug_enabled() -> bool {
    std::env::var("ZAROXI_STATUS_RENDER_DEBUG").as_deref() == Ok("1")
}

/// Whether the status composition trace (`ZAROXI_STATUS_TRACE=1`) is enabled.
fn status_trace_enabled() -> bool {
    std::env::var("ZAROXI_STATUS_TRACE").as_deref() == Ok("1")
}

/// Renders the status bar region from a [`StatusModel`].
pub struct StatusView;

/// Assemble the canonical left/right status zones from the shared [`StatusModel`].
///
/// This is the **single source of truth** for status content semantics. Both
/// renderers consume it: the legacy shell [`StatusView::build_block`] (only under
/// the explicit legacy fallback) and the default cockpit/widget `StatusBar`
/// (via the desktop cockpit input assembly). Having one presenter guarantees the
/// two renderers show identical details — the cockpit bar is no longer a poorer
/// subset.
///
/// Left = primary/global state (workspace, document state, diagnostics).
/// Right = contextual file/editor state (position + selection, then the
/// glanceable language, then the remaining format fields). The language is
/// rotated ahead of the lower-priority format fields so a narrow bar drops
/// encoding/EOL/indent before it drops the language.
pub fn status_zones(model: &StatusModel) -> StatusBarZones {
    let mut left_segments = workspace::segments(model);
    left_segments.extend(document_state::segments(model));
    left_segments.extend(diagnostics::segments(model));

    let mut right_segments = editor_position::segments(model);
    let mut fmt = file_format::segments(model);
    if !fmt.is_empty() {
        // file_format yields [indent, encoding, EOL, language]; move language
        // ahead of the lower-priority format fields.
        fmt.rotate_right(1);
    }
    right_segments.extend(fmt);

    StatusBarZones { left_segments, right_segments }
}

/// Map the shared [`StatusModel`] into the cockpit instrument-panel's typed
/// context + metadata bands. This is the canonical, shared presenter for the
/// cockpit's left band — it derives the same facts the legacy [`status_zones`]
/// does, just shaped into visual roles (breadcrumb leaf vs. ancestors, compact
/// state markers, collapsible metadata chips) instead of flat strings.
///
/// Symbol-path resolution (file → mod → fn → expr) needs cursor→symbol mapping
/// the syntax layer does not expose yet, so today the leaf is the file name and
/// the workspace is the sole ancestor; this is the honest best-available context
/// and degrades to file-only when no workspace is open.
pub fn instrument_context(model: &StatusModel) -> (ContextBand, MetaChips) {
    let mut ancestors = Vec::new();
    if let Some(ws) = &model.workspace {
        ancestors.push(ws.clone());
    }
    let leaf = model.file_name.clone().unwrap_or_else(|| "No file".to_string());
    let position = if model.has_file {
        Some(format!("Ln {}, Col {}", model.line + 1, model.column + 1))
    } else {
        None
    };

    let mut markers = Vec::new();
    if model.modified {
        markers.push(StatusMarker { kind: MarkerKind::Modified, count: None });
    }
    if model.document_state == DocumentState::Parsing {
        markers.push(StatusMarker { kind: MarkerKind::Parsing, count: None });
    }
    if let Some(d) = &model.diagnostics {
        if d.errors > 0 {
            markers.push(StatusMarker { kind: MarkerKind::Error, count: Some(d.errors) });
        }
        if d.warnings > 0 {
            markers.push(StatusMarker { kind: MarkerKind::Warning, count: Some(d.warnings) });
        }
    }

    let context = ContextBand { ancestors, leaf, position, markers };

    // Collapsible metadata chips (only meaningful with a file open).
    let meta = if model.has_file {
        MetaChips {
            language: model.language.clone(),
            indent: Some(model.indent.label()),
            eol: Some(model.line_ending.label().to_string()),
            encoding: Some(model.encoding.to_string()),
        }
    } else {
        MetaChips::default()
    };

    (context, meta)
}

impl StatusView {
    /// Build a **background-only** status strip block (no text).
    ///
    /// Used when the cockpit/widget layer owns the status bar (the default): the
    /// cockpit draws the segment *text* via the cosmic-text pass, but that pass
    /// runs before the vello overlay composite, so the strip's elevated
    /// background cannot live in the cockpit vector scene (it would paint over
    /// the text). Instead the shell shape pass draws this background strip
    /// *under* the text. This is purely a fill — all status content still comes
    /// from the shared [`status_zones`] presenter, never duplicated here.
    pub fn build_background_block(r: &ShellRegion, tokens: &StyleTokens) -> UiBlock {
        let style = StatusStyle::from_tokens(tokens);
        UiBlock {
            id: r.id.to_string(),
            title: String::new(),
            rect: r.into(),
            header_color: Some(style.background),
            header_only: true,
            text_color: Some(style.primary_text),
            ..Default::default()
        }
    }

    /// Build the status bar `UiBlock` for its shell region.
    pub fn build_block(r: &ShellRegion, tokens: &StyleTokens, model: &StatusModel) -> UiBlock {
        let style = StatusStyle::from_tokens(tokens);

        // Single canonical content source (shared with the cockpit renderer).
        let zones = status_zones(model);
        // Left group → block title (left-aligned). Joined for display; the engine
        // renders the (header-only) strip's title text. Guaranteed non-empty so
        // the strip never renders as a visually blank band.
        let mut left_text = zones.left_segments.join("  ");
        let used_fallback = left_text.trim().is_empty();
        if used_fallback {
            left_text = "No file".to_string();
        }

        // Right group → priority-ordered segments carried in `content_spans`. The
        // renderer right-aligns them and drops the lowest-priority (trailing) ones
        // first when the strip is narrow.
        let right_segments = zones.right_segments;
        let right_spans: Vec<(String, [f32; 4])> =
            right_segments.iter().map(|s| (s.clone(), style.primary_text)).collect();
        let content_spans = if right_spans.is_empty() { None } else { Some(right_spans) };

        let rect: Rect = r.into();

        if render_debug_enabled() {
            eprintln!(
                "ZAROXI_STATUS_RENDER_DEBUG[view]: left={:?} right={:?}",
                zones.left_segments, right_segments
            );
            eprintln!(
                "ZAROXI_STATUS_RENDER_DEBUG[view]: left_title={:?} rect=(x={:.0} y={:.0} w={:.0} h={:.0})",
                left_text, rect.x, rect.y, rect.w, rect.h
            );
        }

        if status_trace_enabled() {
            // Real data = an actual document and/or workspace is reflected, as
            // opposed to the pure "No file"/"No Workspace" fallback strip.
            let has_real_data = model.has_file || model.workspace.is_some();
            eprintln!(
                "ZAROXI_STATUS_TRACE: real_data={} fallback={} has_file={} workspace={:?} left={:?} right_segments={}",
                has_real_data,
                used_fallback,
                model.has_file,
                model.workspace,
                left_text,
                content_spans.as_ref().map(|s| s.len()).unwrap_or(0),
            );
        }

        UiBlock {
            id: r.id.to_string(),
            title: left_text,
            rect,
            header_color: Some(style.background),
            header_only: true,
            content_spans,
            text_color: Some(style.primary_text),
            ..Default::default()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::super::model::{DiagnosticCounts, SelectionInfo, StatusInputs, StatusModel};
    use super::status_zones;

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
        let z = status_zones(&rich_model());
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

    /// With no file open the bar stays quiet but still informative: it shows the
    /// workspace + document-identity fallbacks (never blank), and no editor
    /// position or file-format noise on the right.
    #[test]
    fn no_file_renders_informative_fallback_zones() {
        let z = status_zones(&StatusModel::default());
        assert_eq!(z.left_segments, vec!["No Workspace".to_string(), "No file".to_string()]);
        assert!(z.right_segments.is_empty(), "no-file bar must not show editor/format fields");
    }

    /// User-visible guarantee: the status strip is never a visually empty band.
    /// Even with completely default/empty data the rendered title carries stable
    /// fallback text.
    #[test]
    fn build_block_title_is_never_blank() {
        let region = crate::gui::ShellRegion {
            id: "status_bar",
            name: "status_bar",
            rect: crate::gui::Rect { x: 0, y: 800, width: 1200, height: 26 },
        };
        let tokens = zaroxi_core_engine_style::test_utils::test_tokens_dark();
        let block = super::StatusView::build_block(&region, &tokens, &StatusModel::default());
        assert!(!block.title.trim().is_empty(), "status title must never be blank");
        assert!(
            block.title.contains("No file") || block.title.contains("No Workspace"),
            "empty-data status must show a stable fallback; title: {:?}",
            block.title
        );
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
