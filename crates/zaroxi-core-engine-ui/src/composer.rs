use crate::bar::Bar;
use crate::scene::{LabelPrimitive, RectPrimitive};
use zaroxi_core_engine_layout::ShellLayout;

/// Build a simple, deterministic shell UI composed of:
/// - background (full window)
/// - top bar (fixed height)
/// - left sidebar
/// - editor area
/// - status bar
///
/// Returns a stable vector of RectPrimitive in paint order (background first).
pub fn build_shell_ui(window_w: u32, window_h: u32) -> Vec<RectPrimitive> {
    // Compute deterministic shell layout using the existing layout crate.
    let layout = ShellLayout::from_window_size(window_w, window_h);

    let mut rects: Vec<RectPrimitive> = Vec::new();

    // background (full window) — paint first
    rects.push(RectPrimitive::new(
        0.0,
        0.0,
        layout.window_size.width,
        layout.window_size.height,
        [13.0 / 255.0, 14.0 / 255.0, 17.0 / 255.0, 1.0],
    ));

    // top bar color
    rects.push(RectPrimitive::new(
        layout.titlebar.x,
        layout.titlebar.y,
        layout.titlebar.width,
        layout.titlebar.height,
        [0.18, 0.18, 0.22, 1.0],
    ));

    // sidebar color
    rects.push(RectPrimitive::new(
        layout.sidebar.x,
        layout.sidebar.y,
        layout.sidebar.width,
        layout.sidebar.height,
        [0.12, 0.12, 0.14, 1.0],
    ));

    // editor area color
    rects.push(RectPrimitive::new(
        layout.editor.x,
        layout.editor.y,
        layout.editor.width,
        layout.editor.height,
        [0.08, 0.09, 0.11, 1.0],
    ));

    // status bar color
    rects.push(RectPrimitive::new(
        layout.status_bar.x,
        layout.status_bar.y,
        layout.status_bar.width,
        layout.status_bar.height,
        [0.15, 0.15, 0.17, 1.0],
    ));

    rects
}

/// Compose a set of generic `Bar` widgets into scene `RectPrimitive`s.
///
/// Each bar produces one filled rectangle covering its region. This is a
/// structural, engine-level conversion — it does not own colors or theme
/// logic. Callers supply the desired per-bar color by providing a matching
/// [`Vec<[f32; 4]>`] parallel to `bars`.
///
/// Returns primitives in paint order (same order as input).
pub fn compose_bars(bars: &[Bar], colors: &[[f32; 4]]) -> Vec<RectPrimitive> {
    bars.iter()
        .zip(colors.iter())
        .map(|(bar, &color)| {
            RectPrimitive::new(bar.rect.x, bar.rect.y, bar.rect.width, bar.rect.height, color)
        })
        .collect()
}

/// Compose a set of generic `Bar` widgets into scene `LabelPrimitive`s.
///
/// Each bar produces one positioned text label. The label is pinned to the
/// bar's top-left with a small inset padding; the remaining width and height
/// become the label's layout bounds. Colors are caller-supplied (theme
/// agnostic). Returns primitives in input order.
pub fn compose_bar_labels(bars: &[Bar], colors: &[[f32; 4]]) -> Vec<LabelPrimitive> {
    const INSET_X: f32 = 4.0;
    bars.iter()
        .zip(colors.iter())
        .map(|(bar, &color)| {
            LabelPrimitive::new(
                &bar.label,
                bar.rect.x + INSET_X,
                bar.rect.y,
                (bar.rect.width - 2.0 * INSET_X).max(0.0),
                bar.rect.height,
                color,
            )
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bar::Bar;
    use zaroxi_kernel_math::Rect;

    #[test]
    fn compose_bars_produces_one_primitive_per_bar() {
        let bars = vec![
            Bar::new("title", Rect::new(0.0, 0.0, 800.0, 30.0)),
            Bar::new("status", Rect::new(0.0, 570.0, 800.0, 28.0)),
        ];
        let colors = vec![[0.1, 0.1, 0.1, 1.0], [0.15, 0.15, 0.15, 1.0]];
        let prims = compose_bars(&bars, &colors);
        assert_eq!(prims.len(), 2, "one primitive per bar");
    }

    #[test]
    fn compose_bars_preserves_rect_and_color() {
        let rect = Rect::new(10.0, 20.0, 100.0, 30.0);
        let color = [0.2, 0.3, 0.4, 1.0];
        let bars = vec![Bar::new("test", rect)];
        let colors = vec![color];
        let prims = compose_bars(&bars, &colors);
        let p = &prims[0];
        assert_eq!(p.x, 10.0);
        assert_eq!(p.y, 20.0);
        assert_eq!(p.width, 100.0);
        assert_eq!(p.height, 30.0);
        assert_eq!(p.color, [0.2, 0.3, 0.4, 1.0]);
    }

    #[test]
    fn compose_bar_labels_includes_label_text() {
        let bars = vec![Bar::new("Ready", Rect::new(0.0, 0.0, 200.0, 28.0))];
        let colors = vec![[1.0, 1.0, 1.0, 1.0]];
        let labels = compose_bar_labels(&bars, &colors);
        assert_eq!(labels.len(), 1);
        assert_eq!(labels[0].label, "Ready");
    }

    #[test]
    fn compose_bar_labels_applies_inset() {
        let bars = vec![Bar::new("test", Rect::new(0.0, 10.0, 100.0, 20.0))];
        let colors = vec![[1.0; 4]];
        let labels = compose_bar_labels(&bars, &colors);
        let l = &labels[0];
        assert_eq!(l.x, 4.0, "inset from left");
        assert_eq!(l.y, 10.0, "same y as bar");
        assert_eq!(l.max_width, 92.0, "width minus 2*inset");
        assert_eq!(l.max_height, 20.0, "same height as bar");
    }
}
