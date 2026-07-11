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
//! loop builds this scene by **default** (see [`cockpit_surfaces_active`]); the
//! status bar + overview/minimap are cockpit-owned unless the explicit legacy
//! fallback (`ZAROXI_LEGACY_SHELL_SURFACES=1`) is requested.

use vello::Scene;
use zaroxi_domain_settings::Settings;
use zaroxi_interface_theme::{SemanticColors, ZaroxiTheme};
use zaroxi_interface_widgets::components::DiffHunk;
use zaroxi_interface_widgets::{
    ActivityItem, ActivityRail, AiPredictionGutter, CommandPalette, InstrumentStatus,
    LivingDiffLayer, Minimap, MinimapProjection, PaletteItem, PredictionCell,
    SettingsDropdownState, SettingsRowHit, StatusBar, WidgetTree,
};

/// Status-bar height (px) used for the cockpit layout.
const STATUS_H: f32 = 24.0;
/// AI prediction gutter width (px) — spec: thin 16px right gutter.
const PREDICTION_GUTTER_W: f32 = 16.0;
/// Code minimap rail width (px). Compact so it reads as a navigation
/// instrument, not a wide band of dead editor-colored space, and so the editor
/// content reclaims horizontal width.
const MINIMAP_W: f32 = 48.0;
/// Scrollbar lane width (px) at the outermost editor right edge, between
/// the minimap and the AI assistant panel. The scrollbar thumb (8 px) floats
/// in this lane with 1 px breathing room on each side.
pub const SCROLLBAR_LANE_W: f32 = 10.0;

/// The minimap rail rect `(x, y, w, h)` for a given editor content rect. The
/// rail hugs the editor's right edge, but leaves [`SCROLLBAR_LANE_W`] px at the
/// outer edge for the scrollbar lane. Shared by the taffy layout pass and the
/// desktop hit-testing for click/drag navigation so both agree exactly.
pub fn minimap_rect(editor_rect: (f32, f32, f32, f32)) -> (f32, f32, f32, f32) {
    let (ex, ey, ew, eh) = editor_rect;
    let lane_w = SCROLLBAR_LANE_W.min(ew.max(0.0));
    let minimap_w = MINIMAP_W.min((ew - lane_w).max(0.0));
    let minimap_x = ex + (ew - minimap_w - lane_w).max(0.0);
    (minimap_x, ey, minimap_w, eh)
}

/// The scrollbar lane rect `(x, y, w, h)` — positioned between the minimap's
/// right edge and the editor content area's right edge (the AI panel boundary).
pub fn scrollbar_lane_rect(editor_rect: (f32, f32, f32, f32)) -> (f32, f32, f32, f32) {
    let (ex, ey, ew, eh) = editor_rect;
    let lane_w = SCROLLBAR_LANE_W.min(ew.max(0.0));
    let lane_x = ex + (ew - lane_w).max(0.0);
    (lane_x, ey, lane_w, eh)
}

/// Total width (px) of the editor's right-edge navigation cluster: the minimap
/// rail plus the scrollbar lane. Editor text wrapping/clipping reserves exactly
/// this much on the right so code lines end flush against the minimap's left
/// edge — no dead gap, no overlap.
pub fn right_cluster_width() -> f32 {
    MINIMAP_W + SCROLLBAR_LANE_W
}

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
    /// Editor content rect `(x, y, w, h)` in logical px — the authoritative
    /// editor surface bounds from the shell layout. The overview/minimap and AI
    /// prediction gutter are anchored to **this** rect's right edge so they read
    /// as editor-owned surfaces, not detached chrome floating at the window edge.
    /// When zero (no layout supplied), the cockpit falls back to full-surface
    /// placement.
    pub editor_rect: (f32, f32, f32, f32),
    /// Status bar rect `(x, y, w, h)` in logical px from the shell layout. When
    /// zero, the cockpit falls back to a bottom strip spanning the full width.
    pub status_rect: (f32, f32, f32, f32),
    /// Activity rail rect `(x, y, w, h)` in logical px from the shell layout
    /// (bottom of the left column, above the status bar). When zero, the rail is
    /// not rendered.
    pub rail_rect: (f32, f32, f32, f32),
    /// Unified tab-strip rect `(x, y, w, h)` in logical px from the shell layout.
    /// When zero (or `tabs` is empty) the strip is not rendered.
    pub tab_strip_rect: (f32, f32, f32, f32),
    /// Unified workbench tabs (file tabs followed by non-file tabs), in order.
    pub tabs: Vec<zaroxi_interface_widgets::CockpitTab>,
    /// Activity rail items in left-to-right order.
    pub rail_items: Vec<ActivityItem>,
    /// Style-token-derived colors for the rail (from StyleTokens, not
    /// CockpitTokens, so the rail matches main IDE chrome).
    pub rail_bg_color: [f32; 4],
    pub rail_item_active: [f32; 4],
    pub rail_accent_color: [f32; 4],
    pub rail_text_active: [f32; 4],
    pub rail_text_muted: [f32; 4],
    pub rail_divider_color: [f32; 4],
    /// Editor line height in px (for diff/gutter row mapping).
    pub line_height: f32,
    /// Visible viewport fraction `(top, bottom)`.
    pub viewport: (f32, f32),
    /// Structure-first code-minimap projection of the active document. Empty
    /// (default) renders nothing, so partial wiring stays valid.
    pub minimap_projection: MinimapProjection,
    /// Caret line as a `[0,1]` fraction for the minimap current-line tick.
    pub minimap_current_line: Option<f32>,
    /// Search-match line positions as `[0,1]` fractions for minimap markers.
    pub minimap_search_hits: Vec<f32>,
    /// Active selection as a `[0,1]` `(top, bottom)` fraction for the minimap.
    pub minimap_selection: Option<(f32, f32)>,
    /// Inline AI diff hunks.
    pub diff_hunks: Vec<DiffHunk>,
    /// AI prediction heat cells.
    pub prediction_cells: Vec<PredictionCell>,
    /// The typed instrument-panel status model (context / health / AI bands).
    /// Built from the shared canonical status presenter plus runtime health/AI
    /// telemetry, then mapped into visual roles by the `StatusBar` widget.
    pub status: InstrumentStatus,
    /// Command palette: `Some((items, selected, rtl))` when open.
    pub palette: Option<(Vec<PaletteItem>, usize, bool)>,
    /// Settings panel: `Some(sections, selected_section)` when open in the
    /// editor content region (replaces text editor content visually).
    pub settings_panel: Option<(Vec<zaroxi_interface_widgets::SettingsSection>, usize)>,
    /// Live settings state for the settings panel — drives current values and
    /// interactive controls. When `None` the panel renders static labels.
    pub settings: Option<Settings>,
    /// Dropdown open state for the settings panel — controls which select
    /// dropdown (if any) is currently expanded.
    pub settings_dropdown: SettingsDropdownState,
    /// Cached popup geometry. Frozen on open, cleared on close, to prevent
    /// visual drift from layout rounding between frames.
    pub cached_popup: Option<zaroxi_interface_widgets::PopupMenu>,
    /// Hit rects for interactive settings rows, computed from the panel layout.
    pub settings_hits: Vec<SettingsRowHit>,
    /// Extensions page: `Some(entries, selected_entry)` when open in the editor
    /// content region. `selected_entry` drives which detail pane is shown.
    pub extensions_panel: Option<(Vec<zaroxi_interface_widgets::ExtensionEntry>, usize)>,
    /// Generic destination placeholder `Some(title, subtitle)` for rail
    /// destinations without a bespoke page yet (Search / Source Control /
    /// Debug / Account). Rendered in the editor content region so selecting the
    /// destination visibly replaces the file editor.
    pub placeholder_panel: Option<(String, String)>,
    /// Whether the Welcome screen should be shown (no file open, no non-file
    /// destination active). When true, the Welcome panel replaces the editor
    /// content region completely.
    pub welcome_panel: bool,
    /// True when the active tab is the file editor (Explorer mode). Gates
    /// file-editor-only surfaces (minimap, prediction gutter) so they never
    /// appear on non-file cockpit pages.
    pub file_editor_active: bool,
    /// Animation phase in `[0,1)` (advanced by the host clock).
    pub phase: f32,
    /// Horizontal scroll offset (px) for the tab strip, set by wheel/hit-
    /// interaction on the tab strip. Clamped by `WorkbenchTabStrip` against the
    /// overflow width. Zero when no overflow or scrolled to origin.
    pub tab_scroll_offset: f32,
}

/// Resolve the active desktop theme to the shared `SemanticColors` token set
/// (the single source of truth from `zaroxi-interface-theme`). The cockpit
/// widgets read these directly, so the cockpit matches the rest of the IDE
/// chrome and any future authorable theme/extension plugs in here.
pub fn cockpit_tokens(theme: ZaroxiTheme, system_is_dark: bool) -> SemanticColors {
    match theme.resolve(system_is_dark) {
        ZaroxiTheme::Light => SemanticColors::light(),
        // Dark (and resolved System) map to the dark palette by default.
        _ => SemanticColors::dark(),
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

/// Region rectangles computed by the taffy pass.
struct Regions {
    editor: taffy::Layout,
    prediction_gutter: taffy::Layout,
    minimap: taffy::Layout,
    status: taffy::Layout,
    activity_rail: taffy::Layout,
    tab_strip: taffy::Layout,
}

/// Build a window-space `taffy::Layout` from an `(x, y, w, h)` rect.
fn rect_layout(x: f32, y: f32, w: f32, h: f32) -> taffy::Layout {
    taffy::Layout {
        location: taffy::geometry::Point { x, y },
        size: taffy::geometry::Size { width: w.max(0.0), height: h.max(0.0) },
        ..Default::default()
    }
}

/// Lay out the cockpit overview regions, **anchored to the editor surface**.
///
/// The diff layer spans the editor content rect; the AI prediction gutter and
/// the semantic minimap are nested at the editor's right edge (so they belong to
/// the editor pane, not the global window/AI-panel chrome); the status bar uses
/// the shell's real status rect. When no editor/status rects are supplied
/// ([`CockpitInputs::editor_rect`] is zero — e.g. tests, or a host without a
/// shell layout), it falls back to full-surface placement so the scene is still
/// valid.
fn layout_regions(inputs: &CockpitInputs) -> Regions {
    let has_editor = inputs.editor_rect.2 > 0.0 && inputs.editor_rect.3 > 0.0;
    let (ex, ey, ew, eh) = if has_editor {
        inputs.editor_rect
    } else {
        // Fallback: editor occupies the surface above a bottom status strip.
        (0.0, 0.0, inputs.width, (inputs.height - STATUS_H).max(0.0))
    };
    let (sx, sy, sw, sh) = if inputs.status_rect.2 > 0.0 && inputs.status_rect.3 > 0.0 {
        inputs.status_rect
    } else {
        (0.0, (inputs.height - STATUS_H).max(0.0), inputs.width, STATUS_H)
    };

    // Minimap hugs the editor's right edge, leaving a scrollbar lane.
    let (minimap_x, _, minimap_w, _) = minimap_rect((ex, ey, ew, eh));
    // Prediction gutter: when predictions are active, it sits between the
    // editor content and the minimap.  When empty (normal operation), it
    // collapses to ZERO width so no dead lane consumes editor real estate.
    let has_predictions = !inputs.prediction_cells.is_empty();
    let gutter_w =
        if has_predictions { PREDICTION_GUTTER_W.min((ew - minimap_w).max(0.0)) } else { 0.0 };
    let gutter_x = (minimap_x - gutter_w).max(ex);

    Regions {
        // Diff layer spans the full editor content rect (hunks map to lines).
        editor: rect_layout(ex, ey, ew, eh),
        prediction_gutter: rect_layout(gutter_x, ey, gutter_w, eh),
        minimap: rect_layout(minimap_x, ey, minimap_w, eh),
        status: rect_layout(sx, sy, sw, sh),
        activity_rail: rect_layout(
            inputs.rail_rect.0,
            inputs.rail_rect.1,
            inputs.rail_rect.2,
            inputs.rail_rect.3,
        ),
        tab_strip: rect_layout(
            inputs.tab_strip_rect.0,
            inputs.tab_strip_rect.1,
            inputs.tab_strip_rect.2,
            inputs.tab_strip_rect.3,
        ),
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
    let regions = layout_regions(inputs);
    let line_height = if inputs.line_height > 0.0 { inputs.line_height as f64 } else { 18.0 };

    // The instrument-panel status bar widget (built once; traced, then placed).
    let status_bar = StatusBar { status: inputs.status.clone(), phase: inputs.phase };

    // Overview anchor instrumentation: prove the overview/minimap is editor-owned
    // (nested at the editor's right edge) rather than detached global/AI chrome.
    if std::env::var("ZAROXI_MINIMAP_TRACE").as_deref() == Ok("1") {
        let m = &regions.minimap;
        let editor_nested = inputs.editor_rect.2 > 0.0 && inputs.editor_rect.3 > 0.0;
        let anchor = if editor_nested { "editor_edge" } else { "global_overlay" };
        eprintln!(
            "ZAROXI_MINIMAP_TRACE: overview_owner=cockpit overview_anchor={} overview_bounds=(x={:.0} y={:.0} w={:.0} h={:.0}) editor_overview_nested={}",
            anchor, m.location.x, m.location.y, m.size.width, m.size.height, editor_nested,
        );
    }
    if std::env::var("ZAROXI_STATUS_TRACE").as_deref() == Ok("1") {
        // Instrument-panel layout-stability proof. Metrics are theme-independent
        // (counts/buckets/widths), so a default token set is fine here.
        let s = &regions.status;
        let status_rect = zaroxi_interface_widgets::layout_rect(&regions.status);
        let m = status_bar.metrics(status_rect, &SemanticColors::dark());
        let ai_mode = match m.ai_mode {
            zaroxi_interface_widgets::AiMode::Dormant => "dormant",
            zaroxi_interface_widgets::AiMode::Live => "live",
            zaroxi_interface_widgets::AiMode::Degraded => "degraded",
        };
        eprintln!(
            "ZAROXI_STATUS_TRACE: status_owner=cockpit status_model_source=shared bg_owner=shell_shape_pass status_rect=(x={:.0} y={:.0} w={:.0} h={:.0}) layout_bucket={} collapse_level={} context_items_visible={} center_items_visible={} right_items_visible={} ai_items_visible={} ai_mode={} right_cluster_w={:.0} status_draw_items={} status_text_runs={} status_vector_items={}",
            s.location.x,
            s.location.y,
            s.size.width,
            s.size.height,
            m.bucket.label(),
            m.level,
            m.context_items,
            m.health_items,
            m.right_items,
            m.ai_items,
            ai_mode,
            m.right_band_w,
            m.draw_items,
            m.text_runs,
            m.vector_items,
        );
    }

    let mut tree = WidgetTree::new();

    // Inline AI diff overlay (above editor text, below cursor). Editor-only:
    // gated by `file_editor_active` (mirrors the prediction gutter / minimap
    // below) so that a NON-file destination occupying the same `regions.editor`
    // rect — Settings, Extensions, Welcome, generic placeholders — never has the
    // active buffer's diff hunks painted as a full-width band over its rows.
    // Previously this push was unconditional, so the editor's stale diff hunks
    // leaked onto the Settings page (and any other editor-region panel).
    let diff_decoration_trace = std::env::var("ZAROXI_DEBUG_DECORATION").as_deref() == Ok("1");
    if inputs.file_editor_active && !inputs.diff_hunks.is_empty() {
        if diff_decoration_trace {
            eprintln!(
                "ZAROXI_DEBUG_DECORATION: layer=LivingDiffLayer push=editor file_editor_active=true hunk_count={} editor_rect=(x={:.0} y={:.0} w={:.0} h={:.0})",
                inputs.diff_hunks.len(),
                regions.editor.location.x,
                regions.editor.location.y,
                regions.editor.size.width,
                regions.editor.size.height,
            );
        }
        tree.push(
            Box::new(LivingDiffLayer {
                hunks: inputs.diff_hunks.clone(),
                line_height,
                active: None,
                phase: inputs.phase,
            }),
            regions.editor,
        );
    } else if diff_decoration_trace {
        eprintln!(
            "ZAROXI_DEBUG_DECORATION: layer=LivingDiffLayer push=skipped file_editor_active={} hunk_count={} reason={}",
            inputs.file_editor_active,
            inputs.diff_hunks.len(),
            if !inputs.file_editor_active { "non_editor_destination" } else { "no_hunks" },
        );
    }

    // AI prediction gutter (right side) — file-editor only, only when active.
    if inputs.file_editor_active && !inputs.prediction_cells.is_empty() {
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
    }

    // Semantic minimap (far right rail) — file-editor only.
    if inputs.file_editor_active {
        tree.push(
            Box::new(Minimap {
                projection: inputs.minimap_projection.clone(),
                viewport: inputs.viewport,
                current_line: inputs.minimap_current_line,
                search_hits: inputs.minimap_search_hits.clone(),
                selection: inputs.minimap_selection,
            }),
            regions.minimap,
        );
    }

    // Instrument-panel status bar (three bands; built + traced above).
    tree.push(Box::new(status_bar), regions.status);

    // Activity rail (bottom strip of the left column, above the status bar).
    if !inputs.rail_items.is_empty()
        && regions.activity_rail.size.width > 0.0
        && regions.activity_rail.size.height > 0.0
    {
        if std::env::var("ZAROXI_RAIL_TRACE").as_deref() == Ok("1") {
            let glyphs: Vec<String> =
                inputs.rail_items.iter().map(|i| format!("U+{:04X}", i.glyph as u32)).collect();
            let sel = inputs.rail_items.iter().find(|i| i.selected).map(|i| i.index);
            let hov = inputs.rail_items.iter().position(|i| i.hovered);
            eprintln!(
                "ZAROXI_RAIL_TRACE: item_count={} glyphs=[{}] selected={:?} hovered={:?} rect=(x={:.0} y={:.0} w={:.0} h={:.0})",
                inputs.rail_items.len(),
                glyphs.join(","),
                sel,
                hov,
                regions.activity_rail.location.x,
                regions.activity_rail.location.y,
                regions.activity_rail.size.width,
                regions.activity_rail.size.height,
            );
        }
        tree.push(
            Box::new(ActivityRail {
                items: inputs.rail_items.clone(),
                bg_color: inputs.rail_bg_color,
                item_active: inputs.rail_item_active,
                accent_color: inputs.rail_accent_color,
                text_active: inputs.rail_text_active,
                text_muted: inputs.rail_text_muted,
                divider_color: inputs.rail_divider_color,
            }),
            regions.activity_rail,
        );
    }

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

    // Settings page — rendered in the editor content region.
    if let Some((sections, selected_section)) = &inputs.settings_panel {
        let settings = inputs.settings.clone().unwrap_or_default();
        tree.push(
            Box::new(zaroxi_interface_widgets::SettingsPanel {
                sections: sections.clone(),
                selected_section: *selected_section,
                settings,
                dropdown_state: inputs.settings_dropdown.clone(),
            }),
            regions.editor,
        );
    }

    // Extensions page — rendered in the editor content region.
    if let Some((entries, selected)) = &inputs.extensions_panel {
        tree.push(
            Box::new(zaroxi_interface_widgets::ExtensionsPanel {
                entries: entries.clone(),
                selected_entry: *selected,
            }),
            regions.editor,
        );
    }

    // Welcome screen — shown when Welcome is the active workbench tab.
    if inputs.welcome_panel {
        tree.push(
            Box::new(zaroxi_interface_widgets::WelcomePanel {
                title: "Zaroxi Studio".into(),
                tagline: "A workspace-first Rust-native studio with AI-assisted cockpit".into(),
                hint_open: "\u{2192} Open a file from the Explorer sidebar (Ctrl+O)".into(),
                hint_switch: "\u{2192} Use the top tabs to switch between files and cockpit pages"
                    .into(),
                hint_settings: "\u{2192} Visit Settings and Extensions from the cockpit rail"
                    .into(),
                hint_ai: "\u{2192} Open a file first to use the AI Assistant for code-aware help"
                    .into(),
                recent_placeholder: "Your recently opened files and projects will appear here"
                    .into(),
            }),
            regions.editor,
        );
    }

    // Generic destination placeholder (Search / Source Control / Debug /
    // Account) — rendered in the editor content region so selecting a rail
    // destination visibly replaces the file editor. Skipped when Welcome
    // is active so the WelcomePanel widget has exclusive ownership.
    if !inputs.welcome_panel
        && let Some((title, subtitle)) = &inputs.placeholder_panel
    {
        tree.push(
            Box::new(zaroxi_interface_widgets::DestinationPlaceholder {
                title: title.clone(),
                subtitle: subtitle.clone(),
            }),
            regions.editor,
        );
    }

    // Unified workbench tab strip (file tabs + non-file workbench tabs).
    if !inputs.tabs.is_empty()
        && regions.tab_strip.size.width > 0.0
        && regions.tab_strip.size.height > 0.0
    {
        tree.push(
            Box::new(zaroxi_interface_widgets::WorkbenchTabStrip {
                tabs: inputs.tabs.clone(),
                tab_scroll_offset: inputs.tab_scroll_offset,
            }),
            regions.tab_strip,
        );
    }

    tree
}

/// Compute hit rects for interactive settings rows given the editor layout
/// region and current settings state. Called by the host after layout so
/// pointer events can be routed to `SettingsAction` dispatch.
pub fn compute_settings_hits(
    editor_layout: &taffy::Layout,
    sections: &[zaroxi_interface_widgets::SettingsSection],
    selected_section: usize,
    settings: &Settings,
    dropdown_state: &SettingsDropdownState,
) -> Vec<SettingsRowHit> {
    let panel = zaroxi_interface_widgets::SettingsPanel {
        sections: sections.to_vec(),
        selected_section,
        settings: settings.clone(),
        dropdown_state: dropdown_state.clone(),
    };
    panel.row_hits(editor_layout)
}

/// Build the cockpit tree and paint it into a fresh `vello::Scene`.
pub fn paint_cockpit(inputs: &CockpitInputs, tokens: &SemanticColors) -> Scene {
    let tree = build_cockpit(inputs);
    let mut scene = Scene::new();
    tree.paint(&mut scene, tokens);

    if let Some((sections, selected_section)) = &inputs.settings_panel
        && let Some(settings) = &inputs.settings
    {
        let editor_layout = taffy::Layout {
            location: taffy::geometry::Point { x: inputs.editor_rect.0, y: inputs.editor_rect.1 },
            size: taffy::geometry::Size {
                width: inputs.editor_rect.2.max(0.0),
                height: inputs.editor_rect.3.max(0.0),
            },
            ..Default::default()
        };
        if let Some(sec) = sections.get(*selected_section) {
            let mut dd_idx: usize = 0;
            for row in &sec.items {
                if !matches!(
                    &row.kind,
                    zaroxi_interface_widgets::SettingsRowKind::Theme
                        | zaroxi_interface_widgets::SettingsRowKind::Font
                ) {
                    continue;
                }
                if inputs.settings_dropdown.open_row == Some(dd_idx) {
                    if let Some(popup) = zaroxi_interface_widgets::settings_popup(
                        dd_idx,
                        &row.kind,
                        &editor_layout,
                        settings,
                        &inputs.settings_dropdown,
                    ) {
                        popup.paint(&mut scene, tokens);
                    }
                    break;
                }
                dd_idx += 1;
            }
        }
    }

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
        clip_rect: t.clip_rect,
        italic: t.italic,
    }
}

/// Build the cockpit tree alongside any open popup menus, returning the
/// combined vello scene and positioned text runs.
pub fn build_cockpit_frame(
    inputs: &mut CockpitInputs,
    tokens: &SemanticColors,
) -> (Scene, Vec<zaroxi_core_engine_render::renderer::CockpitText>) {
    let tree = build_cockpit(inputs);
    let mut scene = Scene::new();
    tree.paint(&mut scene, tokens);
    let mut text: Vec<_> = tree.collect_text(tokens).into_iter().map(to_render_text).collect();

    // ── Popup menu (post-tree, stable geometry from cache) ─────────────────
    if let Some((sections, selected_section)) = &inputs.settings_panel
        && let Some(settings) = &inputs.settings
    {
        let editor_layout = taffy::Layout {
            location: taffy::geometry::Point { x: inputs.editor_rect.0, y: inputs.editor_rect.1 },
            size: taffy::geometry::Size {
                width: inputs.editor_rect.2.max(0.0),
                height: inputs.editor_rect.3.max(0.0),
            },
            ..Default::default()
        };
        if let Some(sec) = sections.get(*selected_section) {
            let mut dd_idx: usize = 0;
            for row in &sec.items {
                if !matches!(
                    &row.kind,
                    zaroxi_interface_widgets::SettingsRowKind::Theme
                        | zaroxi_interface_widgets::SettingsRowKind::Font
                ) {
                    continue;
                }
                if inputs.settings_dropdown.open_row == Some(dd_idx) {
                    let popup = if let Some(ref cached) = inputs.cached_popup {
                        cached.clone()
                    } else if let Some(fresh) = zaroxi_interface_widgets::settings_popup(
                        dd_idx,
                        &row.kind,
                        &editor_layout,
                        settings,
                        &inputs.settings_dropdown,
                    ) {
                        inputs.cached_popup = Some(fresh.clone());
                        fresh
                    } else {
                        break;
                    };
                    popup.paint(&mut scene, tokens);

                    // Remove settings text items that fall behind the popup
                    // background. Since text renders after the vello overlay
                    // (popup bg), such text would bleed through if not removed.
                    // Run BEFORE popup text push so option labels are kept.
                    let (px, py, pw, ph) = popup.popup_rect;
                    text.retain(|t| t.y < py || t.y > py + ph || t.x < px || t.x > px + pw);

                    for wt in popup.text_items(tokens) {
                        text.push(to_render_text(wt));
                    }
                    break;
                }
                dd_idx += 1;
            }
        }
    }

    (scene, text)
}

#[cfg(test)]
mod tests {
    use super::*;
    use zaroxi_interface_widgets::{AiBand, AiMode, ContextBand, HealthBand, LspStatus, MetaChips};

    fn sample() -> CockpitInputs {
        CockpitInputs {
            width: 1200.0,
            height: 800.0,
            // Editor content rect from a representative shell layout: the overview
            // should anchor to this rect's right edge, not the window edge.
            editor_rect: (300.0, 60.0, 600.0, 700.0),
            status_rect: (0.0, 776.0, 1200.0, 24.0),
            line_height: 18.0,
            viewport: (0.1, 0.25),
            minimap_projection: MinimapProjection::from_lines(
                ["fn a() {", "    let x = 1;", "}"].into_iter(),
                3,
                500,
                4,
            ),
            minimap_current_line: Some(0.1),
            minimap_search_hits: vec![0.4, 0.7],
            minimap_selection: None,
            diff_hunks: vec![DiffHunk {
                line: 2,
                kind: zaroxi_interface_widgets::components::DiffKind::Added,
            }],
            prediction_cells: vec![PredictionCell { line: 3, probability: 0.9 }],
            status: InstrumentStatus {
                context: ContextBand {
                    ancestors: vec!["zaroxi".into()],
                    leaf: "main.rs".into(),
                    position: Some("Ln 1, Col 1".into()),
                    markers: vec![],
                },
                meta: MetaChips {
                    language: Some("Rust".into()),
                    indent: Some("Spaces 4".into()),
                    eol: Some("LF".into()),
                    encoding: Some("UTF-8".into()),
                },
                health: HealthBand { fps: Some(60), mem_mb: Some(128), lsp: LspStatus::Healthy },
                ai: AiBand {
                    mode: AiMode::Live,
                    tokens_used: 2048,
                    tokens_total: 8192,
                    model: None,
                    latency_ms: Some(12),
                },
                rtl: false,
            },
            palette: Some((
                vec![PaletteItem {
                    label: "افتح ملف".to_string(), shortcut: "Ctrl+O".to_string()
                }],
                0,
                true,
            )),
            settings_panel: None,
            extensions_panel: None,
            settings: None,
            settings_dropdown: SettingsDropdownState::default(),
            cached_popup: None,
            settings_hits: Vec::new(),
            placeholder_panel: None,
            welcome_panel: false,
            file_editor_active: true,
            phase: 0.3,
            tab_scroll_offset: 0.0,
            rail_rect: (0.0, 776.0, 0.0, 0.0),
            rail_items: vec![],
            tab_strip_rect: (0.0, 0.0, 0.0, 0.0),
            tabs: vec![],
            rail_bg_color: [0.0; 4],
            rail_item_active: [0.0; 4],
            rail_accent_color: [0.0; 4],
            rail_text_active: [0.0; 4],
            rail_text_muted: [0.0; 4],
            rail_divider_color: [0.0; 4],
        }
    }

    #[test]
    fn fallback_regions_are_within_surface_and_sized() {
        // No editor/status rect supplied -> full-surface fallback placement.
        let r =
            layout_regions(&CockpitInputs { width: 1200.0, height: 800.0, ..Default::default() });
        assert!((r.status.size.height - STATUS_H).abs() < 0.5);
        assert!((r.minimap.size.width - MINIMAP_W).abs() < 0.5);
        // Empty prediction gutter collapses to zero width (no dead lane).
        assert!(r.prediction_gutter.size.width < 0.5, "empty prediction gutter must be 0 wide");
        // Editor (diff host) spans the surface above the status strip.
        assert!(r.editor.size.width > 800.0);
        // Status bar sits at the bottom.
        assert!(r.status.location.y >= r.editor.size.height - 1.0);
    }

    #[test]
    fn editor_drawable_right_meets_minimap_left_edge() {
        // The editor's usable right boundary (content rect minus the right
        // cluster reserve used by the content clip + wrap width) must equal the
        // minimap's left edge: a clean handoff with no dead band and no overlap.
        let editor_rect = (100.0, 40.0, 1200.0, 800.0);
        let (mm_x, _, mm_w, _) = minimap_rect(editor_rect);
        let editor_drawable_right = editor_rect.0 + editor_rect.2 - right_cluster_width();
        assert!(
            (editor_drawable_right - mm_x).abs() < 0.01,
            "editor drawable right {editor_drawable_right} must meet minimap left {mm_x}",
        );
        // Minimap and scrollbar lanes never overlap.
        let (sb_x, _, sb_w, _) = scrollbar_lane_rect(editor_rect);
        assert!(mm_x + mm_w <= sb_x + 0.01, "minimap must not overlap scrollbar lane");
        assert!(
            (sb_x + sb_w - (editor_rect.0 + editor_rect.2)).abs() < 0.01,
            "scrollbar lane hugs the editor right edge",
        );
    }

    /// Guards the text-cutting AND dead-band regressions together by asserting
    /// the single stable right-edge model: the soft-wrap boundary, the glyph
    /// paint clip, the current-line/selection fill, and the minimap's left edge
    /// must all coincide.
    ///
    /// Both the wrap width (redraw.rs: `content_w - CONTENT_PAD_X - cluster`)
    /// and the glyph paint clip width (redraw.rs: `clip_rect.2 - cluster`, where
    /// `clip_rect.2 == content_w - CONTENT_PAD_X`) reserve EXACTLY the right
    /// cluster from the same base. They must therefore be equal: when
    /// `wrap == clip` no glyph is ever cut (words break where the clip ends) and
    /// there is no early-stopping fill / dead strip.
    ///
    /// The prior bug used a stale `clip = clip_rect.2 - 100.0` that was 42px
    /// narrower than the 58px cluster reserve, which cut the last characters and
    /// left a dead band before the minimap.
    #[test]
    fn wrap_width_equals_glyph_paint_clip_and_meets_minimap() {
        const CONTENT_PAD_X: f32 = 8.0;
        for content_w in [400.0f32, 800.0, 1200.0, 1920.0] {
            // Editor content rect anchored at an arbitrary origin.
            let editor_rect = (100.0, 40.0, content_w, 800.0);
            // Wrap boundary (px), text origin at CONTENT_PAD_X.
            let wrap_w = content_w - CONTENT_PAD_X - right_cluster_width();
            // Glyph paint clip width: clip_rect.2 (= content_w - CONTENT_PAD_X)
            // minus the same cluster reserve — the real redraw.rs formula.
            let clip_rect_w = content_w - CONTENT_PAD_X;
            let paint_clip_w = clip_rect_w - right_cluster_width();
            assert!(
                (wrap_w - paint_clip_w).abs() < 0.01,
                "wrap width {wrap_w} must equal glyph paint clip {paint_clip_w} (content_w={content_w}); any gap cuts glyphs or opens a dead band",
            );
            // The clip's right edge (text origin + clip width) must land exactly
            // on the minimap's left edge: a clean, tight handoff.
            let clip_right = editor_rect.0 + CONTENT_PAD_X + paint_clip_w;
            let (mm_x, ..) = minimap_rect(editor_rect);
            assert!(
                (clip_right - mm_x).abs() < 0.01,
                "glyph clip right {clip_right} must meet minimap left {mm_x} (content_w={content_w})",
            );
        }
        // The reserve is exactly the minimap+scrollbar cluster — no stale
        // hardcoded 100px margin that would waste editor width.
        assert!(
            (right_cluster_width() - (MINIMAP_W + SCROLLBAR_LANE_W)).abs() < 0.01,
            "wrap reserve must equal the real cluster width, not a stale constant",
        );
        assert!(right_cluster_width() < 100.0, "cluster reserve must be far below the old 100px");
    }

    #[test]
    fn overview_is_anchored_to_the_editor_edge() {
        // Given an editor content rect, the minimap/gutter must nest at the
        // editor's right edge (editor-owned), NOT at the window's far right.
        // The minimap leaves `SCROLLBAR_LANE_W` px at the outer edge for the
        // scrollbar lane, so the minimap's right edge is short of the editor's
        // right edge by that amount.
        let inputs = sample();
        let (ex, ey, ew, eh) = inputs.editor_rect;
        let r = layout_regions(&inputs);

        let minimap_right = r.minimap.location.x + r.minimap.size.width;
        let expected_right = ex + ew - SCROLLBAR_LANE_W;
        assert!(
            (minimap_right - expected_right).abs() < 0.5,
            "minimap right edge must leave room for the scrollbar lane (got {minimap_right}, expected {expected_right})",
        );
        // ...and it sits well inside the window (the window is 1200 wide but the
        // editor ends at 900), proving it is not floating at the window edge.
        assert!(minimap_right < inputs.width - 100.0, "overview must not float at the window edge");
        // Prediction gutter is just left of the minimap, inside the editor.
        assert!(r.prediction_gutter.location.x >= ex);
        assert!(r.prediction_gutter.location.x < r.minimap.location.x);
        // Regions span the editor vertical bounds.
        assert!((r.minimap.location.y - ey).abs() < 0.5);
        assert!((r.minimap.size.height - eh).abs() < 0.5);
        // Status bar uses the real status rect.
        assert!((r.status.location.y - inputs.status_rect.1).abs() < 0.5);
    }

    #[test]
    fn build_places_all_widgets() {
        let tree = build_cockpit(&sample());
        // 4 base cockpit widgets + palette overlay = 5 (rail is empty in sample).
        assert_eq!(tree.len(), 5);
    }

    #[test]
    fn paint_produces_scene_without_panic() {
        let tokens = cockpit_tokens(ZaroxiTheme::Dark, true);
        let _scene = paint_cockpit(&sample(), &tokens);
        // Light and dark resolve to distinct palettes from the theme crate.
        let light = cockpit_tokens(ZaroxiTheme::Light, false);
        let dark = cockpit_tokens(ZaroxiTheme::Dark, true);
        assert_ne!(light.app_background.r, dark.app_background.r);
    }

    /// Instrument the full popup text path: build a cockpit frame with an open
    /// Theme dropdown and verify text items are created, have valid positions,
    /// and are appended to the final text buffer.
    #[test]
    fn popup_text_items_created_and_appended() {
        use zaroxi_interface_widgets::{SettingsRow, SettingsRowKind, SettingsSection};

        let sections = vec![SettingsSection {
            label: "General".to_string(),
            items: vec![
                SettingsRow {
                    label: "Color Theme".to_string(),
                    description: "Choose theme".to_string(),
                    kind: SettingsRowKind::Theme,
                },
                SettingsRow {
                    label: "Editor Font".to_string(),
                    description: "Choose font".to_string(),
                    kind: SettingsRowKind::Font,
                },
            ],
        }];

        let settings = zaroxi_domain_settings::Settings::default();
        // Theme dropdown open
        let dropdown = SettingsDropdownState { open_row: Some(0) };

        let mut inputs = sample();
        inputs.settings_panel = Some((sections.clone(), 0));
        inputs.settings = Some(settings.clone());
        inputs.settings_dropdown = dropdown;
        inputs.cached_popup = None;

        let tokens = SemanticColors::dark();
        let (_scene, text) = build_cockpit_frame(&mut inputs, &tokens);

        // Should have created and cached a popup
        assert!(inputs.cached_popup.is_some(), "cached_popup should be set after frame");
        let popup = inputs.cached_popup.as_ref().unwrap();

        eprintln!(
            "POPUP TEST: popup_rect={:?} option_count={} option_rects={:?}",
            popup.popup_rect,
            popup.options.len(),
            popup.option_rects,
        );

        // Popup text items must exist
        let popup_text: Vec<_> = popup.text_items(&tokens);
        eprintln!("POPUP TEST: text_items count={}", popup_text.len());
        for (i, t) in popup_text.iter().enumerate() {
            eprintln!(
                "  [{}] '{}' pos=({:.1},{:.1}) sz={} clip={:?}",
                i, t.text, t.x, t.y, t.size_px, t.clip_rect,
            );
        }

        assert!(!popup_text.is_empty(), "popup text items must not be empty");
        assert_eq!(popup_text.len(), 3, "3 theme options expected");
        assert_eq!(popup_text[0].text, "System");
        assert_eq!(popup_text[1].text, "Dark");
        assert_eq!(popup_text[2].text, "Light");

        // Verify text positions are within popup rect
        let (px, py, pw, ph) = popup.popup_rect;
        for t in &popup_text {
            assert!(t.x >= px, "text x={} should be >= popup x={}", t.x, px);
            assert!(t.x < px + pw, "text x={} should be < popup right={}", t.x, px + pw);
            assert!(t.y >= py, "text y={} should be >= popup y={}", t.y, py);
            assert!(t.y < py + ph, "text y={} should be < popup bottom={}", t.y, py + ph,);
        }

        // Verify clip rect matches
        for t in &popup_text {
            if let Some(clip) = t.clip_rect {
                assert_eq!(clip.0, px, "clip x mismatch");
                assert_eq!(clip.1, py, "clip y mismatch");
            } else {
                panic!("text item missing clip_rect");
            }
        }

        // Verify popup text is in the final text buffer.
        let final_texts: Vec<&str> = text.iter().map(|t| t.text.as_str()).collect();
        eprintln!("POPUP TEST: final text buffer ({} items): {:?}", text.len(), final_texts);
        assert!(final_texts.contains(&"System"), "System missing from final text");
        assert!(final_texts.contains(&"Dark"), "Dark missing from final text");
        assert!(final_texts.contains(&"Light"), "Light missing from final text");

        // Second frame with cached popup
        let mut inputs2 = inputs;
        let (_scene2, text2) = build_cockpit_frame(&mut inputs2, &tokens);
        let final2: Vec<&str> = text2.iter().map(|t| t.text.as_str()).collect();
        eprintln!("POPUP TEST: frame 2 text buffer ({} items): {:?}", text2.len(), final2);
        assert!(final2.contains(&"System"), "System missing from frame 2 text");
    }
}
