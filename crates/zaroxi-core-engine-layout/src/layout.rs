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

use zaroxi_kernel_math::{Rect, Size};

/// Deterministic, debug-friendly shell layout projection.
///
/// All coordinates and sizes are in f32 pixels and are clamped to be
/// non-negative. The layout is defined in window-space with (0,0) at the top-left.
#[derive(Debug, Clone, PartialEq)]
pub struct ShellLayout {
    // Top-level regions
    pub window_size: Size,
    pub titlebar: Rect,
    pub sidebar: Rect,
    pub editor: Rect,
    pub ai_panel: Rect,
    pub status_bar: Rect,

    // Editor subregions (all coordinates are absolute window-space)
    pub editor_tab_bar: Rect,
    pub editor_breadcrumb_bar: Rect,
    pub editor_content: Rect,
    pub editor_bottom_panel: Rect,
}

impl ShellLayout {
    /// Compute a ShellLayout deterministically from integer window dimensions.
    ///
    /// This function is tolerant of very small window sizes: regions are clamped
    /// and redistributed so that no negative widths/heights are produced. For
    /// extremely constrained windows some side panels may reduce to zero width
    /// to preserve the editor content area.
    pub fn from_window_size(width: u32, height: u32) -> Self {
        let w = width as f32;
        let h = height as f32;

        // Design defaults (sensible, implementation-light values).
        const TITLEBAR_H: f32 = 28.0;
        const STATUS_H: f32 = 22.0;
        const SIDEBAR_W: f32 = 240.0;
        const AI_PANEL_W: f32 = 300.0;
        const TABBAR_H: f32 = 32.0;
        const BREADCRUMB_H: f32 = 24.0;
        const EDITOR_BOTTOM_H: f32 = 160.0;
        const MIN_EDITOR_CONTENT_W: f32 = 80.0;
        const MIN_EDITOR_CONTENT_H: f32 = 40.0;

        // Clamp the full window size to non-negative (defensive).
        let W = w.max(0.0);
        let H = h.max(0.0);

        // Titlebar and status heights cannot exceed total height.
        let title_h = TITLEBAR_H.min(H);
        let status_h = STATUS_H.min((H - title_h).max(0.0));

        // Height available for the central editor stack.
        let editor_stack_h = (H - title_h - status_h).max(0.0);

        // Editor subregion heights: tab + breadcrumb + bottom + content (content gets remainder).
        let tab_h = TABBAR_H.min(editor_stack_h);
        let breadcrumb_h = BREADCRUMB_H.min((editor_stack_h - tab_h).max(0.0));
        // Reserve desired bottom panel, but clamp to remaining space.
        let bottom_h = EDITOR_BOTTOM_H.min((editor_stack_h - tab_h - breadcrumb_h).max(0.0));

        // Content receives the remainder (may be zero).
        let content_h = (editor_stack_h - tab_h - breadcrumb_h - bottom_h).max(0.0);

        // Width allocation:
        // Start with preferred sidebar and AI widths, clamp against total width.
        let mut sidebar_w = SIDEBAR_W.min(W);
        let mut ai_w = AI_PANEL_W.min((W - sidebar_w).max(0.0));
        let mut editor_w = (W - sidebar_w - ai_w).max(0.0);

        // If editor width would be too small, try to preserve a minimum editor content width
        // by shrinking side panels proportionally. If impossible, collapse side panels.
        if editor_w < MIN_EDITOR_CONTENT_W {
            let required_for_panels = (sidebar_w + ai_w);
            let avail_for_panels = (W - MIN_EDITOR_CONTENT_W).max(0.0);

            if avail_for_panels <= 0.0 {
                // Not enough room for side panels; collapse them to zero and give editor full width.
                sidebar_w = 0.0;
                ai_w = 0.0;
                editor_w = W;
            } else {
                // Shrink panels proportionally based on their desired sizes.
                let total_pref = SIDEBAR_W + AI_PANEL_W;
                if total_pref > 0.0 {
                    let sidebar_prop = SIDEBAR_W / total_pref;
                    sidebar_w = (avail_for_panels * sidebar_prop).min(SIDEBAR_W);
                    ai_w = (avail_for_panels * (1.0 - sidebar_prop)).min(AI_PANEL_W);
                    // Recompute editor width defensively.
                    editor_w = (W - sidebar_w - ai_w).max(0.0);
                }
            }
        }

        // Final safety clamps to avoid negative numbers.
        sidebar_w = sidebar_w.max(0.0);
        ai_w = ai_w.max(0.0);
        editor_w = editor_w.max(0.0);

        // Build Rects (absolute window-space).
        let titlebar = Rect::new(0.0, 0.0, W, title_h);
        let editor_y = title_h;
        let editor_h = editor_stack_h;
        let sidebar = Rect::new(0.0, editor_y, sidebar_w, editor_h);
        let editor = Rect::new(sidebar_w, editor_y, editor_w, editor_h);
        let ai_panel = Rect::new(sidebar_w + editor_w, editor_y, ai_w, editor_h);
        let status_bar = Rect::new(0.0, title_h + editor_h, W, status_h);

        // Editor subregions (absolute coords).
        let mut cursor_y = editor_y;
        let editor_tab_bar = Rect::new(sidebar_w, cursor_y, editor_w, tab_h);
        cursor_y += tab_h;
        let editor_breadcrumb_bar = Rect::new(sidebar_w, cursor_y, editor_w, breadcrumb_h);
        cursor_y += breadcrumb_h;
        let editor_content = Rect::new(sidebar_w, cursor_y, editor_w, content_h);
        cursor_y += content_h;
        let editor_bottom_panel = Rect::new(sidebar_w, cursor_y, editor_w, bottom_h);

        ShellLayout {
            window_size: Size::new(W, H),
            titlebar,
            sidebar,
            editor,
            ai_panel,
            status_bar,
            editor_tab_bar,
            editor_breadcrumb_bar,
            editor_content,
            editor_bottom_panel,
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
            format!("titlebar: {}", rfmt!(self.titlebar)),
            format!("sidebar: {}", rfmt!(self.sidebar)),
            format!("editor: {}", rfmt!(self.editor)),
            format!("ai_panel: {}", rfmt!(self.ai_panel)),
            format!("status_bar: {}", rfmt!(self.status_bar)),
            format!("editor_tab_bar: {}", rfmt!(self.editor_tab_bar)),
            format!("editor_breadcrumb_bar: {}", rfmt!(self.editor_breadcrumb_bar)),
            format!("editor_content: {}", rfmt!(self.editor_content)),
            format!("editor_bottom_panel: {}", rfmt!(self.editor_bottom_panel)),
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::ShellLayout;

    #[test]
    fn layout_large_window_has_expected_nonzero_editor() {
        let l = ShellLayout::from_window_size(1400, 900);
        // Editor region should be positive and fit within window bounds.
        assert!(l.editor.width > 200.0, "editor width too small: {}", l.editor.width);
        assert!(l.editor.height > 200.0, "editor height too small: {}", l.editor.height);
        // Subregions sum to the editor height (within floating point tolerance).
        let total = l.editor_tab_bar.height + l.editor_breadcrumb_bar.height + l.editor_content.height + l.editor_bottom_panel.height;
        let diff = (total - l.editor.height).abs();
        assert!(diff < 0.001, "editor subregion heights must sum to editor height; diff={}", diff);
    }

    #[test]
    fn layout_tiny_window_clamps_and_remains_nonnegative() {
        let l = ShellLayout::from_window_size(60, 40);
        // Everything must be non-negative and inside window bounds.
        assert!(l.window_size.width >= 0.0);
        assert!(l.window_size.height >= 0.0);
        for r in &[
            l.titlebar, l.sidebar, l.editor, l.ai_panel, l.status_bar,
            l.editor_tab_bar, l.editor_breadcrumb_bar, l.editor_content, l.editor_bottom_panel,
        ] {
            assert!(r.width >= 0.0 && r.height >= 0.0, "negative dimension found");
            assert!(r.x + r.width <= l.window_size.width + 0.01, "rect overflows window width");
            assert!(r.y + r.height <= l.window_size.height + 0.01, "rect overflows window height");
        }
    }
}
