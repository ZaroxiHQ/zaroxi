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
use vello::kurbo::{Affine, Arc, BezPath, Circle, Line, Point, Rect, RoundedRect, Stroke};
use vello::peniko::Fill;
use zaroxi_interface_theme::CockpitTokens;
use zaroxi_interface_theme::Color as ThemeColor;

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

    fn paint(&self, scene: &mut Scene, layout: &taffy::Layout, theme: &CockpitTokens) {
        let r = layout_rect(layout);
        fill(scene, &r, theme.minimap_bg);

        // AI-modified regions (drawn first, behind symbols).
        for (start, end) in &self.ai_regions {
            let y0 = self.y_of(*start, r.y0, r.height());
            let y1 = self.y_of(*end, r.y0, r.height()).max(y0 + 2.0);
            fill(scene, &Rect::new(r.x0, y0, r.x1, y1), theme.minimap_ai_region);
        }

        let cx = (r.x0 + r.x1) * 0.5;
        for sym in &self.symbols {
            let y = self.y_of(sym.line, r.y0, r.height());
            match sym.kind {
                SymbolKind::Function => {
                    let block = RoundedRect::new(r.x0 + 6.0, y - 2.0, r.x1 - 6.0, y + 2.0, 1.5);
                    fill(scene, &block, theme.sym_function);
                }
                SymbolKind::Type => fill(scene, &diamond(cx, y, 4.0), theme.sym_type),
                SymbolKind::Import => stroke(
                    scene,
                    1.0,
                    &Line::new((r.x0 + 10.0, y), (r.x1 - 10.0, y)),
                    theme.sym_import,
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

    fn paint(&self, scene: &mut Scene, layout: &taffy::Layout, theme: &CockpitTokens) {
        let r = layout_rect(layout);
        let glow = pulse(self.phase);
        for (i, h) in self.hunks.iter().enumerate() {
            let y0 = r.y0 + h.line as f64 * self.line_height;
            let y1 = y0 + self.line_height;
            let is_active = self.active == Some(i);
            if h.added {
                fill(scene, &Rect::new(r.x0, y0, r.x1, y1), theme.diff_added_bg);
                let border_color = if is_active {
                    theme.diff_added_border.with_alpha(0.4 + 0.6 * glow)
                } else {
                    theme.diff_added_border
                };
                fill(scene, &Rect::new(r.x0, y0, r.x0 + 3.0, y1), border_color);
            } else {
                fill(scene, &Rect::new(r.x0, y0, r.x1, y1), theme.diff_removed_bg);
                let mid = (y0 + y1) * 0.5;
                stroke(
                    scene,
                    1.5,
                    &Line::new((r.x0 + 4.0, mid), (r.x1 - 4.0, mid)),
                    theme.diff_removed_strike,
                );
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

    fn paint(&self, scene: &mut Scene, _layout: &taffy::Layout, theme: &CockpitTokens) {
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
            stroke(scene, 1.5, &path, theme.canvas_connection);

            // Flow particle along the curve.
            let t = if reduce_motion() { 0.5 } else { self.phase.fract() as f64 };
            let pos = cubic(src, c1, c2, dst, t);
            fill(scene, &Circle::new(pos, 2.5), theme.canvas_particle);

            // Related panel.
            let pr = RoundedRect::new(px, py, px + pw, py + ph, 6.0);
            fill(scene, &pr, theme.canvas_panel_bg);
            stroke(scene, 1.0, &pr, theme.canvas_glow);
        }

        // Focused file panel on top.
        let cr = RoundedRect::new(cx, cy, cx + cw, cy + ch, 8.0);
        fill(scene, &cr, theme.canvas_panel_bg);
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

    fn paint(&self, scene: &mut Scene, layout: &taffy::Layout, theme: &CockpitTokens) {
        let r = layout_rect(layout);
        fill(scene, &r, theme.bg_elevated);
        let pulse_a = pulse(self.phase);
        for cell in &self.cells {
            let y0 = r.y0 + cell.line as f64 * self.line_height;
            let y1 = y0 + self.line_height - 1.0;
            let heat =
                lerp_color(theme.ai_prediction_base, theme.ai_prediction_warm, cell.probability);
            fill(scene, &Rect::new(r.x0, y0, r.x1, y1), heat);
            if self.pulse_line == Some(cell.line) {
                fill(
                    scene,
                    &Rect::new(r.x0, y0, r.x1, y1),
                    theme.ai_pulse.with_alpha(0.25 + 0.55 * pulse_a),
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

    fn paint(&self, scene: &mut Scene, layout: &taffy::Layout, theme: &CockpitTokens) {
        let r = layout_rect(layout);
        // Frosted background approximation + border.
        let panel = RoundedRect::new(r.x0, r.y0, r.x1, r.y1, 10.0);
        fill(scene, &panel, theme.palette_bg);
        stroke(scene, 1.0, &panel, theme.palette_border);

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
                theme.palette_match.with_alpha(0.6),
            );
        }
    }

    fn text_items(&self, layout: &taffy::Layout, theme: &CockpitTokens) -> Vec<WidgetText> {
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
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
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
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
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
    /// Width bucket.
    pub bucket: LayoutBucket,
    /// Collapse level.
    pub level: u8,
    /// Visible context-band items (ancestors + leaf + markers + position + meta).
    pub context_items: usize,
    /// Visible health-band items.
    pub health_items: usize,
    /// Visible AI-band items.
    pub ai_items: usize,
    /// AI mode.
    pub ai_mode: AiMode,
    /// Reserved center (health) band width.
    pub center_band_w: f32,
    /// Reserved right (AI) band width.
    pub right_band_w: f32,
    /// Total draw items (text runs + vector items).
    pub draw_items: usize,
    /// Text runs emitted.
    pub text_runs: usize,
    /// Vector items emitted.
    pub vector_items: usize,
}

// ── Component 6: Status Bar (instrument panel) ──────────────────────────────

/// Reserved width (px) of the right AI band — constant so dormant↔live is jitter-free.
const AI_BAND_W: f64 = 118.0;
/// Compact AI band width when space is tight (L4+): just "AI" label + dot.
const AI_COMPACT_W: f64 = 60.0;
/// Reserved width (px) of the center health band at full detail.
const HEALTH_BAND_W: f64 = 124.0;
/// Collapsed health-band width (px) at the tiniest bucket (a single LSP dot).
const HEALTH_BAND_MIN_W: f64 = 22.0;
/// Inner padding (px) inside each band.
const BAND_PAD: f64 = 12.0;
/// Uniform label size (px); emphasis is by colour, not size (calm, dense panel).
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
fn est_text_width(s: &str, size: f32) -> f64 {
    s.chars().count() as f64 * size as f64 * 0.52
}

/// Format a token count compactly (`2048` → `2.0k`).
fn fmt_tokens(n: u32) -> String {
    if n >= 1000 { format!("{:.1}k", n as f32 / 1000.0) } else { n.to_string() }
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
    Sep { x: f64, y0: f64, y1: f64 },
    Dot { c: Point, r: f64, color: ThemeColor, pulse: bool },
    Arc { c: Point, r: f64, frac: f64, track: ThemeColor, value: ThemeColor },
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

    /// Lay a directional sequence of `(text, colour)` runs from `outer_x` toward
    /// the band's inner edge (left→right for LTR, right→left for RTL).
    fn lay_sequence(
        items: &[(String, ThemeColor)],
        outer_x: f32,
        y: f32,
        size: f32,
        rtl: bool,
        gap: f32,
        out: &mut Vec<PText>,
    ) {
        let mut x = outer_x;
        for (s, c) in items {
            if s.is_empty() {
                continue;
            }
            let w = est_text_width(s, size) as f32;
            let run_x = if rtl { x - w } else { x };
            out.push(PText { s: s.clone(), x: run_x, y, size, color: *c });
            x = if rtl { x - w - gap } else { x + w + gap };
        }
    }

    /// Build the full per-frame plan (band layout, collapse, vectors + texts).
    fn plan(&self, r: &Rect, theme: &CockpitTokens) -> StatusPlan {
        let bucket = LayoutBucket::from_width(r.width());
        let level = bucket.collapse_level();
        let cy = (r.y0 + r.y1) * 0.5;
        let y = (cy - STATUS_SIZE as f64 * 0.5) as f32;
        let rtl = self.status.rtl;

        // ── 1. Desired band widths (collapsible per level) ───────────────
        let ai_w = if level >= 4 { AI_COMPACT_W } else { AI_BAND_W };
        let health_w = if level >= 5 {
            HEALTH_BAND_MIN_W
        } else if level >= 3 {
            HEALTH_BAND_MIN_W + 60.0
        } else {
            HEALTH_BAND_W
        };

        // ── 2. Place bands: AI hugs far edge; health is CENTRED on the full
        //       strip; context fills remainder.  Collision priority: AI > health
        //       > context (health shifts toward context when it overlaps AI;
        //       never paint through another band). ─────────────────────────
        let total_w = r.width();
        let slot_center = r.x0 + total_w / 2.0;
        let band = |x: f64, w: f64| Rect::new(x, r.y0, x + w.max(0.0), r.y1);

        let ai_rect = if !rtl { band(r.x1 - ai_w, ai_w) } else { band(r.x0, ai_w) };
        let health_rect = band((slot_center - health_w / 2.0).max(r.x0), health_w);

        // Resolve slot ↔ AI overlap.
        let health_rect = if !rtl {
            let overlap = (health_rect.x1 - ai_rect.x0).max(0.0);
            band((health_rect.x0 - overlap).max(r.x0), health_w)
        } else {
            let overlap = (ai_rect.x1 - health_rect.x0).max(0.0);
            let shifted_x0 = (health_rect.x0 + overlap).min(r.x1 - health_w.max(0.0));
            band(shifted_x0.max(r.x0), health_w)
        };
        let health_rect = if !rtl {
            band(health_rect.x0, (health_rect.width().min(r.x1 - health_rect.x0)).max(0.0))
        } else {
            band(health_rect.x0, (health_rect.width().min(health_rect.x1 - r.x0)).max(0.0))
        };

        let context_rect = if !rtl {
            band(r.x0, (health_rect.x0 - r.x0).max(0.0))
        } else {
            band(health_rect.x1, (r.x1 - health_rect.x1).max(0.0))
        };

        // ── Layout trace ────────────────────────────────────────────────
        if std::env::var("ZAROXI_STATUS_LAYOUT_TRACE").as_deref() == Ok("1") {
            let center_target = r.x0 + total_w / 2.0;
            let center_actual = health_rect.x0 + health_rect.width() / 2.0;
            let shift = center_actual - center_target;
            let overlap_lc = if !rtl {
                (context_rect.x1 - health_rect.x0).max(0.0)
            } else {
                (health_rect.x1 - context_rect.x0).max(0.0)
            };
            let overlap_cr = if !rtl {
                (ai_rect.x0 - health_rect.x1).max(0.0)
            } else {
                (health_rect.x0 - ai_rect.x1).max(0.0)
            };
            eprintln!(
                "ZAROXI_STATUS_LAYOUT_TRACE: bucket={} level={} total_w={:.0} context_w={:.0} center_w={:.0} ai_w={:.0} center_target_x={:.0} center_actual_x={:.0} center_shift_px={:.1} overlap_lc={:.1} overlap_cr={:.1} ai_compact={} health_slim={}",
                bucket.label(),
                level,
                total_w,
                context_rect.width(),
                health_rect.width(),
                ai_rect.width(),
                center_target,
                center_actual,
                shift,
                overlap_lc,
                overlap_cr,
                ai_w <= AI_COMPACT_W + 1.0,
                health_w <= HEALTH_BAND_MIN_W + 2.0,
            );
        }

        let context = context_rect;
        let health = health_rect;
        let ai = ai_rect;

        let mut texts: Vec<PText> = Vec::new();
        let mut vectors: Vec<PVec> = Vec::new();

        // Inter-band separators (thin, low-contrast).
        for sx in [health.x0, health.x1] {
            vectors.push(PVec::Sep { x: sx, y0: r.y0 + 5.0, y1: r.y1 - 5.0 });
        }

        // ── Context band ──────────────────────────────────────────────────
        let ctx = &self.status.context;
        let bc_sep = if rtl { " ‹ " } else { " › " };
        let show_ancestors = level < 5 && !ctx.ancestors.is_empty();

        let mut context_items = 0usize;
        let mut seq: Vec<(String, ThemeColor)> = Vec::new();
        if show_ancestors {
            seq.push((ctx.ancestors.join(bc_sep), theme.text_muted));
            context_items += 1;
        }
        // Leaf carries a leading separator only when ancestors precede it.
        let leaf = if show_ancestors { format!("{bc_sep}{}", ctx.leaf) } else { ctx.leaf.clone() };
        if !leaf.trim().is_empty() {
            seq.push((leaf, theme.text_primary));
            context_items += 1;
        }
        if let Some(pos) = &ctx.position {
            seq.push((pos.clone(), theme.text_muted));
            context_items += 1;
        }
        let meta = self.meta_for_level(level);
        context_items += meta.len();
        if !meta.is_empty() {
            seq.push((meta.join(" · "), theme.text_muted));
        }

        // Reserve a compact state-light cluster at the context band's inner edge.
        let markers = &ctx.markers;
        let marker_slot = markers.len() as f64 * 11.0 + if markers.is_empty() { 0.0 } else { 6.0 };
        let outer_x =
            if rtl { (context.x1 - BAND_PAD) as f32 } else { (context.x0 + BAND_PAD) as f32 };
        Self::lay_sequence(&seq, outer_x, y, STATUS_SIZE, rtl, 8.0, &mut texts);

        // State lights (vector dots) + optional diagnostic count text.
        if !markers.is_empty() {
            let mut mx = if rtl { context.x0 + 6.0 } else { context.x1 - marker_slot + 6.0 };
            let mut diag_text: Option<String> = None;
            let mut diag_color = theme.status_slow;
            for m in markers {
                let (color, pulse) = match m.kind {
                    MarkerKind::Modified => (theme.accent, false),
                    MarkerKind::Parsing => (theme.ai_highlight, true),
                    MarkerKind::Error => (theme.status_error, false),
                    MarkerKind::Warning => (theme.status_slow, false),
                };
                vectors.push(PVec::Dot { c: Point::new(mx, cy), r: 3.0, color, pulse });
                mx += 11.0;
                if let Some(n) = m.count {
                    let tag = match m.kind {
                        MarkerKind::Error => format!("E{n}"),
                        MarkerKind::Warning => format!("W{n}"),
                        _ => continue,
                    };
                    diag_color = if matches!(m.kind, MarkerKind::Error) {
                        theme.status_error
                    } else {
                        diag_color
                    };
                    diag_text = Some(match diag_text {
                        Some(prev) => format!("{prev} {tag}"),
                        None => tag,
                    });
                }
            }
            if let Some(d) = diag_text {
                let w = est_text_width(&d, STATUS_SIZE) as f32;
                let dx = if rtl {
                    context.x0 as f32 + 6.0 + marker_slot as f32
                } else {
                    context.x1 as f32 - w - 6.0
                };
                texts.push(PText { s: d, x: dx, y, size: STATUS_SIZE, color: diag_color });
            }
        }

        // ── Health band ───────────────────────────────────────────────────
        let h = &self.status.health;
        let (lsp_color, lsp_pulse) = match h.lsp {
            LspStatus::Healthy => (theme.status_healthy, true),
            LspStatus::Slow => (theme.status_slow, false),
            LspStatus::Error => (theme.status_error, false),
        };
        let lsp_x = if !rtl { health.x0 + 11.0 } else { health.x1 - 11.0 };
        vectors.push(PVec::Dot {
            c: Point::new(lsp_x, cy),
            r: 3.5,
            color: lsp_color,
            pulse: lsp_pulse,
        });
        let mut health_items = 1usize;
        if level < 5 {
            let mut parts: Vec<String> = Vec::new();
            if let Some(f) = h.fps {
                parts.push(format!("{f}"));
            }
            if let Some(m) = h.mem_mb {
                parts.push(format!("{m}MB"));
            }
            health_items += parts.len();
            if !parts.is_empty() {
                let s = parts.join("  ");
                let tx = if !rtl {
                    (lsp_x + 9.0) as f32
                } else {
                    (lsp_x - 9.0 - est_text_width(&s, STATUS_SIZE)) as f32
                };
                texts.push(PText { s, x: tx, y, size: STATUS_SIZE, color: theme.text_secondary });
            }
        }

        // ── AI band ───────────────────────────────────────────────────────
        let aib = &self.status.ai;
        // Indicator anchored at the band's outer edge (far side).
        let ind_x = if !rtl { ai.x1 - 15.0 } else { ai.x0 + 15.0 };
        let mut ai_items = 1usize; // always at least one presence indicator
        let ai_text: Option<String> = match aib.mode {
            AiMode::Dormant => {
                // Quiet dormant dot only (dim) — no flickering text.
                vectors.push(PVec::Dot {
                    c: Point::new(ind_x, cy),
                    r: 3.0,
                    color: theme.text_muted,
                    pulse: false,
                });
                None
            }
            AiMode::Live if aib.tokens_total > 0 => {
                let frac = (aib.tokens_used as f64 / aib.tokens_total as f64).clamp(0.0, 1.0);
                vectors.push(PVec::Arc {
                    c: Point::new(ind_x, cy),
                    r: 6.0,
                    frac,
                    track: theme.divider,
                    value: theme.ai_highlight,
                });
                let mut parts = vec![format!(
                    "{}/{}",
                    fmt_tokens(aib.tokens_used),
                    fmt_tokens(aib.tokens_total)
                )];
                if let Some(ms) = aib.latency_ms {
                    parts.push(format!("{ms}ms"));
                }
                ai_items += parts.len();
                Some(parts.join("  "))
            }
            AiMode::Live | AiMode::Degraded => {
                // Active or completed without a context total: a compact amber
                // dot (pulses while a request is live) + a token/latency readout.
                vectors.push(PVec::Dot {
                    c: Point::new(ind_x, cy),
                    r: 3.5,
                    color: theme.ai_highlight,
                    pulse: matches!(aib.mode, AiMode::Live),
                });
                let mut parts: Vec<String> = Vec::new();
                if aib.tokens_used > 0 {
                    parts.push(fmt_tokens(aib.tokens_used));
                }
                if let Some(ms) = aib.latency_ms {
                    parts.push(format!("{ms}ms"));
                }
                ai_items += parts.len();
                if parts.is_empty() { None } else { Some(parts.join("  ")) }
            }
        };
        if let Some(model) = &aib.model {
            // Optional model chip, inner-most in the AI band.
            ai_items += 1;
            let mx = if !rtl {
                ai.x0 as f32 + BAND_PAD as f32
            } else {
                (ai.x1 - BAND_PAD) as f32 - est_text_width(model, STATUS_SIZE) as f32
            };
            texts.push(PText {
                s: model.clone(),
                x: mx,
                y,
                size: STATUS_SIZE,
                color: theme.text_muted,
            });
        }
        if let Some(t) = ai_text {
            let w = est_text_width(&t, STATUS_SIZE) as f32;
            let tx = if !rtl { (ind_x - 11.0) as f32 - w } else { (ind_x + 11.0) as f32 };
            texts.push(PText { s: t, x: tx, y, size: STATUS_SIZE, color: theme.text_muted });
        }

        StatusPlan {
            texts,
            vectors,
            bucket,
            level,
            context_items,
            health_items,
            ai_items,
            ai_mode: aib.mode,
            center_band_w: health.width(),
            right_band_w: ai.width(),
        }
    }

    /// Traceable, stable metrics for the current width (proves no segment churn).
    pub fn metrics(&self, status_rect: Rect, theme: &CockpitTokens) -> StatusMetrics {
        let p = self.plan(&status_rect, theme);
        StatusMetrics {
            bucket: p.bucket,
            level: p.level,
            context_items: p.context_items,
            health_items: p.health_items,
            ai_items: p.ai_items,
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

    fn paint(&self, scene: &mut Scene, layout: &taffy::Layout, theme: &CockpitTokens) {
        let r = layout_rect(layout);
        // Top hairline divides the strip from the editor (the elevated strip
        // fill itself is the shell shape pass, drawn under the cosmic-text).
        stroke(scene, 1.0, &Line::new((r.x0, r.y0), (r.x1, r.y0)), theme.divider);

        let plan = self.plan(&r, theme);
        let p = pulse(self.phase);
        for v in &plan.vectors {
            match v {
                PVec::Sep { x, y0, y1 } => {
                    stroke(
                        scene,
                        1.0,
                        &Line::new((*x, *y0), (*x, *y1)),
                        theme.divider.with_alpha(0.6),
                    );
                }
                PVec::Dot { c, r: rad, color, pulse: pulsing } => {
                    let a = if *pulsing { 0.45 + 0.55 * p } else { 1.0 };
                    fill(scene, &Circle::new(*c, *rad), color.with_alpha(a));
                }
                PVec::Arc { c, r: rad, frac, track, value } => {
                    let base = Arc::new(
                        *c,
                        (*rad, *rad),
                        -std::f64::consts::FRAC_PI_2,
                        std::f64::consts::TAU,
                        0.0,
                    );
                    stroke(scene, 2.0, &base, *track);
                    let val = Arc::new(
                        *c,
                        (*rad, *rad),
                        -std::f64::consts::FRAC_PI_2,
                        std::f64::consts::TAU * frac,
                        0.0,
                    );
                    stroke(scene, 2.0, &val, *value);
                }
            }
        }
    }

    fn text_items(&self, layout: &taffy::Layout, theme: &CockpitTokens) -> Vec<WidgetText> {
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
        let theme = CockpitTokens::void();
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
        let theme = CockpitTokens::void();

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
        let theme = CockpitTokens::void();
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
        let theme = CockpitTokens::void();
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
