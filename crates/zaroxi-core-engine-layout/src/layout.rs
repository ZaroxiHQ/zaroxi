#![allow(dead_code)]
//! Lightweight layout foundation for Zaroxi Studio (Phase 3).
//!
//! This module computes deterministic panel rectangles for the main desktop
//! shell using kernel geometry primitives (Rect/Size).
//!
//! Intent:
//! - Keep the algorithm light and resilient to evolving APIs.
//! - Provide a single, deterministic constructor `ShellLayout::from_window_size`
//!   that returns stable Rects for all major regions.
//! - Avoid negative sizes and clamp gracefully on very small windows.
//!
//! Consumers should treat these Rects as authoritative geometry and avoid
//! reinterpreting coordinates.
//!
//! Phase 38: Renamed fields to use app-neutral generic names.
//! - titlebar → top_bar
//! - sidebar → left_panel
//! - editor → center_panel
//! - ai_panel → right_panel
//! - status_bar → bottom_bar
//! - editor_tab_bar → content_tab_strip
//! - editor_breadcrumb_bar → content_breadcrumb
//! - editor_content → content_area
//! - editor_bottom_panel → bottom_panel

use zaroxi_kernel_math::{Rect, Size};

/// Deterministic, debug-friendly app layout projection.
///
/// All coordinates and sizes are in f32 pixels and are clamped to be
/// non-negative. The layout is defined in window-space with (0,0) at the top-left.
#[derive(Debug, Clone, PartialEq)]
pub struct ShellLayout {
    // Top-level regions
    pub window_size: Size,
    pub top_bar: Rect,
    pub left_panel: Rect,
    pub center_panel: Rect,
    pub right_panel: Rect,
    pub bottom_bar: Rect,

    // Center subregions (all coordinates are absolute window-space)
    pub content_tab_strip: Rect,
    pub content_breadcrumb: Rect,
    pub content_area: Rect,
    pub bottom_panel: Rect,
}

impl ShellLayout {
    /// Compute a ShellLayout deterministically from integer window dimensions.
    ///
    /// This function is tolerant of very small window sizes: regions are clamped
    /// and redistributed so that no negative widths/heights are produced. For
    /// extremely constrained windows some side panels may reduce to zero width
    /// to preserve the center content area.
    pub fn from_window_size(width: u32, height: u32) -> Self {
        let w = width as f32;
        let h = height as f32;

        // Design defaults (implementation-light values).
        const TOP_BAR_H: f32 = 30.0;
        const BOTTOM_BAR_H: f32 = 28.0;
        const LEFT_PANEL_W: f32 = 260.0;
        const RIGHT_PANEL_W: f32 = 320.0;
        const TAB_STRIP_H: f32 = 28.0;
        const BREADCRUMB_H: f32 = 20.0;
        const BOTTOM_PANEL_H: f32 = 150.0;
        const MIN_CONTENT_AREA_W: f32 = 80.0;
        const MIN_CONTENT_AREA_H: f32 = 40.0;

        // Clamp the full window size to non-negative (defensive).
        let w_avail = w.max(0.0);
        let h_avail = h.max(0.0);

        // Top bar and bottom bar heights cannot exceed total height.
        let top_h = TOP_BAR_H.min(h_avail);
        let bottom_h = BOTTOM_BAR_H.min((h_avail - top_h).max(0.0));

        // Height available for the central stack.
        let center_stack_h = (h_avail - top_h - bottom_h).max(0.0);

        // Center subregion heights: tab + breadcrumb + bottom + content (content gets remainder).
        let tab_h = TAB_STRIP_H.min(center_stack_h);
        let breadcrumb_h = BREADCRUMB_H.min((center_stack_h - tab_h).max(0.0));
        // Reserve desired bottom panel, but clamp to remaining space.
        let bottom_panel_h = BOTTOM_PANEL_H.min((center_stack_h - tab_h - breadcrumb_h).max(0.0));

        // Content receives the remainder (may be zero).
        let content_h = (center_stack_h - tab_h - breadcrumb_h - bottom_panel_h).max(0.0);

        // Width allocation:
        // Start with preferred left and right panel widths, clamp against total width.
        let mut left_w = LEFT_PANEL_W.min(w_avail);
        let mut right_w = RIGHT_PANEL_W.min((w_avail - left_w).max(0.0));
        let mut center_w = (w_avail - left_w - right_w).max(0.0);

        // If center width would be too small, try to preserve a minimum content width
        // by shrinking side panels proportionally. If impossible, collapse side panels.
        if center_w < MIN_CONTENT_AREA_W {
            let _required_for_panels = left_w + right_w;
            let avail_for_panels = (w_avail - MIN_CONTENT_AREA_W).max(0.0);

            if avail_for_panels <= 0.0 {
                // Not enough room for side panels; collapse them to zero and give center full width.
                left_w = 0.0;
                right_w = 0.0;
                center_w = w_avail;
            } else {
                // Shrink panels proportionally based on their desired sizes.
                let total_pref = LEFT_PANEL_W + RIGHT_PANEL_W;
                if total_pref > 0.0 {
                    let left_prop = LEFT_PANEL_W / total_pref;
                    left_w = (avail_for_panels * left_prop).min(LEFT_PANEL_W);
                    right_w = (avail_for_panels * (1.0 - left_prop)).min(RIGHT_PANEL_W);
                    // Recompute center width defensively.
                    center_w = (w_avail - left_w - right_w).max(0.0);
                }
            }
        }

        // Final safety clamps to avoid negative numbers.
        left_w = left_w.max(0.0);
        right_w = right_w.max(0.0);
        center_w = center_w.max(0.0);

        // Build Rects (absolute window-space).
        let top_bar = Rect::new(0.0, 0.0, w_avail, top_h);
        let center_y = top_h;
        let center_h = center_stack_h;
        let left_panel = Rect::new(0.0, center_y, left_w, center_h);
        let center_panel = Rect::new(left_w, center_y, center_w, center_h);
        let right_panel = Rect::new(left_w + center_w, center_y, right_w, center_h);
        let bottom_bar = Rect::new(0.0, top_h + center_h, w_avail, bottom_h);

        // Center subregions (absolute coords).
        let mut cursor_y = center_y;
        let content_tab_strip = Rect::new(left_w, cursor_y, center_w, tab_h);
        cursor_y += tab_h;
        let content_breadcrumb = Rect::new(left_w, cursor_y, center_w, breadcrumb_h);
        cursor_y += breadcrumb_h;
        let content_area = Rect::new(left_w, cursor_y, center_w, content_h);
        cursor_y += content_h;
        let bottom_panel = Rect::new(left_w, cursor_y, center_w, bottom_panel_h);

        ShellLayout {
            window_size: Size::new(w_avail, h_avail),
            top_bar,
            left_panel,
            center_panel,
            right_panel,
            bottom_bar,
            content_tab_strip,
            content_breadcrumb,
            content_area,
            bottom_panel,
        }
    }

    /// Small helper to create a deterministic, multi-line debug summary.
    pub fn to_debug_lines(&self) -> Vec<String> {
        macro_rules! rfmt {
            ($r:expr) => {
                format!("x={} y={} w={} h={}", $r.x, $r.y, $r.width, $r.height)
            };
        }
        vec![
            format!("window: {}x{}", self.window_size.width, self.window_size.height),
            format!("top_bar: {}", rfmt!(self.top_bar)),
            format!("left_panel: {}", rfmt!(self.left_panel)),
            format!("center_panel: {}", rfmt!(self.center_panel)),
            format!("right_panel: {}", rfmt!(self.right_panel)),
            format!("bottom_bar: {}", rfmt!(self.bottom_bar)),
            format!("content_tab_strip: {}", rfmt!(self.content_tab_strip)),
            format!("content_breadcrumb: {}", rfmt!(self.content_breadcrumb)),
            format!("content_area: {}", rfmt!(self.content_area)),
            format!("bottom_panel: {}", rfmt!(self.bottom_panel)),
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::ShellLayout;

    #[test]
    fn layout_large_window_has_expected_nonzero_content() {
        let l = ShellLayout::from_window_size(1400, 900);
        // Center region should be positive and fit within window bounds.
        assert!(l.center_panel.width > 200.0, "center width too small: {}", l.center_panel.width);
        assert!(
            l.center_panel.height > 200.0,
            "center height too small: {}",
            l.center_panel.height
        );
        // Subregions sum to the center height (within floating point tolerance).
        let total = l.content_tab_strip.height
            + l.content_breadcrumb.height
            + l.content_area.height
            + l.bottom_panel.height;
        let diff = (total - l.center_panel.height).abs();
        assert!(diff < 0.001, "center subregion heights must sum to center height; diff={}", diff);
    }
}

/// Build a simple, deterministic app layout from window dimensions.
///
/// Computes a `ShellLayout` and converts each major region into a colored
/// `RectPrimitive` in paint order (background first).
/// Colors come from host-supplied `StyleTokens`; the engine makes no visual decisions.
pub fn build_shell_ui(
    window_w: u32,
    window_h: u32,
    tokens: &zaroxi_core_engine_style::StyleTokens,
) -> Vec<zaroxi_core_engine_scene::RectPrimitive> {
    use zaroxi_core_engine_scene::RectPrimitive;
    let layout = super::ShellLayout::from_window_size(window_w, window_h);

    let mut rects: Vec<RectPrimitive> = Vec::new();

    // App background
    rects.push(RectPrimitive::new(
        0.0,
        0.0,
        layout.window_size.width,
        layout.window_size.height,
        tokens.app_background.to_array(),
    ));

    // Top bar
    rects.push(RectPrimitive::new(
        layout.top_bar.x,
        layout.top_bar.y,
        layout.top_bar.width,
        layout.top_bar.height,
        tokens.titlebar_background.to_array(),
    ));

    // Left panel (sidebar)
    rects.push(RectPrimitive::new(
        layout.left_panel.x,
        layout.left_panel.y,
        layout.left_panel.width,
        layout.left_panel.height,
        tokens.sidebar_background.to_array(),
    ));

    // Center panel (editor area)
    rects.push(RectPrimitive::new(
        layout.center_panel.x,
        layout.center_panel.y,
        layout.center_panel.width,
        layout.center_panel.height,
        tokens.editor_content_background.to_array(),
    ));

    // Bottom bar (status bar)
    rects.push(RectPrimitive::new(
        layout.bottom_bar.x,
        layout.bottom_bar.y,
        layout.bottom_bar.width,
        layout.bottom_bar.height,
        tokens.status_bar_background.to_array(),
    ));

    rects
}
