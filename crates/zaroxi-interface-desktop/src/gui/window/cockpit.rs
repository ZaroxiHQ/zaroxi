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
use zaroxi_interface_theme::ZaroxiTheme;
use zaroxi_interface_widgets::components::{DiffHunk, MinimapSymbol};
use zaroxi_interface_widgets::{
    AiPredictionGutter, CockpitTokens, CommandPalette, LivingDiffLayer, LspStatus, PaletteItem,
    PredictionCell, SemanticMinimap, StatusBar, WidgetTree,
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

/// Whether cockpit scene building is enabled (`ZAROXI_COCKPIT=1`).
pub fn cockpit_enabled() -> bool {
    matches!(std::env::var("ZAROXI_COCKPIT").as_deref(), Ok("1"))
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
        .new_leaf(Style { flex_grow: 1.0, min_size: Size { width: length(0.0), height: auto() }, ..Default::default() })
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
            pulse_line: inputs.prediction_cells.iter().find(|c| c.probability >= 0.8).map(|c| c.line),
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
}
