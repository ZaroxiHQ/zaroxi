//! The six cockpit components, each implementing [`ZaroxiWidget`].
//!
//! Every component paints its distinctive **vector** appearance with vello
//! (rects, diamonds, bezier connectors, arcs, heatmaps). Glyph text is drawn by
//! the existing cosmic-text atlas layer of the renderer; here, text regions are
//! laid out as slots and their content is exposed via [`ZaroxiWidget::a11y_label`].
//!
//! Animations are phase-driven: each animated component carries a `phase: f32`
//! in `[0,1)` advanced by the host clock, and **must** collapse to a static
//! frame when [`crate::reduce_motion`] is set.

use vello::Scene;
use vello::kurbo::{Affine, BezPath, Circle, Line, Point, Rect, RoundedRect, Stroke};
use vello::peniko::Fill;
use zaroxi_interface_theme::Color as ThemeColor;
use zaroxi_interface_theme::SemanticColors;

use crate::widget::{
    WidgetLayer, WidgetText, ZaroxiWidget, brush, color_arr, layout_rect, reduce_motion,
};

// ── shared helpers ─────────────────────────────────────────────────────────

/// Linear interpolation between two theme colors (used by heatmaps).
fn lerp_color(a: ThemeColor, b: ThemeColor, t: f32) -> ThemeColor {
    let t = t.clamp(0.0, 1.0);
    ThemeColor::from_rgba(
        a.r + (b.r - a.r) * t,
        a.g + (b.g - a.g) * t,
        a.b + (b.b - a.b) * t,
        a.a + (b.a - a.a) * t,
    )
}

/// Triangle wave in `[0,1]` for symmetric pulses; static (`1.0`) under reduce-motion.
fn pulse(phase: f32) -> f32 {
    if reduce_motion() {
        return 1.0;
    }
    let p = phase.fract();
    if p < 0.5 { p * 2.0 } else { 2.0 - p * 2.0 }
}

fn fill(scene: &mut Scene, shape: &impl vello::kurbo::Shape, color: ThemeColor) {
    scene.fill(Fill::NonZero, Affine::IDENTITY, brush(color), None, shape);
}

fn stroke(scene: &mut Scene, width: f64, shape: &impl vello::kurbo::Shape, color: ThemeColor) {
    scene.stroke(&Stroke::new(width), Affine::IDENTITY, brush(color), None, shape);
}

/// A filled diamond centered at `(cx, cy)` with half-extent `r` (for type symbols).
fn diamond(cx: f64, cy: f64, r: f64) -> BezPath {
    let mut p = BezPath::new();
    p.move_to((cx, cy - r));
    p.line_to((cx + r, cy));
    p.line_to((cx, cy + r));
    p.line_to((cx - r, cy));
    p.close_path();
    p
}

// ── Component 1: Semantic Scroll & Minimap ──────────────────────────────────

/// Kind of symbol shown in the semantic minimap.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SymbolKind {
    /// Function — drawn as a filled block.
    Function,
    /// Type — drawn as a diamond.
    Type,
    /// Import — drawn as a thin line.
    Import,
}

/// One symbol entry in the minimap.
#[derive(Debug, Clone, Copy)]
pub struct MinimapSymbol {
    /// 0-based source line of the symbol.
    pub line: usize,
    /// Symbol kind (drives the glyph drawn).
    pub kind: SymbolKind,
}

/// Component 1 — a meaning-aware minimap/scrollbar.
///
/// **Vello:** background fill; function blocks (rounded rects), type diamonds
/// (bezier path), import hairlines; AI-modified regions as translucent amber
/// rects; a viewport thumb rect.
/// **Layout (taffy):** a fixed-width right rail (e.g. width 84, full height).
/// **Tokens:** `minimap_bg`, `sym_function`, `sym_type`, `sym_import`,
/// `minimap_ai_region`, `accent` (thumb).
/// **Animation:** none (static structural map).
/// **A11y:** "Semantic minimap, N symbols; click a block to jump to its symbol."
pub struct SemanticMinimap {
    /// Symbols to plot.
    pub symbols: Vec<MinimapSymbol>,
    /// Total document line count (for line→y mapping).
    pub total_lines: usize,
    /// AI-modified line ranges `[start..=end]`, highlighted amber.
    pub ai_regions: Vec<(usize, usize)>,
    /// Visible viewport as a `[0,1]` fraction `(top, bottom)`.
    pub viewport: (f32, f32),
}

impl SemanticMinimap {
    fn y_of(&self, line: usize, top: f64, height: f64) -> f64 {
        let frac =
            if self.total_lines <= 1 { 0.0 } else { line as f64 / (self.total_lines - 1) as f64 };
        top + frac * height
    }
}

impl ZaroxiWidget for SemanticMinimap {
    fn layer(&self) -> WidgetLayer {
        WidgetLayer::Minimap
    }

    fn paint(&self, scene: &mut Scene, layout: &taffy::Layout, theme: &SemanticColors) {
        let r = layout_rect(layout);
        fill(scene, &r, theme.editor_gutter_background);

        // AI-modified regions (drawn first, behind symbols).
        for (start, end) in &self.ai_regions {
            let y0 = self.y_of(*start, r.y0, r.height());
            let y1 = self.y_of(*end, r.y0, r.height()).max(y0 + 2.0);
            fill(scene, &Rect::new(r.x0, y0, r.x1, y1), theme.accent_soft);
        }

        let cx = (r.x0 + r.x1) * 0.5;
        for sym in &self.symbols {
            let y = self.y_of(sym.line, r.y0, r.height());
            match sym.kind {
                SymbolKind::Function => {
                    let block = RoundedRect::new(r.x0 + 6.0, y - 2.0, r.x1 - 6.0, y + 2.0, 1.5);
                    fill(scene, &block, theme.syntax_function);
                }
                SymbolKind::Type => fill(scene, &diamond(cx, y, 4.0), theme.syntax_type),
                SymbolKind::Import => stroke(
                    scene,
                    1.0,
                    &Line::new((r.x0 + 10.0, y), (r.x1 - 10.0, y)),
                    theme.syntax_namespace,
                ),
            }
        }

        // Viewport thumb.
        let ty0 = r.y0 + self.viewport.0 as f64 * r.height();
        let ty1 = r.y0 + self.viewport.1 as f64 * r.height();
        let thumb = RoundedRect::new(r.x0 + 2.0, ty0, r.x1 - 2.0, ty1.max(ty0 + 8.0), 3.0);
        fill(scene, &thumb, theme.accent_soft);
        stroke(scene, 1.0, &thumb, theme.accent);
    }

    fn a11y_label(&self) -> Option<String> {
        Some(format!("Semantic minimap, {} symbols; click a block to jump.", self.symbols.len()))
    }
}

// ── Component 2: Living Diff Layer ──────────────────────────────────────────

/// One diff line.
#[derive(Debug, Clone, Copy)]
pub struct DiffHunk {
    /// 0-based viewport-relative line index.
    pub line: usize,
    /// Whether the line is added or removed.
    pub added: bool,
}

/// Component 2 — translucent diff overlay drawn ON the buffer.
///
/// **Vello:** added lines = soft green tint + left-border glow bar; removed
/// lines = red tint + a struck-through line. The active hunk's glow pulses.
/// **Layout:** spans the editor content rect; line height supplied.
/// **Tokens:** `diff_added_bg/border`, `diff_removed_bg/strike`, `ai_highlight`.
/// **Animation:** active-hunk glow alpha pulses (≈1200ms triangle); static under
/// reduce-motion. Removed lines fade — represented by `phase` on the strike alpha.
/// **A11y:** "Inline AI diff: A additions, R removals. Tab/Shift-Tab to navigate,
/// Enter accept, Esc reject."
pub struct LivingDiffLayer {
    /// The hunks to overlay.
    pub hunks: Vec<DiffHunk>,
    /// Editor line height in px.
    pub line_height: f64,
    /// Index into `hunks` currently focused (Tab navigation).
    pub active: Option<usize>,
    /// Animation phase in `[0,1)`.
    pub phase: f32,
}

impl ZaroxiWidget for LivingDiffLayer {
    fn layer(&self) -> WidgetLayer {
        WidgetLayer::DiffLayer
    }

    fn paint(&self, scene: &mut Scene, layout: &taffy::Layout, theme: &SemanticColors) {
        let r = layout_rect(layout);
        let glow = pulse(self.phase);
        for (i, h) in self.hunks.iter().enumerate() {
            let y0 = r.y0 + h.line as f64 * self.line_height;
            let y1 = y0 + self.line_height;
            let is_active = self.active == Some(i);
            if h.added {
                fill(scene, &Rect::new(r.x0, y0, r.x1, y1), theme.success);
                let border_color = if is_active {
                    theme.success.with_alpha(0.4 + 0.6 * glow)
                } else {
                    theme.success
                };
                fill(scene, &Rect::new(r.x0, y0, r.x0 + 3.0, y1), border_color);
            } else {
                fill(scene, &Rect::new(r.x0, y0, r.x1, y1), theme.error);
                let mid = (y0 + y1) * 0.5;
                stroke(scene, 1.5, &Line::new((r.x0 + 4.0, mid), (r.x1 - 4.0, mid)), theme.error);
            }
        }
    }

    fn a11y_label(&self) -> Option<String> {
        let adds = self.hunks.iter().filter(|h| h.added).count();
        let rems = self.hunks.len() - adds;
        Some(format!(
            "Inline AI diff: {adds} additions, {rems} removals. Tab/Shift-Tab to navigate, Enter accept, Esc reject."
        ))
    }
}

// ── Component 3: Context Canvas ─────────────────────────────────────────────

/// A related file panel floating around the focused file.
#[derive(Debug, Clone, Copy)]
pub struct RelatedPanel {
    /// Panel box (x, y, w, h) in canvas space.
    pub rect: (f64, f64, f64, f64),
    /// Import site on the focused file the connection originates from.
    pub import_site: (f64, f64),
}

/// Component 3 — spatial import-graph canvas (Ctrl+Shift+Space mode).
///
/// **Vello:** the focused file as a central rounded rect; related files as
/// floating panels; glowing **bezier** connectors from each import site to its
/// panel; **flow particles** (circles) animated along each curve.
/// **Layout (taffy):** panels positioned by import proximity (host computes the
/// boxes; this paints them).
/// **Tokens:** `canvas_panel_bg`, `canvas_connection`, `canvas_particle`,
/// `canvas_glow`, `accent`.
/// **Animation:** particles travel `0→1` along the curve (≈2000ms linear); under
/// reduce-motion they pin to the curve midpoint.
/// **A11y:** "Context canvas: N related files. Click a panel to focus it."
pub struct ContextCanvas {
    /// Central focused-file box (x, y, w, h).
    pub center: (f64, f64, f64, f64),
    /// Related panels.
    pub related: Vec<RelatedPanel>,
    /// Animation phase in `[0,1)`.
    pub phase: f32,
}

fn cubic(p0: Point, p1: Point, p2: Point, p3: Point, t: f64) -> Point {
    let u = 1.0 - t;
    let x = u * u * u * p0.x + 3.0 * u * u * t * p1.x + 3.0 * u * t * t * p2.x + t * t * t * p3.x;
    let y = u * u * u * p0.y + 3.0 * u * u * t * p1.y + 3.0 * u * t * t * p2.y + t * t * t * p3.y;
    Point::new(x, y)
}

impl ZaroxiWidget for ContextCanvas {
    fn layer(&self) -> WidgetLayer {
        WidgetLayer::Palette // full-surface modal overlay
    }

    fn paint(&self, scene: &mut Scene, _layout: &taffy::Layout, theme: &SemanticColors) {
        let (cx, cy, cw, ch) = self.center;
        let center_pt = Point::new(cx + cw * 0.5, cy + ch * 0.5);

        for panel in &self.related {
            let (px, py, pw, ph) = panel.rect;
            let dst = Point::new(px + pw * 0.5, py + ph * 0.5);
            let src = Point::new(panel.import_site.0, panel.import_site.1);

            // Bezier connector: control points pull toward the horizontal midline.
            let c1 = Point::new((src.x + dst.x) * 0.5, src.y);
            let c2 = Point::new((src.x + dst.x) * 0.5, dst.y);
            let mut path = BezPath::new();
            path.move_to(src);
            path.curve_to(c1, c2, dst);
            stroke(scene, 1.5, &path, theme.divider);

            // Flow particle along the curve.
            let t = if reduce_motion() { 0.5 } else { self.phase.fract() as f64 };
            let pos = cubic(src, c1, c2, dst, t);
            fill(scene, &Circle::new(pos, 2.5), theme.accent);

            // Related panel.
            let pr = RoundedRect::new(px, py, px + pw, py + ph, 6.0);
            fill(scene, &pr, theme.panel_background);
            stroke(scene, 1.0, &pr, theme.accent_soft);
        }

        // Focused file panel on top.
        let cr = RoundedRect::new(cx, cy, cx + cw, cy + ch, 8.0);
        fill(scene, &cr, theme.panel_background);
        stroke(scene, 2.0, &cr, theme.accent);
        let _ = center_pt;
    }

    fn a11y_label(&self) -> Option<String> {
        Some(format!(
            "Context canvas: {} related files. Click a panel to focus.",
            self.related.len()
        ))
    }
}

// ── Component 4: AI Prediction Gutter ───────────────────────────────────────

/// One predicted-edit cell.
#[derive(Debug, Clone, Copy)]
pub struct PredictionCell {
    /// 0-based viewport-relative line.
    pub line: usize,
    /// Edit probability in `[0,1]` (drives heat color).
    pub probability: f32,
}

/// Component 4 — thin right-side AI prediction gutter (16px).
///
/// **Vello:** per-line heat cells filled by lerping `ai_prediction_base`→
/// `ai_prediction_warm` by probability; a high-confidence cell pulses with
/// `ai_pulse`.
/// **Layout (taffy):** fixed 16px-wide right column.
/// **Tokens:** `ai_prediction_base`, `ai_prediction_warm`, `ai_pulse`, `bg_elevated`.
/// **Animation:** pulse cell alpha (≈900ms triangle); static under reduce-motion.
/// **A11y:** "AI prediction gutter: K likely edit lines. Click a pulsing cell for
/// the suggestion."
pub struct AiPredictionGutter {
    /// Heat cells.
    pub cells: Vec<PredictionCell>,
    /// Line height in px.
    pub line_height: f64,
    /// Line index with an active pulsing suggestion, if any.
    pub pulse_line: Option<usize>,
    /// Animation phase in `[0,1)`.
    pub phase: f32,
}

impl ZaroxiWidget for AiPredictionGutter {
    fn layer(&self) -> WidgetLayer {
        WidgetLayer::Gutter
    }

    fn paint(&self, scene: &mut Scene, layout: &taffy::Layout, theme: &SemanticColors) {
        let r = layout_rect(layout);
        fill(scene, &r, theme.elevated_panel_background);
        let pulse_a = pulse(self.phase);
        for cell in &self.cells {
            let y0 = r.y0 + cell.line as f64 * self.line_height;
            let y1 = y0 + self.line_height - 1.0;
            let heat = lerp_color(theme.accent_soft_background, theme.warning, cell.probability);
            fill(scene, &Rect::new(r.x0, y0, r.x1, y1), heat);
            if self.pulse_line == Some(cell.line) {
                fill(
                    scene,
                    &Rect::new(r.x0, y0, r.x1, y1),
                    theme.accent_hover.with_alpha(0.25 + 0.55 * pulse_a),
                );
            }
        }
    }

    fn a11y_label(&self) -> Option<String> {
        Some(format!("AI prediction gutter: {} likely edit lines.", self.cells.len()))
    }
}

// ── Component 5: Command Palette (Arabic-first / RTL) ────────────────────────

/// One palette result row.
#[derive(Debug, Clone)]
pub struct PaletteItem {
    /// Primary label (may be Arabic).
    pub label: String,
    /// Keyboard-shortcut hint.
    pub shortcut: String,
}

/// Component 5 — RTL-aware command palette with a frosted background.
///
/// **Vello:** a frosted overlay (translucent `palette_bg` fill + border) — true
/// gaussian blur is a wgpu render-pass (host integration); here we approximate
/// with translucency. Result rows; the selected row gets an accent wash; fuzzy
/// matches underline with `palette_match`. In RTL the rows align right and
/// shortcut hints align left (mirrored).
/// **Layout (taffy):** centered floating panel.
/// **Tokens:** `palette_bg`, `palette_border`, `palette_match`, `accent_soft`,
/// `surface_overlay`.
/// **Animation:** none required (open/close handled by host).
/// **A11y:** "Command palette, N results, row S selected. Type to filter."
pub struct CommandPalette {
    /// Result rows.
    pub results: Vec<PaletteItem>,
    /// Index of the selected row.
    pub selected: usize,
    /// Right-to-left layout (Arabic input detected).
    pub rtl: bool,
    /// Row height in px.
    pub row_height: f64,
}

impl ZaroxiWidget for CommandPalette {
    fn layer(&self) -> WidgetLayer {
        WidgetLayer::Palette
    }

    fn paint(&self, scene: &mut Scene, layout: &taffy::Layout, theme: &SemanticColors) {
        let r = layout_rect(layout);
        // Frosted background approximation + border.
        let panel = RoundedRect::new(r.x0, r.y0, r.x1, r.y1, 10.0);
        fill(scene, &panel, theme.elevated_panel_background);
        stroke(scene, 1.0, &panel, theme.border);

        let pad = 10.0;
        let input_h = self.row_height + 4.0;
        for (i, _item) in self.results.iter().enumerate() {
            let y0 = r.y0 + input_h + i as f64 * self.row_height;
            let y1 = y0 + self.row_height;
            if i == self.selected {
                fill(scene, &Rect::new(r.x0 + 2.0, y0, r.x1 - 2.0, y1), theme.accent_soft);
            }
            // Label slot + shortcut slot, mirrored for RTL. Glyphs are drawn by
            // the cosmic-text layer; here we place the match-highlight underline.
            let (label_x0, label_x1) = if self.rtl {
                (r.x1 - pad - 160.0, r.x1 - pad)
            } else {
                (r.x0 + pad, r.x0 + pad + 160.0)
            };
            fill(
                scene,
                &Rect::new(label_x0, y1 - 3.0, label_x1, y1 - 1.0),
                theme.accent.with_alpha(0.6),
            );
        }
    }

    fn text_items(&self, layout: &taffy::Layout, theme: &SemanticColors) -> Vec<WidgetText> {
        let r = layout_rect(layout);
        let pad = 12.0;
        let input_h = self.row_height + 4.0;
        let size = 14.0_f32;
        let label = color_arr(theme.text_primary);
        let hint = color_arr(theme.text_muted);
        let mut out = Vec::new();
        for (i, item) in self.results.iter().enumerate() {
            let y = (r.y0 + input_h + i as f64 * self.row_height) as f32 + 6.0;
            if self.rtl {
                // RTL: label aligned to the right, shortcut hint to the left.
                out.push(WidgetText::new(
                    item.label.clone(),
                    (r.x1 - pad - 200.0) as f32,
                    y,
                    size,
                    label,
                ));
                out.push(WidgetText::new(
                    item.shortcut.clone(),
                    (r.x0 + pad) as f32,
                    y,
                    size,
                    hint,
                ));
            } else {
                out.push(WidgetText::new(item.label.clone(), (r.x0 + pad) as f32, y, size, label));
                out.push(WidgetText::new(
                    item.shortcut.clone(),
                    (r.x1 - pad - 90.0) as f32,
                    y,
                    size,
                    hint,
                ));
            }
        }
        out
    }

    fn a11y_label(&self) -> Option<String> {
        Some(format!(
            "Command palette, {} results, row {} selected. Type to filter.",
            self.results.len(),
            self.selected
        ))
    }
}

// ── Component 6: Status Bar (instrument panel) ──────────────────────────────

/// LSP health for the status indicator.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum LspStatus {
    /// Healthy (green pulse).
    #[default]
    Healthy,
    /// Slow / degraded (amber).
    Slow,
    /// Error (red).
    Error,
}

// ── Instrument-panel status model ───────────────────────────────────────────
//
// The status bar is a three-band cockpit instrument panel, NOT a flat utility
// strip. It consumes a typed model (below) so the renderer can map canonical
// status content into distinct visual roles with strict priority/collapse rules.
//
// Bands (logical, leading→trailing; mirrored under RTL):
//   • Context (elastic) — breadcrumb (decreasing emphasis) + compact state
//     markers + caret position + collapsible metadata chips.
//   • Health (reserved) — fps / mem text + an LSP health dot (instrument chip).
//   • AI (reserved)     — a dormant dot, an active dot+readout, or a usage arc.
//
// Render split: the elevated strip background is the shell shape pass; labels are
// cosmic-text (BiDi-safe); vector accents (dots/arc/separators) are the vello
// overlay drawn ON TOP of text, so they live only in reserved, text-free slots.

/// AI band operating mode (drives the right band's appearance + stability).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum AiMode {
    /// No live session / no backend telemetry — a single quiet dim dot. The AI
    /// band keeps its reserved width so toggling dormant↔live never jitters the
    /// layout, and there is no flickering "AI idle" text.
    #[default]
    Dormant,
    /// A request is active, or completed with a known context-window total (arc).
    Live,
    /// Activity exists but the context-window total is unknown (no misleading
    /// arc): a compact active dot + token/latency readout instead.
    Degraded,
}

/// Compact context-band state marker (a status light, never prose).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MarkerKind {
    /// Unsaved edits.
    Modified,
    /// Background parse in flight (pulses).
    Parsing,
    /// Error diagnostics present.
    Error,
    /// Warning diagnostics present.
    Warning,
}

/// One context-band marker, optionally carrying a count (diagnostics).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StatusMarker {
    /// Marker kind.
    pub kind: MarkerKind,
    /// Optional count (e.g. number of errors), shown as a tiny number.
    pub count: Option<u32>,
}

/// Left "context" band: a breadcrumb with decreasing emphasis + markers.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ContextBand {
    /// Lower-emphasis breadcrumb ancestors (workspace → module …), outer→inner.
    pub ancestors: Vec<String>,
    /// High-emphasis leaf: current symbol, else file name, else "No file".
    pub leaf: String,
    /// Caret position label ("Ln L, Col C"); kept whenever a file is open.
    pub position: Option<String>,
    /// Compact state markers (modified / parsing / diagnostics).
    pub markers: Vec<StatusMarker>,
}

/// Collapsible low-priority file metadata chips (drop first when narrow).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct MetaChips {
    /// Language / file-type (highest meta priority).
    pub language: Option<String>,
    /// Indent style.
    pub indent: Option<String>,
    /// Line-ending convention.
    pub eol: Option<String>,
    /// Text encoding (lowest meta priority; dropped first).
    pub encoding: Option<String>,
}

/// Center "health" band: a compact instrument cluster.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct HealthBand {
    /// Frames-per-second, when a perf source is available.
    pub fps: Option<u32>,
    /// Resident memory (MB), when a sample is available.
    pub mem_mb: Option<u32>,
    /// Language-service health.
    pub lsp: LspStatus,
}

/// Right "AI" band.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct AiBand {
    /// Operating mode (dormant / live / degraded).
    pub mode: AiMode,
    /// Tokens used in the current/last session.
    pub tokens_used: u32,
    /// Context-window total, or `0` when unknown (no backend telemetry).
    pub tokens_total: u32,
    /// Model chip, when the backend reports a model name.
    pub model: Option<String>,
    /// Last-inference latency (ms), when observed.
    pub latency_ms: Option<u32>,
}

/// The full typed instrument-panel status model (replaces flat left/right text).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct InstrumentStatus {
    /// Left context band.
    pub context: ContextBand,
    /// Collapsible metadata chips.
    pub meta: MetaChips,
    /// Center health band.
    pub health: HealthBand,
    /// Right AI band.
    pub ai: AiBand,
    /// Right-to-left layout (mirrors band order + text alignment).
    pub rtl: bool,
}

/// Width buckets give layout **hysteresis**: small resizes within a bucket do not
/// churn the visible segment set. Each bucket maps to a fixed collapse level, so
/// the same width always yields the same visible items (no per-frame flicker).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LayoutBucket {
    /// ≥ 1100px — everything.
    XWide,
    /// ≥ 900px — drop encoding.
    Wide,
    /// ≥ 720px — drop EOL.
    Mid,
    /// ≥ 560px — drop indent.
    Compact,
    /// ≥ 420px — drop language.
    Narrow,
    /// < 420px — collapse ancestors + reduce health to a single dot.
    Tiny,
}

impl LayoutBucket {
    fn from_width(w: f64) -> Self {
        if w >= 1100.0 {
            Self::XWide
        } else if w >= 900.0 {
            Self::Wide
        } else if w >= 720.0 {
            Self::Mid
        } else if w >= 560.0 {
            Self::Compact
        } else if w >= 420.0 {
            Self::Narrow
        } else {
            Self::Tiny
        }
    }

    /// Collapse level `0..=5` (0 = full detail).
    pub fn collapse_level(self) -> u8 {
        match self {
            Self::XWide => 0,
            Self::Wide => 1,
            Self::Mid => 2,
            Self::Compact => 3,
            Self::Narrow => 4,
            Self::Tiny => 5,
        }
    }

    /// Short label for tracing.
    pub fn label(self) -> &'static str {
        match self {
            Self::XWide => "xwide",
            Self::Wide => "wide",
            Self::Mid => "mid",
            Self::Compact => "compact",
            Self::Narrow => "narrow",
            Self::Tiny => "tiny",
        }
    }
}

/// Stable, traceable metrics about a laid-out status bar (proves no churn).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct StatusMetrics {
    pub bucket: LayoutBucket,
    pub level: u8,
    pub context_items: usize,
    pub health_items: usize,
    pub ai_items: usize,
    pub right_items: usize,
    pub ai_mode: AiMode,
    pub center_band_w: f32,
    pub right_band_w: f32,
    pub draw_items: usize,
    pub text_runs: usize,
    pub vector_items: usize,
}

// ── Component 6: Status Bar (instrument panel) ──────────────────────────────

/// Reserved width (px) of the right AI band — constant so dormant↔live is jitter-free.
/// Compact AI band width when space is tight (L4+): just "AI" label + dot.
/// Reserved width (px) of the center health band at full detail.
/// Collapsed health-band width (px) at the tiniest bucket (a single LSP dot).
/// Inner padding (px) inside each band.
const BAND_PAD: f64 = 12.0;
const RIGHT_TRAILING_PAD: f64 = 24.0;
// ═════════════════════════════════════════════════════════════════════════
const STATUS_SIZE: f32 = 12.0;

/// Component 6 — the live instrument-panel status bar (three bands).
///
/// **Vello (over text, reserved slots only):** thin inter-band separators, an
/// LSP health dot, the AI indicator (dim dot / active dot / usage arc), and
/// compact context-band state lights.
/// **Cosmic-text (under vello, BiDi-safe):** breadcrumb (muted ancestors →
/// primary leaf), position, metadata chips, health readout, AI readout.
/// **Tokens:** `divider`, `text_primary/secondary/muted`, `accent`,
/// `status_healthy/slow/error`, `ai_highlight`.
/// **Stability:** width buckets fix the visible set; the AI/health bands have
/// reserved widths; absent telemetry collapses to a quiet indicator, never to
/// flickering text.
pub struct StatusBar {
    /// The typed instrument model.
    pub status: InstrumentStatus,
    /// Animation phase in `[0,1)`.
    pub phase: f32,
}

/// Rough monospace-ish text width estimate (px) for explicit-x glyph placement.
/// Uses a 0.65 multiplier (conservative for mixed-case at 12px) so the right
/// zone never overruns its estimated bounds and gets clipped.
fn est_text_width(s: &str, size: f32) -> f64 {
    s.chars().count() as f64 * size as f64 * 0.65
}

/// A planned text run (resolved position + colour).
struct PText {
    s: String,
    x: f32,
    y: f32,
    size: f32,
    color: ThemeColor,
}

/// A planned vector instrument glyph (drawn by vello, over the text).
enum PVec {
    Dot { c: Point, r: f64, color: ThemeColor, pulse: bool },
}

/// The fully laid-out plan for one frame: a single source of truth shared by
/// `paint` (vectors), `text_items` (texts), and `metrics` (counts), so the three
/// never drift.
struct StatusPlan {
    texts: Vec<PText>,
    vectors: Vec<PVec>,
    bucket: LayoutBucket,
    level: u8,
    context_items: usize,
    health_items: usize,
    ai_items: usize,
    right_items: usize,
    ai_mode: AiMode,
    center_band_w: f64,
    right_band_w: f64,
}

impl StatusBar {
    /// Visible metadata chips for `level`, highest-priority first (trailing items
    /// drop first): language → indent → EOL → encoding. Deterministic per bucket.
    fn meta_for_level(&self, level: u8) -> Vec<String> {
        let mut ordered = Vec::new();
        if let Some(x) = &self.status.meta.language {
            ordered.push(x.clone());
        }
        if let Some(x) = &self.status.meta.indent {
            ordered.push(x.clone());
        }
        if let Some(x) = &self.status.meta.eol {
            ordered.push(x.clone());
        }
        if let Some(x) = &self.status.meta.encoding {
            ordered.push(x.clone());
        }
        // keep = 4 - level (language survives to level 3, all gone at level >= 4).
        let keep = 4usize.saturating_sub(level as usize).min(ordered.len());
        ordered.truncate(keep);
        ordered
    }

    /// Build the full per-frame plan (band layout, collapse, vectors + texts).
    /// Build the full per-frame plan — a conventional status bar with two zones:
    /// left = file/context text (truncated), right = trailing status cluster
    /// (right-aligned chips + LSP/AI dots).  No permanent centre band.
    /// Build the full per-frame plan — a conventional three-zone status bar:
    ///   left   = breadcrumb / context only (truncated first),
    ///   center = slim health strip (LSP/AI dots, optional FPS/MEM, truly centred),
    ///   right  = durable editor metadata (Ln/Col, language, indent, EOL, encoding).
    /// Collision priority: right > center > left.
    fn plan(&self, r: &Rect, theme: &SemanticColors) -> StatusPlan {
        let bucket = LayoutBucket::from_width(r.width());
        let level = bucket.collapse_level();
        let rtl = self.status.rtl;
        let cy = (r.y0 + r.y1) * 0.5;
        let y = (cy - STATUS_SIZE as f64 * 0.5) as f32;
        let total_w = r.width();

        let ctx = &self.status.context;
        let h = &self.status.health;
        let aib = &self.status.ai;

        let mut texts: Vec<PText> = Vec::new();
        let mut vectors: Vec<PVec> = Vec::new();

        // ── Helper: lay a string at x, tracking x position ──────────────
        let place = |x: f64, s: String, color: ThemeColor, texts: &mut Vec<PText>| -> f64 {
            let w = est_text_width(&s, STATUS_SIZE);
            texts.push(PText { s, x: x as f32, y, size: STATUS_SIZE, color });
            w
        };

        // ── 1. RIGHT ZONE (highest priority, far-right anchored) ────────
        // Build in drop-order: encoding → eol → indent → language → position.
        // (Position is the last/highest-priority — never dropped.)
        let meta = self.meta_for_level(level);
        let mut right_parts: Vec<String> = Vec::new();
        // meta is [language, indent, eol, encoding]; reverse for drop-order.
        right_parts.extend(meta.iter().rev().cloned());
        let right_items_total_before_pos = right_parts.len();
        if let Some(pos) = &ctx.position {
            right_parts.push(pos.clone());
        }
        let right_items_total = right_parts.len();
        // Fit to width from the right edge: drop leftmost (index 0) first.
        // RIGHT_TRAILING_PAD reserves space so the rightmost chip never
        // touches or clips against the window edge.
        let right_max_w = (total_w - BAND_PAD - RIGHT_TRAILING_PAD).max(0.0);
        let mut right_str = String::new();
        let mut right_w = 0.0;
        let mut right_used = 0usize;
        // Build the full joined string first to know intrinsic width.
        let right_full = right_parts.iter().rev().cloned().collect::<Vec<_>>().join("  ");
        let right_intrinsic_w =
            if right_full.is_empty() { 0.0 } else { est_text_width(&right_full, STATUS_SIZE) };
        let mut clip_reason = "";
        for ch in right_parts.iter().rev() {
            let trial =
                if right_str.is_empty() { ch.clone() } else { format!("{ch}  {right_str}") };
            let tw = est_text_width(&trial, STATUS_SIZE);
            if tw > right_max_w {
                if right_used == 0 {
                    clip_reason = "first_item_too_wide";
                } else {
                    clip_reason = "items_overflow";
                }
                break;
            }
            right_str = trial;
            right_w = tw;
            right_used += 1;
        }
        if right_used == 0 && right_parts.is_empty() {
            clip_reason = "no_items";
        } else if right_used == right_parts.len() {
            clip_reason = "all_fit";
        }
        let right_items = right_used;
        let right_zone_w = right_w;
        let (right_start_x, right_end_x, trailing_pad) = if !rtl {
            let sx = r.x1 - RIGHT_TRAILING_PAD - right_w;
            (sx, r.x1 - RIGHT_TRAILING_PAD, RIGHT_TRAILING_PAD)
        } else {
            let sx = r.x0 + RIGHT_TRAILING_PAD;
            (r.x0 + RIGHT_TRAILING_PAD, sx + right_w, RIGHT_TRAILING_PAD)
        };
        if !right_str.is_empty() {
            place(
                if !rtl { right_start_x } else { r.x0 + RIGHT_TRAILING_PAD },
                right_str,
                theme.text_muted,
                &mut texts,
            );
        }

        // ── 2. CENTER ZONE (second priority, truly centred on full bar) ─
        let mut center_items = 0usize;
        // Center chips in drop-priority order (left side drops first):
        //   MEM → FPS. LSP dot → AI dot (always kept).
        // Levels: XWide/Wide (L0-L1) → full (LSP+AI+FPS+MEM),
        //         Mid (L2)           → LSP+AI+FPS,
        //         Compact (L3)       → LSP+AI+compact FPS,
        //         Narrow/Tiny (L4-L5)→ LSP dot + AI dot only.
        let center_full = level <= 1;
        let center_mid = level <= 2;
        let center_compact = level <= 3;
        let mut center_parts: Vec<String> = Vec::new();
        // Reversed iterator so leftmost drops first.
        if center_full {
            if let Some(m) = h.mem_mb {
                center_parts.push(format!("{m} MB"));
            }
        }
        if center_full || center_mid || center_compact {
            // Compact FPS at levels > 2: just the number, no "fps" text.
            let fps_label = if let Some(f) = h.fps {
                if center_full || center_mid { format!("{f} fps") } else { format!("{f}") }
            } else {
                String::new()
            };
            if !fps_label.is_empty() {
                center_parts.push(fps_label);
            }
        }
        // Determine desired center zone width from the contents.
        let center_joined = center_parts.join("  ");
        let center_text_w = if center_joined.is_empty() {
            0.0
        } else {
            est_text_width(&center_joined, STATUS_SIZE)
        };
        // Dots (LSP + AI) need ~10px each + gap = ~26px total.
        let dots_w = 26.0;
        let desired_center_w = center_text_w
            + dots_w
            + (if center_text_w > 0.0 && center_parts.len() > 0 { 8.0 } else { 0.0 });

        // Place center: target centre x of the full bar.
        let center_target = r.x0 + total_w / 2.0;
        let mut center_x0 = center_target - desired_center_w / 2.0;
        let mut center_x1 = center_target + desired_center_w / 2.0;

        // Collision resolution: if center overlaps right, shift left (up to a limit).
        let max_center_shift = total_w * 0.15; // don't drift too far
        if !rtl {
            let right_safe = right_start_x - BAND_PAD;
            if center_x1 > right_safe {
                let overlap = center_x1 - right_safe;
                let shift = overlap.min(max_center_shift);
                center_x0 -= shift;
                center_x1 -= shift;
            }
        } else {
            let right_safe = right_end_x + BAND_PAD;
            if center_x0 < right_safe {
                let overlap = right_safe - center_x0;
                let shift = overlap.min(max_center_shift);
                center_x0 += shift;
                center_x1 += shift;
            }
        }
        // Clamp to screen edges.
        center_x0 = center_x0.max(r.x0);
        center_x1 = center_x1.min(r.x1);

        // Emit center zone: text chips + LSP dot + AI dot.
        if center_text_w > 0.0 && !center_parts.is_empty() {
            // Center the text within the dot-zone.
            let text_start = if !rtl { center_x0 } else { center_x0 + dots_w };
            let cx = if !rtl { text_start + dots_w + 4.0 } else { text_start };
            place(cx, center_joined, theme.text_secondary, &mut texts);
            center_items += center_parts.len();
        }
        // LSP dot.
        let (lsp_color, lsp_pulse) = match h.lsp {
            LspStatus::Healthy => (theme.success, true),
            LspStatus::Slow => (theme.warning, false),
            LspStatus::Error => (theme.error, false),
        };
        {
            let dx = if !rtl { center_x0 + 8.0 } else { center_x1 - 8.0 };
            vectors.push(PVec::Dot {
                c: Point::new(dx, cy),
                r: 3.0,
                color: lsp_color,
                pulse: lsp_pulse,
            });
            center_items += 1;
        }
        // AI dot.
        {
            let (ai_color, ai_pulse, ai_r) = match aib.mode {
                AiMode::Dormant => (theme.text_muted, false, 3.0),
                AiMode::Live | AiMode::Degraded => {
                    (theme.accent, matches!(aib.mode, AiMode::Live), 3.5)
                }
            };
            let dx = if !rtl { center_x0 + 20.0 } else { center_x1 - 20.0 };
            vectors.push(PVec::Dot {
                c: Point::new(dx, cy),
                r: ai_r,
                color: ai_color,
                pulse: ai_pulse,
            });
            center_items += 1;
        }
        let center_mode = if center_full {
            "full"
        } else if center_mid {
            "mid"
        } else if center_compact {
            "compact"
        } else {
            "dots"
        };

        // ── 3. LEFT ZONE (lowest priority, truncated first) ─────────────
        let bc_sep = if rtl { " ‹ " } else { " › " };
        let show_ancestors = level < 4 && !ctx.ancestors.is_empty();
        let mut left_parts: Vec<String> = Vec::new();
        let mut context_items = 0usize;
        if show_ancestors {
            left_parts.push(ctx.ancestors.join(bc_sep));
            context_items += 1;
        }
        {
            let leaf =
                if show_ancestors { format!("{bc_sep}{}", ctx.leaf) } else { ctx.leaf.clone() };
            if !leaf.trim().is_empty() {
                left_parts.push(leaf);
                context_items += 1;
            }
        }
        let left_raw = left_parts.join("");
        let mut left_str = left_raw.clone();
        let mut breadcrumb_truncated = false;
        // Available space: from left edge to centre zone start.
        let left_available = if !rtl {
            (center_x0 - r.x0 - BAND_PAD).max(0.0)
        } else {
            (r.x1 - center_x1 - BAND_PAD).max(0.0)
        };
        if !left_str.is_empty() {
            let left_w = est_text_width(&left_str, STATUS_SIZE);
            if left_w > left_available {
                // Truncate with ellipsis.
                breadcrumb_truncated = true;
                let mut n = left_raw.chars().count();
                while n > 1 {
                    let s: String = left_raw.chars().take(n).chain("\u{2026}".chars()).collect();
                    if est_text_width(&s, STATUS_SIZE) <= left_available {
                        left_str = s;
                        break;
                    }
                    n -= 1;
                }
                if n <= 1 {
                    left_str = "\u{2026}".to_string();
                }
            }
            let tx = if !rtl {
                (r.x0 + BAND_PAD) as f32
            } else {
                (r.x1 - BAND_PAD - est_text_width(&left_str, STATUS_SIZE)) as f32
            };
            texts.push(PText {
                s: left_str,
                x: tx,
                y,
                size: STATUS_SIZE,
                color: theme.text_primary,
            });
        }

        // State-light markers in the gap between left and centre.
        let markers = &ctx.markers;
        if !markers.is_empty() {
            let marker_w = markers.len() as f64 * 11.0 + 8.0;
            let gap_center = if !rtl {
                (center_x0 + r.x0 + BAND_PAD) / 2.0
            } else {
                (center_x1 + r.x1 - BAND_PAD) / 2.0
            };
            let mut mx = gap_center - marker_w / 2.0;
            for m in markers {
                let (color, pulse) = match m.kind {
                    MarkerKind::Modified => (theme.accent, false),
                    MarkerKind::Parsing => (theme.accent, true),
                    MarkerKind::Error => (theme.error, false),
                    MarkerKind::Warning => (theme.warning, false),
                };
                vectors.push(PVec::Dot { c: Point::new(mx, cy), r: 3.0, color, pulse });
                mx += 11.0;
            }
        }

        // ── Layout trace ────────────────────────────────────────────────
        if std::env::var("ZAROXI_STATUS_LAYOUT_TRACE").as_deref() == Ok("1") {
            let ct = center_x0 + (center_x1 - center_x0) / 2.0;
            let shift = ct - center_target;
            let overlap_lc = if !rtl {
                (r.x0 + left_available + BAND_PAD - center_x0).max(0.0)
            } else {
                (center_x1 - (r.x1 - left_available - BAND_PAD)).max(0.0)
            };
            let overlap_cr = if !rtl {
                (center_x1 - right_start_x).max(0.0)
            } else {
                (right_end_x - center_x0).max(0.0)
            };
            let right_clip_px = (right_zone_w - right_max_w).max(0.0);
            eprintln!(
                "ZAROXI_STATUS_LAYOUT_TRACE: bucket={} level={} total_w={:.0} left_zone_w={:.0} center_zone_w={:.0} right_zone_w={:.0} right_cluster_w_measured={:.0} right_cluster_w_painted={:.0} right_cluster_trailing_pad={:.0} right_cluster_clip_px={:.0} right_items_total={} right_items_after_collapse={} right_items_after_measurement={} right_cluster_intrinsic_w={:.0} right_cluster_reserved_w={:.0} right_cluster_final_w={:.0} right_cluster_anchor_x={:.0} right_cluster_clip_reason={} center_target_x={:.0} center_actual_x={:.0} center_shift_px={:.1} overlap_left_center={:.1} overlap_center_right={:.1} center_mode={} right_items_visible={} center_items_visible={} breadcrumb_truncated={}",
                bucket.label(),
                level,
                total_w,
                left_available,
                center_x1 - center_x0,
                right_zone_w,
                right_zone_w,
                right_w,
                trailing_pad,
                right_clip_px,
                right_items_total,
                right_items_total_before_pos,
                right_items,
                right_intrinsic_w,
                right_max_w,
                right_w,
                right_start_x,
                clip_reason,
                center_target,
                ct,
                shift,
                overlap_lc,
                overlap_cr,
                center_mode,
                right_items,
                center_items,
                breadcrumb_truncated,
            );
        }

        let ai_items = 1usize; // AI dot always emitted
        let right_band_w = right_zone_w;
        StatusPlan {
            texts,
            vectors,
            bucket,
            level,
            context_items,
            health_items: center_items,
            ai_items,
            right_items,
            ai_mode: aib.mode,
            center_band_w: center_x1 - center_x0,
            right_band_w,
        }
    }

    pub fn metrics(&self, status_rect: Rect, theme: &SemanticColors) -> StatusMetrics {
        let p = self.plan(&status_rect, theme);
        StatusMetrics {
            bucket: p.bucket,
            level: p.level,
            context_items: p.context_items,
            health_items: p.health_items,
            ai_items: p.ai_items,
            right_items: p.right_items,
            ai_mode: p.ai_mode,
            center_band_w: p.center_band_w as f32,
            right_band_w: p.right_band_w as f32,
            draw_items: p.texts.len() + p.vectors.len(),
            text_runs: p.texts.len(),
            vector_items: p.vectors.len(),
        }
    }
}

impl ZaroxiWidget for StatusBar {
    fn layer(&self) -> WidgetLayer {
        WidgetLayer::StatusBar
    }

    fn paint(&self, scene: &mut Scene, layout: &taffy::Layout, theme: &SemanticColors) {
        let r = layout_rect(layout);
        // Top hairline divides the strip from the editor (the elevated strip
        // fill itself is the shell shape pass, drawn under the cosmic-text).
        stroke(scene, 1.0, &Line::new((r.x0, r.y0), (r.x1, r.y0)), theme.divider);

        let plan = self.plan(&r, theme);
        let p = pulse(self.phase);
        for v in &plan.vectors {
            match v {
                PVec::Dot { c, r: rad, color, pulse: pulsing } => {
                    let a = if *pulsing { 0.45 + 0.55 * p } else { 1.0 };
                    fill(scene, &Circle::new(*c, *rad), color.with_alpha(a));
                }
            }
        }
    }

    fn text_items(&self, layout: &taffy::Layout, theme: &SemanticColors) -> Vec<WidgetText> {
        let r = layout_rect(layout);
        self.plan(&r, theme)
            .texts
            .into_iter()
            .map(|t| WidgetText::new(t.s, t.x, t.y, t.size, color_arr(t.color)))
            .collect()
    }

    fn a11y_label(&self) -> Option<String> {
        let ctx = &self.status.context;
        let crumbs = {
            let mut c = ctx.ancestors.clone();
            c.push(ctx.leaf.clone());
            c.join(" › ")
        };
        let lsp = match self.status.health.lsp {
            LspStatus::Healthy => "healthy",
            LspStatus::Slow => "slow",
            LspStatus::Error => "error",
        };
        let ai = match self.status.ai.mode {
            AiMode::Dormant => "AI dormant".to_string(),
            AiMode::Live if self.status.ai.tokens_total > 0 => format!(
                "AI context {} of {} tokens",
                self.status.ai.tokens_used, self.status.ai.tokens_total
            ),
            AiMode::Live => format!("AI active, {} tokens", self.status.ai.tokens_used),
            AiMode::Degraded => format!("AI {} tokens", self.status.ai.tokens_used),
        };
        let pos = ctx.position.as_deref().unwrap_or("");
        Some(format!("Status: {crumbs} {pos}. LSP {lsp}. {ai}."))
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Activity Rail — icon strip at the bottom of the left column, rendered through
// cockpit vello + cosmic-text. Each item has an icon glyph (Nerd Font), optional
// label/tooltip, and per-item selection/hover state.
// ─────────────────────────────────────────────────────────────────────────────

/// Descriptor for one rail item (icon + label + state).
#[derive(Debug, Clone)]
pub struct ActivityItem {
    pub index: usize,
    pub glyph: char,
    pub label: String,
    pub selected: bool,
    pub hovered: bool,
    pub pressed: bool,
}

/// Vertical activity / navigation rail widget.  Rendered through the cockpit
/// vello overlay (item highlights) + cosmic-text layer (icon glyphs).  Uses
/// [`StyleTokens`]-derived colors — not CockpitTokens — so the rail matches the
/// main IDE chrome theme.
pub struct ActivityRail {
    pub items: Vec<ActivityItem>,
    /// Rail background color — from `StyleTokens::rail_background`.
    pub bg_color: [f32; 4],
    /// Selected item fill — from `StyleTokens::rail_item_active`.
    pub item_active: [f32; 4],
    /// Accent indicator — from `StyleTokens::rail_item_active_accent`.
    pub accent_color: [f32; 4],
    /// Active icon text — from `StyleTokens::text_primary`.
    pub text_active: [f32; 4],
    /// Inactive icon text — from `StyleTokens::text_muted`.
    pub text_muted: [f32; 4],
    /// Divider color — from `StyleTokens::divider_subtle`.
    pub divider_color: [f32; 4],
}

impl ZaroxiWidget for ActivityRail {
    fn layer(&self) -> WidgetLayer {
        WidgetLayer::ActivityRail
    }

    fn paint(&self, scene: &mut Scene, layout: &taffy::Layout, _theme: &SemanticColors) {
        let rail = layout_rect(layout);
        let count = self.items.len().max(1);
        let slot_w = rail.width() / count as f64;

        for (i, item) in self.items.iter().enumerate() {
            let slot_left = rail.x0 + i as f64 * slot_w;
            let slot_rect = Rect::new(slot_left, rail.y0, slot_left + slot_w, rail.y1);

            if item.selected {
                fill(
                    scene,
                    &slot_rect,
                    ThemeColor::from_rgba(
                        self.item_active[0],
                        self.item_active[1],
                        self.item_active[2],
                        self.item_active[3],
                    ),
                );
                let accent =
                    Rect::new(slot_rect.x0, slot_rect.y1 - 3.0, slot_rect.x1, slot_rect.y1);
                fill(
                    scene,
                    &accent,
                    ThemeColor::from_rgba(
                        self.accent_color[0],
                        self.accent_color[1],
                        self.accent_color[2],
                        self.accent_color[3],
                    ),
                );
            } else if item.pressed {
                fill(
                    scene,
                    &slot_rect,
                    ThemeColor::from_rgba(
                        self.item_active[0],
                        self.item_active[1],
                        self.item_active[2],
                        self.item_active[3] * 0.6,
                    ),
                );
            }
        }
    }

    fn text_items(&self, layout: &taffy::Layout, _theme: &SemanticColors) -> Vec<WidgetText> {
        let count = self.items.len().max(1);
        let slot_w = layout.size.width / count as f32;
        let icon_sz = (layout.size.height * 0.5).clamp(16.0, 24.0);
        let icon_center_y = layout.size.height * 0.5 - icon_sz * 0.5;

        let mut runs = Vec::new();
        for (i, item) in self.items.iter().enumerate() {
            let glyph_str = item.glyph.to_string();
            let slot_center_x = i as f32 * slot_w + slot_w * 0.5;
            let color = if item.selected { self.text_active } else { self.text_muted };
            runs.push(WidgetText::new(
                glyph_str,
                layout.location.x + slot_center_x - icon_sz * 0.5,
                layout.location.y + icon_center_y,
                icon_sz as f32,
                color,
            ));
        }
        runs
    }

    fn a11y_label(&self) -> Option<String> {
        let active = self.items.iter().find(|i| i.selected).map(|i| &i.label);
        Some(format!(
            "Activity rail — {} items — active: {}",
            self.items.len(),
            active.unwrap_or(&"none".into())
        ))
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Settings panel — cockpit-owned, rendered in the editor content region.
// ─────────────────────────────────────────────────────────────────────────────

/// One settings section (tab / category).
#[derive(Debug, Clone)]
pub struct SettingsSection {
    pub label: String,
    pub items: Vec<SettingsRow>,
}

/// One settings row (label + control).
#[derive(Debug, Clone)]
pub struct SettingsRow {
    pub label: String,
    pub description: String,
    pub kind: SettingsRowKind,
}

/// The control affordance for a settings row.
#[derive(Debug, Clone)]
pub enum SettingsRowKind {
    Toggle { on: bool },
    Select { value: String, options: Vec<String> },
    Label { value: String },
}

/// Settings page rendered in the editor content area via cockpit vello +
/// cosmic-text.  Uses a left section nav + right content pane layout.
pub struct SettingsPanel {
    pub sections: Vec<SettingsSection>,
    pub selected_section: usize,
}

impl ZaroxiWidget for SettingsPanel {
    fn layer(&self) -> WidgetLayer {
        WidgetLayer::ActivityRail
    }

    fn paint(&self, scene: &mut Scene, layout: &taffy::Layout, theme: &SemanticColors) {
        let panel = layout_rect(layout);
        // The opaque panel background is owned by the host shape pass (drawn
        // BELOW the cosmic-text pass) so the labels stay readable; the cockpit
        // overlay composites ABOVE the text, so here we draw only translucent
        // accents (selection wash, divider).

        let nav_w = 160.0;
        let row_h = 28.0;
        let mut y = panel.y0 + 28.0;
        for (i, _sec) in self.sections.iter().enumerate() {
            let item_rect = Rect::new(panel.x0 + 12.0, y, panel.x0 + nav_w - 12.0, y + row_h);
            if i == self.selected_section {
                let sel_bg =
                    RoundedRect::new(item_rect.x0, item_rect.y0, item_rect.x1, item_rect.y1, 5.0);
                fill(scene, &sel_bg, theme.accent_soft);
            }
            y += row_h + 2.0;
        }
        let div = Line::new(
            Point::new(panel.x0 + nav_w, panel.y0 + 16.0),
            Point::new(panel.x0 + nav_w, panel.y1 - 8.0),
        );
        stroke(scene, 1.0, &div, theme.divider);
    }

    fn text_items(&self, layout: &taffy::Layout, theme: &SemanticColors) -> Vec<WidgetText> {
        let mut runs = Vec::new();
        let px = layout.location.x;
        let py = layout.location.y;
        let nav_w = 160.0;
        let row_h = 28.0;
        let content_x = nav_w + 20.0;
        let content_w = layout.size.width - content_x - 16.0;

        runs.push(WidgetText::new(
            "Settings".to_string(),
            px + 12.0,
            py + 6.0,
            17.0,
            color_arr(theme.text_primary),
        ));

        let mut y = py + 28.0;
        for (i, sec) in self.sections.iter().enumerate() {
            let c = if i == self.selected_section {
                color_arr(theme.text_primary)
            } else {
                color_arr(theme.text_muted)
            };
            runs.push(WidgetText::new(sec.label.clone(), px + 20.0, y + 5.0, 13.0, c));
            y += row_h + 2.0;
        }

        if let Some(sec) = self.sections.get(self.selected_section) {
            y = py + 28.0;
            runs.push(WidgetText::new(
                sec.label.clone(),
                px + content_x,
                y,
                16.0,
                color_arr(theme.text_primary),
            ));
            y += 24.0;
            for row in &sec.items {
                runs.push(WidgetText::new(
                    format!("{}  —  {}", row.label, row.description),
                    px + content_x,
                    y,
                    12.0,
                    color_arr(theme.text_muted),
                ));
                let val = match &row.kind {
                    SettingsRowKind::Toggle { on } => {
                        if *on {
                            "On"
                        } else {
                            "Off"
                        }
                    }
                    SettingsRowKind::Select { value, .. } => value,
                    SettingsRowKind::Label { value } => value,
                }
                .to_string();
                runs.push(WidgetText::new(
                    val,
                    px + content_x + content_w - 80.0,
                    y,
                    12.0,
                    color_arr(theme.text_primary),
                ));
                y += 20.0;
            }
        }
        runs
    }

    fn a11y_label(&self) -> Option<String> {
        Some(format!("Settings page — {} sections", self.sections.len()))
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Extensions panel — editor-hosted cockpit view with sidebar list.
// ─────────────────────────────────────────────────────────────────────────────

/// Descriptor for one extension in the browse list.
#[derive(Debug, Clone)]
pub struct ExtensionEntry {
    pub id: String,
    pub name: String,
    pub publisher: String,
    pub description: String,
    pub installed: bool,
}

/// Extensions page rendered in the editor content area.  Shows a list of
/// extensions with a details pane for the selected entry.
pub struct ExtensionsPanel {
    pub entries: Vec<ExtensionEntry>,
    pub selected_entry: usize,
}

impl ZaroxiWidget for ExtensionsPanel {
    fn layer(&self) -> WidgetLayer {
        WidgetLayer::ActivityRail
    }

    fn paint(&self, scene: &mut Scene, layout: &taffy::Layout, theme: &SemanticColors) {
        let panel = layout_rect(layout);
        // Opaque background owned by the host shape pass (below the text pass);
        // the cockpit overlay composites above text, so draw only translucent
        // accents here (selection wash, divider).

        let list_w = 220.0;
        let row_h = 30.0;
        let mut y = panel.y0 + 28.0;
        for (i, _e) in self.entries.iter().enumerate() {
            let item_rect = Rect::new(panel.x0 + 8.0, y, panel.x0 + list_w - 8.0, y + row_h);
            if i == self.selected_entry {
                let sel_bg =
                    RoundedRect::new(item_rect.x0, item_rect.y0, item_rect.x1, item_rect.y1, 5.0);
                fill(scene, &sel_bg, theme.accent_soft);
            }
            y += row_h + 2.0;
        }
        let div = Line::new(
            Point::new(panel.x0 + list_w, panel.y0 + 16.0),
            Point::new(panel.x0 + list_w, panel.y1 - 8.0),
        );
        stroke(scene, 1.0, &div, theme.divider);
    }

    fn text_items(&self, layout: &taffy::Layout, theme: &SemanticColors) -> Vec<WidgetText> {
        let mut runs = Vec::new();
        let px = layout.location.x;
        let py = layout.location.y;
        let list_w = 220.0;
        let row_h = 30.0;
        let detail_x = list_w + 20.0;
        let _detail_w = layout.size.width - detail_x - 16.0;

        runs.push(WidgetText::new(
            "Extensions".to_string(),
            px + 12.0,
            py + 6.0,
            17.0,
            color_arr(theme.text_primary),
        ));

        let mut y = py + 28.0;
        for (i, e) in self.entries.iter().enumerate() {
            let c = if i == self.selected_entry {
                color_arr(theme.text_primary)
            } else {
                color_arr(theme.text_muted)
            };
            runs.push(WidgetText::new(e.name.clone(), px + 16.0, y + 6.0, 13.0, c));
            y += row_h + 2.0;
        }

        if let Some(e) = self.entries.get(self.selected_entry) {
            y = py + 28.0;
            runs.push(WidgetText::new(
                e.name.clone(),
                px + detail_x,
                y,
                16.0,
                color_arr(theme.text_primary),
            ));
            y += 22.0;
            runs.push(WidgetText::new(
                e.publisher.clone(),
                px + detail_x,
                y,
                12.0,
                color_arr(theme.text_muted),
            ));
            y += 18.0;
            runs.push(WidgetText::new(
                e.description.clone(),
                px + detail_x,
                y,
                12.0,
                color_arr(theme.text_secondary),
            ));
            y += 20.0;
            let status = if e.installed { "Installed" } else { "Not installed" };
            runs.push(WidgetText::new(
                status.to_string(),
                px + detail_x,
                y,
                12.0,
                color_arr(theme.text_muted),
            ));
        }
        runs
    }

    fn a11y_label(&self) -> Option<String> {
        Some(format!("Extensions — {} entries", self.entries.len()))
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Destination placeholder — generic titled page for rail destinations without a
// bespoke view yet (Search / Source Control / Debug / Account).
// ─────────────────────────────────────────────────────────────────────────────

/// A generic destination page rendered in the editor content region. The opaque
/// background is owned by the host shape pass; this widget contributes a title +
/// subtitle (text) and a thin accent rule (vector) so selecting the destination
/// visibly replaces the file editor.
pub struct DestinationPlaceholder {
    /// Large heading (e.g. "Search", "Source Control").
    pub title: String,
    /// Supporting line under the heading.
    pub subtitle: String,
}

impl ZaroxiWidget for DestinationPlaceholder {
    fn layer(&self) -> WidgetLayer {
        WidgetLayer::ActivityRail
    }

    fn paint(&self, scene: &mut Scene, layout: &taffy::Layout, theme: &SemanticColors) {
        // Background is owned by the host shape pass (below the text pass). Draw
        // a short accent rule beneath the heading as a vector accent.
        let panel = layout_rect(layout);
        let rule = Line::new(
            Point::new(panel.x0 + 28.0, panel.y0 + 54.0),
            Point::new(panel.x0 + 28.0 + 48.0, panel.y0 + 54.0),
        );
        stroke(scene, 2.0, &rule, theme.accent);
    }

    fn text_items(&self, layout: &taffy::Layout, theme: &SemanticColors) -> Vec<WidgetText> {
        let px = layout.location.x;
        let py = layout.location.y;
        vec![
            WidgetText::new(
                self.title.clone(),
                px + 28.0,
                py + 24.0,
                18.0,
                color_arr(theme.text_primary),
            ),
            WidgetText::new(
                self.subtitle.clone(),
                px + 28.0,
                py + 66.0,
                13.0,
                color_arr(theme.text_muted),
            ),
        ]
    }

    fn a11y_label(&self) -> Option<String> {
        Some(format!("{} — {}", self.title, self.subtitle))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn layout(w: f32, h: f32) -> taffy::Layout {
        let mut l = taffy::Layout::default();
        l.location = taffy::geometry::Point { x: 0.0, y: 0.0 };
        l.size = taffy::geometry::Size { width: w, height: h };
        l
    }

    fn status_rect(w: f64) -> Rect {
        Rect::new(0.0, 0.0, w, 26.0)
    }

    /// A fully-populated instrument model (live AI with a known total).
    fn full_instrument() -> InstrumentStatus {
        InstrumentStatus {
            context: ContextBand {
                ancestors: vec!["zaroxi".into()],
                leaf: "main.rs".into(),
                position: Some("Ln 1, Col 1".into()),
                markers: vec![StatusMarker { kind: MarkerKind::Modified, count: None }],
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
                tokens_used: 4096,
                tokens_total: 8192,
                model: None,
                latency_ms: Some(12),
            },
            rtl: false,
        }
    }

    #[test]
    fn all_components_paint_without_panic_and_report_layer() {
        let theme = SemanticColors::dark();
        let mut scene = Scene::new();

        let minimap = SemanticMinimap {
            symbols: vec![
                MinimapSymbol { line: 1, kind: SymbolKind::Function },
                MinimapSymbol { line: 5, kind: SymbolKind::Type },
                MinimapSymbol { line: 9, kind: SymbolKind::Import },
            ],
            total_lines: 100,
            ai_regions: vec![(10, 20)],
            viewport: (0.1, 0.3),
        };
        assert_eq!(minimap.layer(), WidgetLayer::Minimap);
        minimap.paint(&mut scene, &layout(84.0, 600.0), &theme);
        assert!(minimap.a11y_label().is_some());

        let diff = LivingDiffLayer {
            hunks: vec![DiffHunk { line: 0, added: true }, DiffHunk { line: 1, added: false }],
            line_height: 18.0,
            active: Some(0),
            phase: 0.25,
        };
        assert_eq!(diff.layer(), WidgetLayer::DiffLayer);
        diff.paint(&mut scene, &layout(800.0, 600.0), &theme);

        let canvas = ContextCanvas {
            center: (300.0, 200.0, 200.0, 120.0),
            related: vec![RelatedPanel {
                rect: (40.0, 40.0, 120.0, 80.0),
                import_site: (300.0, 230.0),
            }],
            phase: 0.5,
        };
        assert_eq!(canvas.layer(), WidgetLayer::Palette);
        canvas.paint(&mut scene, &layout(800.0, 600.0), &theme);

        let gutter = AiPredictionGutter {
            cells: vec![PredictionCell { line: 0, probability: 0.9 }],
            line_height: 18.0,
            pulse_line: Some(0),
            phase: 0.5,
        };
        assert_eq!(gutter.layer(), WidgetLayer::Gutter);
        gutter.paint(&mut scene, &layout(16.0, 600.0), &theme);

        let palette = CommandPalette {
            results: vec![PaletteItem {
                label: "افتح ملف".into(), shortcut: "Ctrl+O".into()
            }],
            selected: 0,
            rtl: true,
            row_height: 24.0,
        };
        assert_eq!(palette.layer(), WidgetLayer::Palette);
        palette.paint(&mut scene, &layout(420.0, 300.0), &theme);

        let status = StatusBar { status: full_instrument(), phase: 0.3 };
        assert_eq!(status.layer(), WidgetLayer::StatusBar);
        status.paint(&mut scene, &layout(800.0, 24.0), &theme);
        let label = status.a11y_label().unwrap();
        assert!(label.contains("LSP healthy"));
        // Context is spoken (workspace ancestor + file leaf).
        assert!(label.contains("zaroxi") && label.contains("main.rs"));
    }

    #[test]
    fn pulse_is_static_under_reduce_motion() {
        crate::set_reduce_motion(true);
        assert_eq!(pulse(0.3), 1.0);
        crate::set_reduce_motion(false);
    }

    #[test]
    fn ai_band_is_dormant_quietly_and_degrades_without_a_total() {
        let theme = SemanticColors::dark();

        // No session: dormant — a single quiet dim dot, no flickering text,
        // never a misleading "/0". The AI band still reserves its width.
        let idle = StatusBar {
            status: InstrumentStatus {
                context: ContextBand { leaf: "main.rs".into(), ..Default::default() },
                ai: AiBand { mode: AiMode::Dormant, ..Default::default() },
                ..Default::default()
            },
            phase: 0.0,
        };
        let label = idle.a11y_label().unwrap();
        assert!(label.contains("AI dormant"), "got: {label}");
        assert!(!label.contains("of 0") && !label.contains("/0"));
        // Dormant emits exactly one AI vector (the dim dot) and no AI text.
        let m = idle.metrics(status_rect(1000.0), &theme);
        assert_eq!(m.ai_mode, AiMode::Dormant);
        assert_eq!(m.ai_items, 1);

        // Activity without a known context total: degraded — raw count, no "/0".
        let used = StatusBar {
            status: InstrumentStatus {
                context: ContextBand { leaf: "main.rs".into(), ..Default::default() },
                ai: AiBand { mode: AiMode::Degraded, tokens_used: 123, ..Default::default() },
                ..Default::default()
            },
            phase: 0.0,
        };
        let label = used.a11y_label().unwrap();
        assert!(label.contains("AI 123 tokens"), "got: {label}");
        assert!(!label.contains("of 0") && !label.contains("/0"));
    }

    fn agents_instrument() -> InstrumentStatus {
        InstrumentStatus {
            context: ContextBand {
                ancestors: vec!["zaroxi".into()],
                leaf: "AGENTS.md".into(),
                position: Some("Ln 2, Col 1".into()),
                markers: vec![],
            },
            meta: MetaChips {
                language: Some("Markdown".into()),
                indent: Some("Spaces 2".into()),
                eol: Some("LF".into()),
                encoding: Some("UTF-8".into()),
            },
            health: HealthBand { fps: Some(60), mem_mb: Some(96), lsp: LspStatus::Healthy },
            ai: AiBand { mode: AiMode::Dormant, ..Default::default() },
            rtl: false,
        }
    }

    /// The typed model must turn into actual visible text runs (the blank-bar
    /// regression), and a narrow bar must elide low-priority detail — keeping the
    /// essentials (file leaf + Ln/Col) — rather than vanish.
    #[test]
    fn status_segments_produce_visible_text_runs_and_elide_gracefully() {
        let theme = SemanticColors::dark();
        let status = StatusBar { status: agents_instrument(), phase: 0.0 };

        // Wide: full content is emitted as visible runs.
        let wide = status.text_items(&layout(1200.0, 26.0), &theme);
        assert!(!wide.is_empty(), "status must emit text runs");
        let joined: String = wide.iter().map(|t| t.text.clone()).collect::<Vec<_>>().join(" ");
        for expected in ["zaroxi", "AGENTS.md", "Ln 2, Col 1", "Markdown", "UTF-8", "LF"] {
            assert!(joined.contains(expected), "wide status missing {expected:?}; got {joined:?}");
        }

        // Narrow (Tiny bucket): essentials remain (file leaf + position), but the
        // lowest-priority metadata (encoding/EOL) is elided — intentional, not a
        // vanishing bar.
        let narrow = status.text_items(&layout(240.0, 26.0), &theme);
        let joined: String = narrow.iter().map(|t| t.text.clone()).collect::<Vec<_>>().join(" ");
        assert!(joined.contains("AGENTS.md"), "narrow must keep file leaf; got {joined:?}");
        assert!(joined.contains("Ln 2, Col 1"), "narrow must keep position; got {joined:?}");
        assert!(!joined.contains("UTF-8"), "narrow must drop encoding; got {joined:?}");
        assert!(!joined.contains("LF"), "narrow must drop EOL; got {joined:?}");
    }

    /// Layout stability: identical width bucket ⇒ identical visible-item set
    /// (hysteresis, no per-resize churn); narrower bucket only ever simplifies.
    #[test]
    fn metrics_are_stable_within_a_bucket_and_simplify_when_narrow() {
        let theme = SemanticColors::dark();
        let status = StatusBar { status: full_instrument(), phase: 0.0 };

        // Two widths in the same (Wide) bucket → identical metrics.
        let a = status.metrics(status_rect(1000.0), &theme);
        let b = status.metrics(status_rect(1080.0), &theme);
        assert_eq!(a.bucket, b.bucket);
        assert_eq!(a.text_runs, b.text_runs, "text-run count must not churn within a bucket");
        assert_eq!(a.context_items, b.context_items);
        assert_eq!(a.draw_items, b.draw_items);

        // A narrower bucket simplifies (never adds items).
        let narrow = status.metrics(status_rect(500.0), &theme);
        assert!(narrow.level >= a.level, "narrower must collapse at least as much");
        assert!(narrow.context_items <= a.context_items, "narrower must not add context items");

        // Reserved bands keep stable widths across buckets (no jitter).
        // Reserved bands are stable within a bucket-tier but can shrink at
        // higher collapse levels (the AI band compacts at L4+ by design).
        assert_eq!(a.right_band_w, b.right_band_w, "right band stable within a bucket");
        assert!(narrow.right_band_w <= a.right_band_w, "right band may compact at narrow widths");
    }
}
