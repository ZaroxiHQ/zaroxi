//! Cockpit overlay wiring: turns desktop app state into a vello cockpit scene.
//!
//! This is the desktop-side integration of [`zaroxi_interface_widgets`]: it lays
//! out the cockpit regions with **taffy**, composes the cockpit
//! [`WidgetTree`](zaroxi_interface_widgets::WidgetTree) from a per-frame
//! [`CockpitInputs`] snapshot, and paints it into a `vello::Scene`.
//!
//! Building the scene is GPU-free (vello scene encoding is CPU-side). Putting
//! those pixels on the window surface is the separate, feature-gated
//! `vello_pipeline` composite step (see `zaroxi-core-engine-render-backend`),
//! which requires on-device validation against this workspace's wgpu. The frame
//! loop builds this scene only when `ZAROXI_COCKPIT=1`, so default rendering is
//! unchanged.

use taffy::prelude::*;
use vello::Scene;
use zaroxi_core_editor_rope::LineIndex;
use zaroxi_core_platform_syntax::SymbolKind as SyntaxSymbolKind;
use zaroxi_core_platform_syntax::highlight::HighlightSpan;
use zaroxi_interface_theme::ZaroxiTheme;
use zaroxi_interface_widgets::components::{DiffHunk, MinimapSymbol};
use zaroxi_interface_widgets::{
    AiPredictionGutter, CockpitTokens, CommandPalette, LivingDiffLayer, LspStatus, PaletteItem,
    PredictionCell, SemanticMinimap, StatusBar, SymbolKind, WidgetTree,
};

/// Status-bar height (px) used for the cockpit layout.
const STATUS_H: f32 = 24.0;
/// AI prediction gutter width (px) — spec: thin 16px right gutter.
const PREDICTION_GUTTER_W: f32 = 16.0;
/// Semantic minimap rail width (px).
const MINIMAP_W: f32 = 84.0;

/// Per-frame snapshot of the app state the cockpit widgets consume.
///
/// Built cheaply from whatever the frame loop has available; fields default to
/// empty so partial wiring still produces a valid scene.
#[derive(Debug, Clone, Default)]
pub struct CockpitInputs {
    /// Surface width in logical px.
    pub width: f32,
    /// Surface height in logical px.
    pub height: f32,
    /// Editor line height in px (for diff/gutter row mapping).
    pub line_height: f32,
    /// Total document line count.
    pub total_lines: usize,
    /// Minimap symbols `(line, kind)`.
    pub minimap_symbols: Vec<MinimapSymbol>,
    /// AI-modified line ranges for the minimap.
    pub ai_regions: Vec<(usize, usize)>,
    /// Visible viewport fraction `(top, bottom)`.
    pub viewport: (f32, f32),
    /// Inline AI diff hunks.
    pub diff_hunks: Vec<DiffHunk>,
    /// AI prediction heat cells.
    pub prediction_cells: Vec<PredictionCell>,
    /// Symbol-path breadcrumb (file → mod → fn → expr).
    pub breadcrumb: Vec<String>,
    /// LSP health.
    pub lsp: LspStatus,
    /// AI context tokens used / available.
    pub ai_tokens_used: u32,
    /// AI context tokens available.
    pub ai_tokens_total: u32,
    /// Command palette: `Some((items, selected, rtl))` when open.
    pub palette: Option<(Vec<PaletteItem>, usize, bool)>,
    /// Animation phase in `[0,1)` (advanced by the host clock).
    pub phase: f32,
}

/// Map the active desktop theme to a cockpit token set.
pub fn cockpit_tokens(theme: ZaroxiTheme, system_is_dark: bool) -> CockpitTokens {
    match theme.resolve(system_is_dark) {
        ZaroxiTheme::Light => CockpitTokens::light(),
        // Dark (and resolved System) map to Void by default.
        _ => CockpitTokens::void(),
    }
}

/// Legacy `ZAROXI_COCKPIT` flag. Retained for compatibility/diagnostics only —
/// it no longer gates the desired UI. The cockpit/widget status + overview
/// surfaces are now the **default** ownership (see [`cockpit_surfaces_active`]),
/// so the desired UI appears without any env var.
pub fn cockpit_enabled() -> bool {
    matches!(std::env::var("ZAROXI_COCKPIT").as_deref(), Ok("1"))
}

/// Whether the explicit legacy-shell fallback is requested
/// (`ZAROXI_LEGACY_SHELL_SURFACES=1`). This is the migration safety switch: when
/// set, the legacy shell status surface owns the bottom bar and the cockpit
/// overlay surfaces are suppressed. Default (unset) = the new cockpit path.
pub fn legacy_shell_surfaces() -> bool {
    matches!(std::env::var("ZAROXI_LEGACY_SHELL_SURFACES").as_deref(), Ok("1"))
}

/// Whether the cockpit/widget-owned surfaces (status bar + overview/minimap) are
/// the active owners. This is the **default** — true unless the legacy fallback
/// is explicitly enabled. It is the single predicate the render loop and the
/// shell composition use to guarantee mutual exclusivity (exactly one owner per
/// surface, never both, never none).
pub fn cockpit_surfaces_active() -> bool {
    !legacy_shell_surfaces()
}

/// Build the breadcrumb from live editor state: the file basename plus the
/// cursor position. The full symbol path (`mod → fn → expr`) needs tree-sitter
/// symbol resolution, which the syntax layer does not expose yet, so this is the
/// honest, cheap subset available today.
pub fn breadcrumb(active_file: Option<&str>, cursor_line: usize, cursor_col: usize) -> Vec<String> {
    let mut out = Vec::new();
    if let Some(path) = active_file {
        let name = path.rsplit(['/', '\\']).next().unwrap_or(path);
        if !name.is_empty() {
            out.push(name.to_string());
        }
    }
    out.push(format!("Ln {}", cursor_line + 1));
    out.push(format!("Col {}", cursor_col + 1));
    out
}

/// Best-effort viewport fraction centered on the cursor line.
///
/// `EditorViewport` does not expose the first-visible line, so a precise visible
/// band is not derivable; this approximates one around the cursor so the minimap
/// thumb tracks editing. A precise band needs first-visible-line plumbing.
pub fn cursor_viewport(cursor_line: usize, total_lines: usize) -> (f32, f32) {
    if total_lines <= 1 {
        return (0.0, 1.0);
    }
    let c = cursor_line as f32 / total_lines as f32;
    let top = (c - 0.05).clamp(0.0, 0.95);
    (top, (top + 0.15).min(1.0))
}

/// Map a syntax-layer structural [`SyntaxSymbolKind`] to the minimap's
/// [`SymbolKind`]. Namespaces render as the minimap's "import" hairline glyph.
fn to_widget_kind(kind: SyntaxSymbolKind) -> SymbolKind {
    match kind {
        SyntaxSymbolKind::Function => SymbolKind::Function,
        SyntaxSymbolKind::Type => SymbolKind::Type,
        SyntaxSymbolKind::Namespace => SymbolKind::Import,
    }
}

/// Extract minimap symbols from full-document highlight `spans`.
///
/// Builds a byte→line [`LineIndex`] from `source` (whose byte offsets match the
/// spans by the editor's document contract), runs the syntax layer's structural
/// [`extract_symbols`](zaroxi_core_platform_syntax::extract_symbols), and maps
/// each result onto a [`MinimapSymbol`]. Cost is `O(source_bytes)` for the line
/// index plus `O(spans)`; callers should recompute only when the spans change.
pub fn extract_minimap_symbols(spans: &[HighlightSpan], source: &str) -> Vec<MinimapSymbol> {
    let line_index = LineIndex::from_str(source);
    let doc_symbols =
        zaroxi_core_platform_syntax::extract_symbols(spans, source, line_index.line_starts());
    doc_symbols
        .into_iter()
        .map(|s| MinimapSymbol { line: s.line, kind: to_widget_kind(s.kind) })
        .collect()
}

/// Region rectangles computed by the taffy pass.
struct Regions {
    editor: taffy::Layout,
    prediction_gutter: taffy::Layout,
    minimap: taffy::Layout,
    status: taffy::Layout,
}

/// Lay out the cockpit regions with taffy:
/// `column[ row[ editor(grow) | prediction_gutter(16) | minimap(84) ] | status(24) ]`.
fn layout_regions(width: f32, height: f32) -> Regions {
    let mut taffy: TaffyTree<()> = TaffyTree::new();

    let editor = taffy
        .new_leaf(Style {
            flex_grow: 1.0,
            min_size: Size { width: length(0.0), height: auto() },
            ..Default::default()
        })
        .unwrap();
    let prediction_gutter = taffy
        .new_leaf(Style {
            size: Size { width: length(PREDICTION_GUTTER_W), height: auto() },
            flex_shrink: 0.0,
            ..Default::default()
        })
        .unwrap();
    let minimap = taffy
        .new_leaf(Style {
            size: Size { width: length(MINIMAP_W), height: auto() },
            flex_shrink: 0.0,
            ..Default::default()
        })
        .unwrap();
    let body = taffy
        .new_with_children(
            Style { flex_grow: 1.0, flex_direction: FlexDirection::Row, ..Default::default() },
            &[editor, prediction_gutter, minimap],
        )
        .unwrap();
    let status = taffy
        .new_leaf(Style {
            size: Size { width: auto(), height: length(STATUS_H) },
            flex_shrink: 0.0,
            ..Default::default()
        })
        .unwrap();
    let root = taffy
        .new_with_children(
            Style {
                flex_direction: FlexDirection::Column,
                size: Size { width: length(width), height: length(height) },
                ..Default::default()
            },
            &[body, status],
        )
        .unwrap();

    taffy
        .compute_layout(
            root,
            Size {
                width: AvailableSpace::Definite(width),
                height: AvailableSpace::Definite(height),
            },
        )
        .unwrap();

    // Body is offset within root; leaf locations are relative to their parent,
    // so add the body offset to its children to get window-space rects.
    let body_l = *taffy.layout(body).unwrap();
    let mut editor_l = *taffy.layout(editor).unwrap();
    let mut pred_l = *taffy.layout(prediction_gutter).unwrap();
    let mut mini_l = *taffy.layout(minimap).unwrap();
    for l in [&mut editor_l, &mut pred_l, &mut mini_l] {
        l.location.x += body_l.location.x;
        l.location.y += body_l.location.y;
    }
    Regions {
        editor: editor_l,
        prediction_gutter: pred_l,
        minimap: mini_l,
        status: *taffy.layout(status).unwrap(),
    }
}

/// Build a centered overlay layout (for the palette) inside `host`.
fn centered(host: &taffy::Layout, w: f32, h: f32) -> taffy::Layout {
    let x = host.location.x + (host.size.width - w).max(0.0) * 0.5;
    let y = host.location.y + (host.size.height - h).max(0.0) * 0.25;
    taffy::Layout {
        location: taffy::geometry::Point { x, y },
        size: taffy::geometry::Size { width: w, height: h },
        ..Default::default()
    }
}

/// Compose the cockpit [`WidgetTree`] from a frame snapshot.
pub fn build_cockpit(inputs: &CockpitInputs) -> WidgetTree {
    let regions = layout_regions(inputs.width, inputs.height);
    let line_height = if inputs.line_height > 0.0 { inputs.line_height as f64 } else { 18.0 };
    let mut tree = WidgetTree::new();

    // Inline AI diff overlay (above editor text, below cursor).
    tree.push(
        Box::new(LivingDiffLayer {
            hunks: inputs.diff_hunks.clone(),
            line_height,
            active: None,
            phase: inputs.phase,
        }),
        regions.editor,
    );

    // AI prediction gutter (right side).
    tree.push(
        Box::new(AiPredictionGutter {
            cells: inputs.prediction_cells.clone(),
            line_height,
            pulse_line: inputs
                .prediction_cells
                .iter()
                .find(|c| c.probability >= 0.8)
                .map(|c| c.line),
            phase: inputs.phase,
        }),
        regions.prediction_gutter,
    );

    // Semantic minimap (far right rail).
    tree.push(
        Box::new(SemanticMinimap {
            symbols: inputs.minimap_symbols.clone(),
            total_lines: inputs.total_lines,
            ai_regions: inputs.ai_regions.clone(),
            viewport: inputs.viewport,
        }),
        regions.minimap,
    );

    // Instrument-panel status bar.
    tree.push(
        Box::new(StatusBar {
            breadcrumb: inputs.breadcrumb.clone(),
            lsp: inputs.lsp,
            ai_tokens_used: inputs.ai_tokens_used,
            ai_tokens_total: inputs.ai_tokens_total,
            phase: inputs.phase,
        }),
        regions.status,
    );

    // Command palette overlay (when open).
    if let Some((items, selected, rtl)) = &inputs.palette {
        let palette_layout = centered(&regions.editor, 520.0, 320.0);
        tree.push(
            Box::new(CommandPalette {
                results: items.clone(),
                selected: *selected,
                rtl: *rtl,
                row_height: 28.0,
            }),
            palette_layout,
        );
    }

    tree
}

/// Build the cockpit tree and paint it into a fresh `vello::Scene`.
pub fn paint_cockpit(inputs: &CockpitInputs, tokens: &CockpitTokens) -> Scene {
    let tree = build_cockpit(inputs);
    let mut scene = Scene::new();
    tree.paint(&mut scene, tokens);
    scene
}

/// Convert a widget text run into the render crate's cockpit-text type.
fn to_render_text(
    t: zaroxi_interface_widgets::WidgetText,
) -> zaroxi_core_engine_render::renderer::CockpitText {
    zaroxi_core_engine_render::renderer::CockpitText {
        text: t.text,
        x: t.x,
        y: t.y,
        size_px: t.size_px,
        color: t.color,
    }
}

/// Build the cockpit tree once and return both the vello vector scene and the
/// positioned text runs (the latter drawn by the cosmic-text layer).
pub fn build_cockpit_frame(
    inputs: &CockpitInputs,
    tokens: &CockpitTokens,
) -> (Scene, Vec<zaroxi_core_engine_render::renderer::CockpitText>) {
    let tree = build_cockpit(inputs);
    let mut scene = Scene::new();
    tree.paint(&mut scene, tokens);
    let text = tree.collect_text(tokens).into_iter().map(to_render_text).collect();
    (scene, text)
}

#[cfg(test)]
mod tests {
    use super::*;
    use zaroxi_interface_widgets::SymbolKind;

    fn sample() -> CockpitInputs {
        CockpitInputs {
            width: 1200.0,
            height: 800.0,
            line_height: 18.0,
            total_lines: 320,
            minimap_symbols: vec![
                MinimapSymbol { line: 5, kind: SymbolKind::Function },
                MinimapSymbol { line: 40, kind: SymbolKind::Type },
            ],
            ai_regions: vec![(30, 36)],
            viewport: (0.1, 0.25),
            diff_hunks: vec![DiffHunk { line: 2, added: true }],
            prediction_cells: vec![PredictionCell { line: 3, probability: 0.9 }],
            breadcrumb: vec!["main.rs".into(), "run".into()],
            lsp: LspStatus::Healthy,
            ai_tokens_used: 2048,
            ai_tokens_total: 8192,
            palette: Some((
                vec![PaletteItem { label: "افتح ملف".into(), shortcut: "Ctrl+O".into() }],
                0,
                true,
            )),
            phase: 0.3,
        }
    }

    #[test]
    fn regions_are_within_surface_and_sized() {
        let r = layout_regions(1200.0, 800.0);
        assert!((r.status.size.height - STATUS_H).abs() < 0.5);
        assert!((r.minimap.size.width - MINIMAP_W).abs() < 0.5);
        assert!((r.prediction_gutter.size.width - PREDICTION_GUTTER_W).abs() < 0.5);
        // Editor takes the remaining width.
        assert!(r.editor.size.width > 800.0);
        // Status bar sits at the bottom.
        assert!(r.status.location.y >= r.editor.size.height - 1.0);
    }

    #[test]
    fn build_places_all_widgets() {
        let tree = build_cockpit(&sample());
        // 4 base widgets + palette overlay = 5.
        assert_eq!(tree.len(), 5);
    }

    #[test]
    fn paint_produces_scene_without_panic() {
        let tokens = cockpit_tokens(ZaroxiTheme::Dark, true);
        let _scene = paint_cockpit(&sample(), &tokens);
        // Light theme maps to the light token set.
        let light = cockpit_tokens(ZaroxiTheme::Light, false);
        assert!(!light.is_dark);
    }

    #[test]
    fn breadcrumb_uses_basename_and_one_based_cursor() {
        let b = breadcrumb(Some("/home/u/proj/src/main.rs"), 41, 7);
        assert_eq!(b, vec!["main.rs".to_string(), "Ln 42".to_string(), "Col 8".to_string()]);
        // No file -> just cursor.
        let b = breadcrumb(None, 0, 0);
        assert_eq!(b, vec!["Ln 1".to_string(), "Col 1".to_string()]);
    }

    #[test]
    fn cursor_viewport_is_bounded_and_tracks_cursor() {
        assert_eq!(cursor_viewport(0, 0), (0.0, 1.0));
        let (t0, b0) = cursor_viewport(0, 100);
        assert!(t0 >= 0.0 && b0 <= 1.0 && t0 < b0);
        let (t_top, _) = cursor_viewport(10, 100);
        let (t_bot, _) = cursor_viewport(90, 100);
        assert!(t_bot > t_top, "viewport top tracks cursor downward");
    }

    #[test]
    fn extract_minimap_symbols_maps_kinds_and_lines() {
        use zaroxi_core_platform_syntax::highlight::{Highlight, HighlightSpan};
        let source = "fn run() {}\ntype Foo = u8;\nuse std::io;";
        let fn_at = source.find("run").unwrap();
        let ty_at = source.find("Foo").unwrap();
        let ns_at = source.find("std").unwrap();
        let spans = vec![
            HighlightSpan { start: fn_at, end: fn_at + 3, highlight: Highlight::Function },
            HighlightSpan { start: ty_at, end: ty_at + 3, highlight: Highlight::Type },
            HighlightSpan { start: ns_at, end: ns_at + 3, highlight: Highlight::Namespace },
        ];
        let syms = extract_minimap_symbols(&spans, source);
        assert_eq!(syms.len(), 3);
        assert_eq!((syms[0].line, syms[0].kind), (0, SymbolKind::Function));
        assert_eq!((syms[1].line, syms[1].kind), (1, SymbolKind::Type));
        // Namespace maps to the minimap's import hairline glyph.
        assert_eq!((syms[2].line, syms[2].kind), (2, SymbolKind::Import));
    }
}
