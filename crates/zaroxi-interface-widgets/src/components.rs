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
use zaroxi_interface_theme::Color as ThemeColor;
use zaroxi_interface_theme::CockpitTokens;

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
        let frac = if self.total_lines <= 1 {
            0.0
        } else {
            line as f64 / (self.total_lines - 1) as f64
        };
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
                SymbolKind::Import => {
                    stroke(scene, 1.0, &Line::new((r.x0 + 10.0, y), (r.x1 - 10.0, y)), theme.sym_import)
                }
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
                stroke(scene, 1.5, &Line::new((r.x0 + 4.0, mid), (r.x1 - 4.0, mid)), theme.diff_removed_strike);
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
        Some(format!("Context canvas: {} related files. Click a panel to focus.", self.related.len()))
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
            let heat = lerp_color(theme.ai_prediction_base, theme.ai_prediction_warm, cell.probability);
            fill(scene, &Rect::new(r.x0, y0, r.x1, y1), heat);
            if self.pulse_line == Some(cell.line) {
                fill(scene, &Rect::new(r.x0, y0, r.x1, y1), theme.ai_pulse.with_alpha(0.25 + 0.55 * pulse_a));
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
                out.push(WidgetText::new(item.label.clone(), (r.x1 - pad - 200.0) as f32, y, size, label));
                out.push(WidgetText::new(item.shortcut.clone(), (r.x0 + pad) as f32, y, size, hint));
            } else {
                out.push(WidgetText::new(item.label.clone(), (r.x0 + pad) as f32, y, size, label));
                out.push(WidgetText::new(item.shortcut.clone(), (r.x1 - pad - 90.0) as f32, y, size, hint));
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

/// Component 6 — the live instrument-panel status bar.
///
/// **Vello:** left breadcrumb slots + separators; center LSP dot (pulsing when
/// healthy) with FPS/mem slots; right AI-context **arc** (used/total sweep) +
/// model + latency slots.
/// **Layout (taffy):** full-width bar, three flex regions (start/center/end).
/// **Tokens:** `bg_elevated`, `divider`, `status_healthy/slow/error`, `accent`,
/// `ai_highlight`, `text_secondary`.
/// **Animation:** healthy LSP dot pulses (≈1000ms); the AI arc morphs its sweep
/// toward the target fraction. Static under reduce-motion.
/// **A11y:** "Status: <breadcrumb>. LSP <state>. AI context U of T tokens."
pub struct StatusBar {
    /// Symbol-path breadcrumb (file → mod → fn → expr).
    pub breadcrumb: Vec<String>,
    /// LSP health.
    pub lsp: LspStatus,
    /// AI tokens used / available.
    pub ai_tokens_used: u32,
    /// AI tokens available.
    pub ai_tokens_total: u32,
    /// Animation phase in `[0,1)`.
    pub phase: f32,
}

impl ZaroxiWidget for StatusBar {
    fn layer(&self) -> WidgetLayer {
        WidgetLayer::StatusBar
    }

    fn paint(&self, scene: &mut Scene, layout: &taffy::Layout, theme: &CockpitTokens) {
        let r = layout_rect(layout);
        fill(scene, &r, theme.bg_elevated);
        stroke(scene, 1.0, &Line::new((r.x0, r.y0), (r.x1, r.y0)), theme.divider);

        let cy = (r.y0 + r.y1) * 0.5;

        // Left: breadcrumb slots with chevron separators.
        let mut x = r.x0 + 10.0;
        for (i, _crumb) in self.breadcrumb.iter().enumerate() {
            if i > 0 {
                let mut chev = BezPath::new();
                chev.move_to((x, cy - 3.0));
                chev.line_to((x + 3.0, cy));
                chev.line_to((x, cy + 3.0));
                stroke(scene, 1.0, &chev, theme.text_muted);
                x += 8.0;
            }
            fill(scene, &Rect::new(x, cy - 4.0, x + 60.0, cy + 4.0), theme.surface);
            x += 68.0;
        }

        // Center: LSP health dot (pulses when healthy).
        let (dot_color, animate) = match self.lsp {
            LspStatus::Healthy => (theme.status_healthy, true),
            LspStatus::Slow => (theme.status_slow, false),
            LspStatus::Error => (theme.status_error, false),
        };
        let a = if animate { 0.4 + 0.6 * pulse(self.phase) } else { 1.0 };
        let center_x = (r.x0 + r.x1) * 0.5;
        fill(scene, &Circle::new(Point::new(center_x, cy), 4.0), dot_color.with_alpha(a));

        // Right: AI context usage arc.
        let frac = if self.ai_tokens_total == 0 {
            0.0
        } else {
            (self.ai_tokens_used as f64 / self.ai_tokens_total as f64).clamp(0.0, 1.0)
        };
        let arc_center = Point::new(r.x1 - 16.0, cy);
        let radius = 7.0;
        // Track + value arc (sweep proportional to usage), starting at top.
        let track = Arc::new(arc_center, (radius, radius), -std::f64::consts::FRAC_PI_2, std::f64::consts::TAU, 0.0);
        stroke(scene, 2.0, &track, theme.divider);
        let value = Arc::new(
            arc_center,
            (radius, radius),
            -std::f64::consts::FRAC_PI_2,
            std::f64::consts::TAU * frac,
            0.0,
        );
        stroke(scene, 2.0, &value, theme.ai_highlight);
    }

    fn text_items(&self, layout: &taffy::Layout, theme: &CockpitTokens) -> Vec<WidgetText> {
        let r = layout_rect(layout);
        let cy = ((r.y0 + r.y1) * 0.5) as f32;
        let size = 13.0_f32;
        let baseline_y = cy - size * 0.5;
        let mut out = Vec::new();
        let mut x = (r.x0 + 10.0) as f32;
        for (i, crumb) in self.breadcrumb.iter().enumerate() {
            if i > 0 {
                x += 8.0; // chevron separator gap
            }
            out.push(WidgetText::new(
                crumb.clone(),
                x,
                baseline_y,
                size,
                color_arr(theme.text_secondary),
            ));
            x += 68.0;
        }
        // AI context usage "used/total" to the left of the arc.
        out.push(WidgetText::new(
            format!("{}/{}", self.ai_tokens_used, self.ai_tokens_total),
            (r.x1 - 96.0) as f32,
            baseline_y,
            size,
            color_arr(theme.text_muted),
        ));
        out
    }

    fn a11y_label(&self) -> Option<String> {
        let state = match self.lsp {
            LspStatus::Healthy => "healthy",
            LspStatus::Slow => "slow",
            LspStatus::Error => "error",
        };
        Some(format!(
            "Status: {}. LSP {state}. AI context {} of {} tokens.",
            self.breadcrumb.join(" \u{203a} "),
            self.ai_tokens_used,
            self.ai_tokens_total
        ))
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
            related: vec![RelatedPanel { rect: (40.0, 40.0, 120.0, 80.0), import_site: (300.0, 230.0) }],
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
            results: vec![PaletteItem { label: "افتح ملف".into(), shortcut: "Ctrl+O".into() }],
            selected: 0,
            rtl: true,
            row_height: 24.0,
        };
        assert_eq!(palette.layer(), WidgetLayer::Palette);
        palette.paint(&mut scene, &layout(420.0, 300.0), &theme);

        let status = StatusBar {
            breadcrumb: vec!["main.rs".into(), "app".into(), "run".into()],
            lsp: LspStatus::Healthy,
            ai_tokens_used: 4096,
            ai_tokens_total: 8192,
            phase: 0.3,
        };
        assert_eq!(status.layer(), WidgetLayer::StatusBar);
        status.paint(&mut scene, &layout(800.0, 24.0), &theme);
        assert!(status.a11y_label().unwrap().contains("LSP healthy"));
    }

    #[test]
    fn pulse_is_static_under_reduce_motion() {
        crate::set_reduce_motion(true);
        assert_eq!(pulse(0.3), 1.0);
        crate::set_reduce_motion(false);
    }
}
