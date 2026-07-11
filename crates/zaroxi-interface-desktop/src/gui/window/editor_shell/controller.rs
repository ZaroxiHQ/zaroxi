/*!
Editor Phase 1 — Layout controller.

Manages the Taffy layout lifecycle:
- Caches the last layout result and its window dimensions
- Skips recomputation when input dimensions are unchanged
- Provides a single entry point for layout computation
- Bridges the Taffy output to the types expected by `frame.rs`, `renderbridge.rs`,
  and `app.rs` (ShellRegion, ShellLayout, etc.)
*/

use crate::gui::{Rect, ShellRegion, Size};
use zaroxi_interface_theme::theme::ZaroxiTheme;

use super::layout::{EditorShellLayout, compute_layout};
use super::view::EditorViewport;

/// Cached layout state to avoid recomputation on unchanged window dimensions.
pub struct ShellLayoutController {
    last_width: u32,
    last_height: u32,
    cached_layout: Option<EditorShellLayout>,
    cached_viewport: Option<EditorViewport>,
    cached_shell_regions: Option<Vec<ShellRegion>>,
    cached_shell_layout: Option<zaroxi_core_engine_ui::ShellLayout>,
    cached_size: Option<Size>,
}

impl ShellLayoutController {
    pub fn new() -> Self {
        Self {
            last_width: 0,
            last_height: 0,
            cached_layout: None,
            cached_viewport: None,
            cached_shell_regions: None,
            cached_shell_layout: None,
            cached_size: None,
        }
    }

    /// Compute or retrieve the cached layout for the given window size.
    /// Only recomputes Taffy if the dimensions have changed.
    pub fn get_or_compute(
        &mut self,
        width: u32,
        height: u32,
        theme: ZaroxiTheme,
    ) -> &EditorShellLayout {
        if width != self.last_width || height != self.last_height {
            self.recompute(width, height, theme);
        }
        self.cached_layout.as_ref().unwrap()
    }

    /// Get the cached editor viewport.
    pub fn viewport(&self) -> &EditorViewport {
        self.cached_viewport.as_ref().unwrap()
    }

    /// Get the cached shell regions (compatible with frame.rs).
    pub fn shell_regions(&self) -> &[ShellRegion] {
        self.cached_shell_regions.as_deref().unwrap_or(&[])
    }

    /// Get the cached ShellLayout (compatible with widget tree builder).
    pub fn engine_shell_layout(&self) -> &zaroxi_core_engine_ui::ShellLayout {
        self.cached_shell_layout.as_ref().unwrap()
    }

    /// Get the cached size.
    pub fn size(&self) -> &Size {
        self.cached_size.as_ref().unwrap()
    }

    // ── Internal recompute ──

    fn recompute(&mut self, width: u32, height: u32, _theme: ZaroxiTheme) {
        let w = width as f32;
        let h = height as f32;
        let layout = compute_layout(w, h);

        let viewport = EditorViewport::from_content_rect(layout.editor_content_rect);
        let shell_regions = build_shell_regions_from_layout(&layout);
        let engine_layout = build_engine_shell_layout(&layout);
        let size = Size { width, height };

        self.last_width = width;
        self.last_height = height;
        self.cached_layout = Some(layout);
        self.cached_viewport = Some(viewport);
        self.cached_shell_regions = Some(shell_regions);
        self.cached_shell_layout = Some(engine_layout);
        self.cached_size = Some(size);
    }
}

impl Default for ShellLayoutController {
    fn default() -> Self {
        Self::new()
    }
}

// ── Conversion: EditorShellLayout → ShellRegion[] ──────────────────────

/// Build the canonical shell region list from a computed layout.
///
/// This is the **single source of truth** for shell regions: the live render
/// loop reads it via [`ShellLayoutController::shell_regions`], and `ShellFrame`
/// delegates to it too, so there is exactly one region builder (no duplicated
/// "second shell").
pub(crate) fn build_shell_regions_from_layout(l: &EditorShellLayout) -> Vec<ShellRegion> {
    let tr = |r: (f32, f32, f32, f32)| Rect {
        x: r.0.max(0.0) as u32,
        y: r.1.max(0.0) as u32,
        width: r.2.max(0.0) as u32,
        height: r.3.max(0.0) as u32,
    };

    let regions = vec![
        ShellRegion { id: "toolbar", name: "editor_header_toolbar", rect: tr(l.toolbar_rect) },
        ShellRegion { id: "app_rail", name: "app_rail", rect: tr(l.rail_rect) },
        ShellRegion { id: "sidebar", name: "sidebar", rect: tr(l.sidebar_rect) },
        ShellRegion { id: "editor_tabs", name: "editor_tabs", rect: tr(l.editor_tabs_rect) },
        ShellRegion { id: "breadcrumb", name: "breadcrumb", rect: tr(l.breadcrumb_rect) },
        ShellRegion { id: "gutter_lane", name: "gutter_lane", rect: tr(l.gutter_rect) },
        ShellRegion {
            id: "editor_content",
            name: "editor_content",
            rect: tr(l.editor_content_rect),
        },
        // No legacy minimap_lane region: the overview/minimap surface is owned by
        // the cockpit/widget layer (editor-edge), not a dead shell sibling column.
        ShellRegion {
            id: "center_bottom_panel",
            name: "center_bottom_panel",
            rect: tr(l.terminal_rect),
        },
        ShellRegion {
            id: "bottom_dock",
            name: "bottom_dock",
            rect: Rect { x: 0, y: 0, width: 0, height: 0 },
        },
        ShellRegion {
            id: "ai_panel_header",
            name: "ai_panel_header",
            rect: tr(l.assistant_header_rect),
        },
        ShellRegion {
            id: "ai_panel_content",
            name: "ai_panel_content",
            rect: tr(l.assistant_content_rect),
        },
        ShellRegion { id: "status_bar", name: "status_bar", rect: tr(l.status_bar_rect) },
    ];

    // Ownership instrumentation. This builder is the single, canonical region
    // source (live loop + ShellFrame), and runs on each layout (re)compute, so
    // it is the authoritative place to make default-vs-fallback ownership explicit.
    let legacy = crate::gui::window::cockpit::legacy_shell_surfaces();
    let cockpit_flag = crate::gui::window::cockpit::cockpit_enabled();
    let status_owner = if legacy { "legacy" } else { "cockpit" };
    let overview_owner = if legacy { "none" } else { "cockpit" };

    if std::env::var("ZAROXI_LAYOUT_TRACE").as_deref() == Ok("1") {
        eprintln!(
            "ZAROXI_LAYOUT_TRACE: window={}x{} status_owner={} overview_owner={} legacy_fallback_enabled={} cockpit_flag={} minimap_lane_reserved=false",
            l.window_size.0 as u32,
            l.window_size.1 as u32,
            status_owner,
            overview_owner,
            legacy,
            cockpit_flag,
        );
        for r in &regions {
            let role = crate::gui::region_dispatch::region_role(r.id);
            eprintln!(
                "ZAROXI_LAYOUT_TRACE:   id={:<20} role={:?} rect=(x={} y={} w={} h={})",
                r.id, role, r.rect.x, r.rect.y, r.rect.width, r.rect.height,
            );
        }
    }
    if std::env::var("ZAROXI_MINIMAP_TRACE").as_deref() == Ok("1") {
        eprintln!(
            "ZAROXI_MINIMAP_TRACE: overview_owner={} minimap_lane_reserved=false editor_overview_nested={} legacy_fallback_enabled={}",
            overview_owner, !legacy, legacy,
        );
    }

    regions
}

// ── Conversion: EditorShellLayout → ShellLayout (engine) ───────────────

fn build_engine_shell_layout(l: &EditorShellLayout) -> zaroxi_core_engine_ui::ShellLayout {
    use zaroxi_kernel_math::{Rect, Size};

    let r = |tuple: (f32, f32, f32, f32)| Rect {
        x: tuple.0,
        y: tuple.1,
        width: tuple.2,
        height: tuple.3,
    };

    let left_panel_r = Rect {
        x: l.sidebar_rect.0,
        y: l.sidebar_rect.1,
        width: l.sidebar_rect.2.max(l.rail_rect.2),
        height: l.sidebar_rect.3 + l.rail_rect.3,
    };

    let right_panel_r = Rect {
        x: l.assistant_rect.0,
        y: l.assistant_rect.1,
        width: l.assistant_rect.2,
        height: l.assistant_rect.3,
    };

    let center_panel_r = Rect {
        x: left_panel_r.x + left_panel_r.width,
        y: l.editor_tabs_rect.1,
        width: right_panel_r.x - (left_panel_r.x + left_panel_r.width),
        height: l.editor_tabs_rect.3
            + l.breadcrumb_rect.3
            + l.editor_content_rect.3
            + l.terminal_rect.3,
    };

    zaroxi_core_engine_ui::ShellLayout {
        window_size: Size::new(l.window_size.0, l.window_size.1),
        top_bar: r(l.toolbar_rect),
        left_panel: left_panel_r,
        center_panel: center_panel_r,
        right_panel: right_panel_r,
        bottom_bar: r(l.status_bar_rect),
        content_tab_strip: r(l.editor_tabs_rect),
        content_breadcrumb: r(l.breadcrumb_rect),
        content_area: r(l.editor_content_rect),
        bottom_panel: r(l.terminal_rect),
    }
}

// ── Tests ──────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use zaroxi_interface_theme::theme::ZaroxiTheme;

    #[test]
    fn layout_is_deterministic() {
        let l1 = compute_layout(1200.0, 800.0);
        let l2 = compute_layout(1200.0, 800.0);
        assert_eq!(l1.editor_content_rect, l2.editor_content_rect);
        assert_eq!(l1.toolbar_rect, l2.toolbar_rect);
    }

    #[test]
    fn editor_has_minimum_width() {
        let l = compute_layout(600.0, 400.0);
        assert!(
            l.editor_content_rect.2 > 0.0,
            "editor content w={} should be positive at 600px",
            l.editor_content_rect.2
        );
    }

    #[test]
    fn editor_has_minimum_width_at_very_narrow() {
        let l = compute_layout(400.0, 400.0);
        assert!(
            l.editor_content_rect.2 >= 40.0,
            "editor content w={} < 40 at 400px",
            l.editor_content_rect.2
        );
    }

    #[test]
    fn side_panels_collapse_at_narrow() {
        let l = compute_layout(500.0, 400.0);
        assert!(
            l.sidebar_rect.2 < 200.0,
            "sidebar w={} should shrink below 200 at 500px",
            l.sidebar_rect.2
        );
        assert!(
            l.assistant_rect.2 < 300.0,
            "assistant w={} should collapse at 500px",
            l.assistant_rect.2
        );
    }

    #[test]
    fn full_layout_at_wide() {
        let l = compute_layout(1400.0, 900.0);
        assert!(l.sidebar_rect.2 >= 250.0, "sidebar should be near full at 1400px");
        assert!(l.assistant_rect.2 >= 350.0, "assistant should be near full at 1400px");
    }

    #[test]
    fn minimap_collapses_before_editor() {
        let l = compute_layout(800.0, 600.0);
        assert!(l.editor_content_rect.2 > 0.0, "editor must have positive width at 800px");
    }

    #[test]
    fn rail_never_collapses() {
        for w in [400u32, 500, 700, 900, 1200] {
            let l = compute_layout(w as f32, 400.0);
            assert!(
                l.rail_rect.3 >= 30.0,
                "rail h={} should stay >= 30 at width {}",
                l.rail_rect.3,
                w
            );
        }
    }

    #[test]
    fn controller_caches_layout() {
        let mut ctrl = ShellLayoutController::new();
        let r1 = ctrl.get_or_compute(1200, 800, ZaroxiTheme::Dark).editor_content_rect;
        let r2 = ctrl.get_or_compute(1200, 800, ZaroxiTheme::Dark).editor_content_rect;
        assert_eq!(r1, r2);
    }

    #[test]
    fn controller_recomputes_on_resize() {
        let mut ctrl = ShellLayoutController::new();
        let r1 = ctrl.get_or_compute(1200, 800, ZaroxiTheme::Dark).editor_content_rect;
        let r2 = ctrl.get_or_compute(800, 600, ZaroxiTheme::Dark).editor_content_rect;
        assert_ne!(r1, r2);
    }

    #[test]
    fn shell_regions_match_layout_rects() {
        let mut ctrl = ShellLayoutController::new();
        let (ex, ey, ew, eh) =
            ctrl.get_or_compute(1200, 800, ZaroxiTheme::Dark).editor_content_rect;
        let regions = ctrl.shell_regions();
        let editor_region = regions.iter().find(|r| r.id == "editor_content").unwrap();
        assert_eq!(editor_region.rect.x, ex.max(0.0) as u32);
        assert_eq!(editor_region.rect.y, ey.max(0.0) as u32);
        assert_eq!(editor_region.rect.width, ew.max(0.0) as u32);
        assert_eq!(editor_region.rect.height, eh.max(0.0) as u32);
    }

    #[test]
    fn viewport_clip_is_inset_from_content() {
        let vp = EditorViewport::from_content_rect((100.0, 50.0, 500.0, 300.0));
        assert!(vp.clip_rect.0 > vp.content_rect.0, "clip x should be inset");
        assert!(vp.clip_rect.1 > vp.content_rect.1, "clip y should be inset");
        assert!(vp.clip_rect.2 < vp.content_rect.2, "clip w should be smaller");
        assert!(vp.clip_rect.3 < vp.content_rect.3, "clip h should be smaller");
    }

    #[test]
    fn viewport_point_containment() {
        let vp = EditorViewport::from_content_rect((100.0, 50.0, 500.0, 300.0));
        assert!(vp.contains_point(150.0, 100.0));
        assert!(!vp.contains_point(50.0, 100.0));
        assert!(vp.clip_contains_point(120.0, 60.0));
        assert!(!vp.clip_contains_point(101.0, 51.0));
    }
}
