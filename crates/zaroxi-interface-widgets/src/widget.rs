//! The `ZaroxiWidget` trait, compositing layers, the global reduce-motion flag,
//! and shared vello paint helpers.

use std::sync::atomic::{AtomicBool, Ordering};

use vello::Scene;
use vello::kurbo::{Affine, Rect};
use vello::peniko::{Color as VColor, Fill};
use zaroxi_interface_theme::Color as ThemeColor;
use zaroxi_interface_theme::SemanticColors;

/// Compositing layer for a widget. The [`crate::WidgetTree`] paints widgets in
/// ascending layer order; the `Ord` derive encodes that order:
/// `Background < Editor < DiffLayer < Gutter < Minimap < StatusBar < Palette < Tooltip`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum WidgetLayer {
    /// Window background / base surface.
    Background,
    /// Editor text buffer.
    Editor,
    /// Translucent AI diff overlay (above text, below cursor).
    DiffLayer,
    /// Left gutter (line numbers, blame heat, LSP/AI markers).
    Gutter,
    /// Semantic minimap / scroll widget.
    Minimap,
    /// Instrument-panel status bar.
    StatusBar,
    /// Activity / navigation rail (icon strip at the bottom of the left column).
    ActivityRail,
    /// Command palette (when open).
    Palette,
    /// Tooltips / transient overlays.
    Tooltip,
}

/// Global "reduce motion" flag. All animated widgets must check
/// [`reduce_motion`] and fall back to a static frame when it is set.
static REDUCE_MOTION: AtomicBool = AtomicBool::new(false);

/// Enable/disable the global reduce-motion flag.
pub fn set_reduce_motion(on: bool) {
    REDUCE_MOTION.store(on, Ordering::Relaxed);
}

/// Whether motion should be suppressed (honours OS "reduce motion" prefs when
/// the host wires them in via [`set_reduce_motion`]).
pub fn reduce_motion() -> bool {
    REDUCE_MOTION.load(Ordering::Relaxed)
}

/// Convert a theme [`ThemeColor`] into a vello brush color.
#[inline]
pub fn brush(c: ThemeColor) -> VColor {
    VColor::new([c.r, c.g, c.b, c.a])
}

/// Convert a computed `taffy::Layout` box into a vello [`Rect`].
#[inline]
pub fn layout_rect(layout: &taffy::Layout) -> Rect {
    let x = layout.location.x as f64;
    let y = layout.location.y as f64;
    Rect::new(x, y, x + layout.size.width as f64, y + layout.size.height as f64)
}

/// Fill an axis-aligned rect with a theme color (the workhorse for the
/// rects-and-text-plus-vector cockpit widgets).
#[inline]
pub fn fill_rect(scene: &mut Scene, rect: Rect, color: ThemeColor) {
    scene.fill(Fill::NonZero, Affine::IDENTITY, brush(color), None, &rect);
}

/// Convert a theme [`ThemeColor`] into an RGBA array (for [`WidgetText`] /
/// cosmic-text commands).
#[inline]
pub fn color_arr(c: ThemeColor) -> [f32; 4] {
    [c.r, c.g, c.b, c.a]
}

/// A positioned text run a widget wants drawn by the host's cosmic-text layer.
///
/// The vello overlay draws only vector visuals; glyphs are delegated to
/// cosmic-text (the renderer's authoritative text path). Widgets emit these so
/// the host can queue them as text commands at the widget's slot coordinates.
/// `x`/`y` are top-left in the same coordinate space as `paint` (physical px).
#[derive(Debug, Clone, PartialEq)]
pub struct WidgetText {
    /// The string to render (may be Arabic / RTL — cosmic-text shapes BiDi).
    pub text: String,
    /// Left edge of the text in physical px.
    pub x: f32,
    /// Top edge of the text in physical px.
    pub y: f32,
    /// Font size in px.
    pub size_px: f32,
    /// RGBA color.
    pub color: [f32; 4],
    /// Optional clip rect `(x, y, w, h)` in physical px. When set, the host
    /// renderer will cull glyphs that fall outside this region. Set this to the
    /// widget's layout rect so text never bleeds across panel boundaries.
    pub clip_rect: Option<(f32, f32, f32, f32)>,
}

impl WidgetText {
    /// Convenience constructor.
    pub fn new(text: impl Into<String>, x: f32, y: f32, size_px: f32, color: [f32; 4]) -> Self {
        Self { text: text.into(), x, y, size_px, color, clip_rect: None }
    }

    /// Attach a clip rect `(x, y, w, h)` so the host renderer culls glyphs
    /// outside this region. Chain after [`WidgetText::new`].
    pub fn with_clip(mut self, clip: (f32, f32, f32, f32)) -> Self {
        self.clip_rect = Some(clip);
        self
    }
}

/// A cockpit widget: composes itself into a vello [`Scene`] given its computed
/// `taffy::Layout` and the active [`SemanticColors`] theme. This is the single
/// trait every component implements.
pub trait ZaroxiWidget {
    /// Compositing layer (drives [`crate::WidgetTree`] paint order).
    fn layer(&self) -> WidgetLayer;

    /// Paint the widget's **vector** visuals into `scene` within `layout`,
    /// reading colors from `theme`. Implementations must respect
    /// [`reduce_motion`] for any animation. Text is *not* drawn here — see
    /// [`ZaroxiWidget::text_items`].
    fn paint(&self, scene: &mut Scene, layout: &taffy::Layout, theme: &SemanticColors);

    /// Positioned text runs this widget wants drawn by the cosmic-text layer.
    /// Default: none. Components with labels override this so the host can queue
    /// the text at the matching slot coordinates.
    fn text_items(&self, _layout: &taffy::Layout, _theme: &SemanticColors) -> Vec<WidgetText> {
        Vec::new()
    }

    /// Optional screen-reader / accessibility hint describing this widget.
    fn a11y_label(&self) -> Option<String> {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn layer_order_is_background_to_tooltip() {
        let mut layers = WidgetLayer::ALL.to_vec();
        layers.sort();
        assert_eq!(layers, WidgetLayer::ALL.to_vec());
        assert!(WidgetLayer::Background < WidgetLayer::Editor);
        assert!(WidgetLayer::DiffLayer < WidgetLayer::Gutter);
        assert!(WidgetLayer::Palette < WidgetLayer::Tooltip);
    }

    #[test]
    fn reduce_motion_toggles() {
        set_reduce_motion(true);
        assert!(reduce_motion());
        set_reduce_motion(false);
        assert!(!reduce_motion());
    }

    #[test]
    fn brush_preserves_channels() {
        let c = ThemeColor::from_rgba(0.1, 0.2, 0.3, 0.4);
        let v = brush(c);
        let comps = v.components;
        assert!((comps[0] - 0.1).abs() < 1e-6);
        assert!((comps[3] - 0.4).abs() < 1e-6);
    }

    #[test]
    fn layout_rect_maps_box() {
        let mut l = taffy::Layout::default();
        l.location = taffy::geometry::Point { x: 10.0, y: 20.0 };
        l.size = taffy::geometry::Size { width: 100.0, height: 40.0 };
        let r = layout_rect(&l);
        assert_eq!((r.x0, r.y0, r.x1, r.y1), (10.0, 20.0, 110.0, 60.0));
    }
}

impl WidgetLayer {
    /// All layers in paint order.
    pub const ALL: [WidgetLayer; 9] = [
        WidgetLayer::Background,
        WidgetLayer::Editor,
        WidgetLayer::DiffLayer,
        WidgetLayer::Gutter,
        WidgetLayer::Minimap,
        WidgetLayer::StatusBar,
        WidgetLayer::ActivityRail,
        WidgetLayer::Palette,
        WidgetLayer::Tooltip,
    ];
}
