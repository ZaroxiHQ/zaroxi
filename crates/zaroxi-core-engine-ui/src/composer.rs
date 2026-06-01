use crate::bar::Bar;
use crate::content::ContentView;
use zaroxi_core_engine_scene::{LabelPrimitive, RectPrimitive, WidgetScene};

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

/// Compose a set of generic `Bar` widgets into a single `WidgetScene`.
///
/// This is the preferred grouped composition API: it produces both background
/// rectangles and text labels in one call, returning a bundled `WidgetScene`
/// instead of forcing callers to coordinate two separate primitive vectors.
///
/// `rect_colors` and `label_colors` must each be parallel to `bars`.
pub fn compose_bars_scene(
    bars: &[Bar],
    rect_colors: &[[f32; 4]],
    label_colors: &[[f32; 4]],
) -> WidgetScene {
    let rects = compose_bars(bars, rect_colors);
    let labels = compose_bar_labels(bars, label_colors);
    WidgetScene::new(rects, labels)
}

/// Compose a `ContentView` into a `WidgetScene`.
///
/// Lays out title, subtitle, and code lines within the given region rect using
/// a simple vertical stack. The title receives `title_color`, everything else
/// uses `body_color`. Returns only label primitives (no background rects) —
/// the caller owns the panel background.
pub fn compose_content_view(
    region: &zaroxi_kernel_math::Rect,
    content: &ContentView,
    title_color: [f32; 4],
    body_color: [f32; 4],
) -> WidgetScene {
    let title_h: f32 = 18.0;
    let subtitle_h: f32 = 14.0;
    let line_h: f32 = 16.0;
    let pad_x: f32 = 10.0;
    let mut labels: Vec<LabelPrimitive> = Vec::new();
    let mut y = region.y + 4.0;

    // Title
    labels.push(LabelPrimitive::new(
        &content.title,
        region.x + pad_x,
        y,
        (region.width - pad_x * 2.0).max(0.0),
        title_h,
        title_color,
    ));
    y += title_h + 2.0;

    // Subtitle
    if !content.subtitle.is_empty() {
        labels.push(LabelPrimitive::new(
            &content.subtitle,
            region.x + pad_x,
            y,
            (region.width - pad_x * 2.0).max(0.0),
            subtitle_h,
            body_color,
        ));
        y += subtitle_h + 4.0;
    }

    // Code lines
    let max_lines = ((region.y + region.height - y) / line_h) as usize;
    for (i, line) in content.lines.iter().enumerate().take(max_lines) {
        if y + line_h > region.y + region.height {
            break;
        }
        labels.push(LabelPrimitive::new(
            line,
            region.x + pad_x,
            y,
            (region.width - pad_x * 2.0).max(0.0),
            line_h,
            body_color,
        ));
        y += line_h;
        let _ = i; // used only for iter position
    }

    WidgetScene::new(Vec::new(), labels)
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
