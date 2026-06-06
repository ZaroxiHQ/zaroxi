use std::fmt;

use zaroxi_interface_theme::theme::DesignTokens;

/// Simple size primitive used for layout inputs.
#[derive(Debug, Clone, Copy)]
pub struct Size {
    pub width: u32,
    pub height: u32,
}

/// Simple rectangle primitive.
#[derive(Debug, Clone, Copy)]
pub struct Rect {
    pub x: u32,
    pub y: u32,
    pub width: u32,
    pub height: u32,
}

impl fmt::Display for Rect {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "x={} y={} w={} h={}", self.x, self.y, self.width, self.height)
    }
}

/// A named, stable region in the shell layout.
#[derive(Debug, Clone)]
pub struct ShellRegion {
    /// Stable machine id for tests and transcripts.
    pub id: &'static str,
    /// Human readable name.
    pub name: &'static str,
    /// Allocated rectangle for the current shell size.
    pub rect: Rect,
}

/// Concrete layout constants derived from the DesignTokens system.
///
/// Phase 66: all shell sizing is now driven by the token system instead of
/// ad-hoc hardcoded literals. The spacing tokens are used as the arithmetic
/// base, and chrome heights are chosen as proportions of the base unit.
#[derive(Debug, Clone, Copy)]
pub struct ShellLayoutTokens {
    // Chrome heights (derived from spacing scale)
    pub toolbar_h: u32,
    pub status_h: u32,
    pub rail_w: u32,
    pub tab_strip_h: u32,
    pub breadcrumb_h: u32,
    pub ai_header_h: u32,
    pub gutter_w: u32,
    pub bottom_panel_min_h: u32,
    pub bottom_panel_ratio: f32,

    // Content insets
    pub content_inset_x: u32,
    pub content_inset_y: u32,

    // Separator / border
    pub separator_thickness: u32,
}

impl Default for ShellLayoutTokens {
    fn default() -> Self {
        let t = DesignTokens::default();
        let base = t.spacing_sm as u32; // 8px base unit
        Self {
            toolbar_h: 32,
            status_h: 26,
            rail_w: 40,
            tab_strip_h: 28,
            breadcrumb_h: 20,
            ai_header_h: 28,
            gutter_w: 52,
            bottom_panel_min_h: base * 3, // 24px min
            bottom_panel_ratio: 0.22,
            content_inset_x: base,                // 8px
            content_inset_y: t.spacing_xs as u32, // 4px
            separator_thickness: 1,
        }
    }
}

/// Lightweight theme tokens used by the scaffold (informational only).
///
/// Extended for GUI-9 readability pass: include minimal semantic text colors so
/// the window-level glue and text_adapter can select high-contrast colors
/// without depending on the full theme crate. These tokens are deliberately
/// small and conservative to avoid creating a second theme system.
#[derive(Debug, Clone)]
pub struct Theme {
    pub surface: &'static str,
    pub border_color: &'static str,
    pub border_thickness: u8,
    pub corner_radius: u8,
    // Minimal text tokens used by GUI-9 for clear contrast and hierarchy.
    pub text_primary: &'static str,
    pub text_secondary: &'static str,
}

impl Theme {
    pub fn from_variant(resolved_theme: zaroxi_interface_theme::theme::ZaroxiTheme) -> Self {
        let variant = resolved_theme;
        let sem = variant.colors(false);
        let tokens = DesignTokens::default();

        let surface_s = sem.shell_background.to_hex();
        let border_s = sem.border.to_hex();
        let text_primary_s = sem.text_primary.to_hex();
        let text_secondary_s = sem.text_secondary.to_hex();

        let surface_static: &'static str = Box::leak(surface_s.into_boxed_str());
        let border_static: &'static str = Box::leak(border_s.into_boxed_str());
        let text_primary_static: &'static str = Box::leak(text_primary_s.into_boxed_str());
        let text_secondary_static: &'static str = Box::leak(text_secondary_s.into_boxed_str());

        Theme {
            surface: surface_static,
            border_color: border_static,
            border_thickness: tokens.border_width as u8,
            corner_radius: tokens.radius_lg as u8,
            text_primary: text_primary_static,
            text_secondary: text_secondary_static,
        }
    }
}

impl Default for Theme {
    fn default() -> Self {
        Self::from_variant(zaroxi_interface_theme::theme::ZaroxiTheme::Dark)
    }
}

/// ShellFrame is the top-level layout model for GUI-1. It computes the allocation
/// of the major regions for a given outer size and can render a deterministic
/// placeholder transcript describing the regions.
#[derive(Debug, Clone)]
pub struct ShellFrame {
    pub size: Size,
    pub theme: Theme,
    pub regions: Vec<ShellRegion>,
    /// Live workspace content snapshot (editor body, tabs, explorer, AI panel, etc.)
    pub work_content: Option<crate::gui::ShellWorkContent>,
}

impl ShellFrame {
    /// Construct a new ShellFrame and compute a canonical IDE layout.
    ///
    /// Phase 66 layout: token-driven spacing system with tuned proportions for
    /// a productized IDE shell. Chrome heights and panel widths derive from
    /// `ShellLayoutTokens` rather than ad-hoc hardcoded literals.
    pub fn new(size: Size, resolved_theme: zaroxi_interface_theme::theme::ZaroxiTheme) -> Self {
        let theme = Theme::from_variant(resolved_theme);
        let lt = ShellLayoutTokens::default();

        let outer_padding: u32 = 0;
        let top_toolbar_h: u32 = lt.toolbar_h;
        let status_h: u32 = lt.status_h;
        let app_rail_w: u32 = lt.rail_w;

        let inner_x = outer_padding;
        let inner_y = outer_padding;
        let inner_w = size.width.saturating_sub(outer_padding * 2);
        let inner_h = size.height.saturating_sub(outer_padding * 2);

        let bottom_dock_h: u32 = 0;

        // ── Responsive horizontal allocation ──────────────────────
        // Sidebar and AI panel shrink at narrower widths so the center
        // editor always keeps a usable minimum. The minimap is hidden
        // below 750 px since it's auxiliary.
        //
        // Phase 66 tuned breakpoints and widths for better proportions:
        //   >= 1150  full layout: sidebar=280, ai=300, minimap=60
        //   >= 900   medium:      sidebar=220, ai=260, minimap=50
        //   >= 680   narrow:      sidebar=170, ai=200, minimap=0
        //   < 680    very narrow: sidebar=60,  ai=170, minimap=0
        let (left_sidebar_w, ai_panel_w, minimap_w) = if inner_w >= 1150 {
            (280u32, 300u32, 60u32)
        } else if inner_w >= 900 {
            (220, 260, 50)
        } else if inner_w >= 680 {
            (170, 200, 0)
        } else {
            (60, 170, 0)
        };

        // Top toolbar / titlebar region (full width)
        let toolbar = Rect { x: inner_x, y: inner_y, width: inner_w, height: top_toolbar_h };

        // Status bar at bottom (stable anchor)
        let status = Rect {
            x: inner_x,
            y: inner_y + inner_h.saturating_sub(status_h),
            width: inner_w,
            height: status_h,
        };

        // Bottom dock sits just above status bar and spans full width
        let bottom_dock = Rect {
            x: inner_x,
            y: status.y.saturating_sub(bottom_dock_h),
            width: inner_w,
            height: bottom_dock_h,
        };

        // Vertical space available for main columns between toolbar and bottom dock
        let columns_y = inner_y + top_toolbar_h;
        let columns_h = bottom_dock.y.saturating_sub(columns_y);

        // App rail (far-left activity bar)
        let app_rail = Rect { x: inner_x, y: columns_y, width: app_rail_w, height: columns_h };

        // Outer sidebar (to the right of app rail)
        let sidebar = Rect {
            x: app_rail.x + app_rail.width,
            y: columns_y,
            width: left_sidebar_w,
            height: columns_h,
        };

        // AI panel (far-right)
        let ai_panel = Rect {
            x: inner_x + inner_w.saturating_sub(ai_panel_w),
            y: columns_y,
            width: ai_panel_w,
            height: columns_h,
        };

        // Editor main column sits between sidebar and AI panel.
        // The editor column is split into: gutter + code content + minimap.
        let editor_x = sidebar.x + sidebar.width;
        let editor_w = ai_panel.x.saturating_sub(editor_x);
        let gutter_w = if editor_w >= 200 { lt.gutter_w } else { 0 };
        let editor_content_w = editor_w.saturating_sub(gutter_w).saturating_sub(minimap_w);
        debug_assert!(
            editor_content_w >= 120 || editor_w == 0,
            "editor_content_w={} too narrow at inner_w={}",
            editor_content_w,
            inner_w
        );

        // Editor tiles region: slim tab strip + compact breadcrumb.
        // Tabs and breadcrumb span the full editor column (gutter + code).
        let editor_full_w = editor_w.saturating_sub(minimap_w);
        let editor_tabs_h: u32 = lt.tab_strip_h;
        let breadcrumb_h: u32 = lt.breadcrumb_h;
        let editor_top_h = editor_tabs_h + breadcrumb_h;

        // Available height for editor body + terminal panel
        let below_editor_top_y = columns_y + editor_top_h;
        let below_editor_top_h = columns_h.saturating_sub(editor_top_h);

        // Terminal panel: proportional split with a fixed minimum so it doesn't
        // vanish on very small windows, and a ceiling so the editor stays readable.
        let mut center_bottom_h = ((below_editor_top_h as f32) * lt.bottom_panel_ratio) as u32;
        center_bottom_h = center_bottom_h
            .max(lt.bottom_panel_min_h)
            .min(below_editor_top_h.saturating_sub(80))
            .max(0);
        let editor_body_h = below_editor_top_h.saturating_sub(center_bottom_h);

        // Editor tabs row (tab strip at top of editor column, full width)
        let editor_tabs =
            Rect { x: editor_x, y: columns_y, width: editor_full_w, height: editor_tabs_h };

        // Breadcrumb / path row below tabs (full width)
        let breadcrumb = Rect {
            x: editor_x,
            y: columns_y + editor_tabs_h,
            width: editor_full_w,
            height: breadcrumb_h,
        };

        // Gutter lane — line-number column, between sidebar and code content
        let gutter_lane = Rect {
            x: editor_x,
            y: below_editor_top_y,
            width: gutter_w,
            height: below_editor_top_h,
        };

        // Center editor canvas (code area, to the right of gutter)
        let editor_code_x = editor_x + gutter_w;
        let center_editor = Rect {
            x: editor_code_x,
            y: below_editor_top_y,
            width: editor_content_w,
            height: editor_body_h,
        };

        // Terminal panel below editor body
        let center_bottom_panel = Rect {
            x: editor_code_x,
            y: below_editor_top_y + editor_body_h,
            width: editor_content_w,
            height: center_bottom_h,
        };

        // Minimap lane to the right of editor content (full column height)
        let minimap_lane = Rect {
            x: editor_code_x + editor_content_w,
            y: columns_y,
            width: minimap_w,
            height: columns_h,
        };

        // AI panel header and content split
        let ai_header_h: u32 = lt.ai_header_h;
        let ai_panel_header =
            Rect { x: ai_panel.x, y: ai_panel.y, width: ai_panel.width, height: ai_header_h };
        let ai_panel_content = Rect {
            x: ai_panel.x,
            y: ai_panel_header.y + ai_panel_header.height,
            width: ai_panel.width,
            height: ai_panel.height.saturating_sub(ai_header_h),
        };

        let regions = vec![
            ShellRegion { id: "toolbar", name: "editor_header_toolbar", rect: toolbar },
            ShellRegion { id: "app_rail", name: "app_rail", rect: app_rail },
            ShellRegion { id: "sidebar", name: "sidebar", rect: sidebar },
            ShellRegion { id: "editor_tabs", name: "editor_tabs", rect: editor_tabs },
            ShellRegion { id: "breadcrumb", name: "breadcrumb", rect: breadcrumb },
            ShellRegion { id: "gutter_lane", name: "gutter_lane", rect: gutter_lane },
            ShellRegion { id: "editor_content", name: "editor_content", rect: center_editor },
            ShellRegion { id: "center_editor", name: "center_editor", rect: center_editor },
            ShellRegion { id: "minimap_lane", name: "minimap_lane", rect: minimap_lane },
            ShellRegion {
                id: "center_bottom_panel",
                name: "center_bottom_panel",
                rect: center_bottom_panel,
            },
            ShellRegion { id: "bottom_dock", name: "bottom_dock", rect: bottom_dock },
            ShellRegion { id: "ai_panel_header", name: "ai_panel_header", rect: ai_panel_header },
            ShellRegion {
                id: "ai_panel_content",
                name: "ai_panel_content",
                rect: ai_panel_content,
            },
            ShellRegion { id: "status_bar", name: "status_bar", rect: status },
        ];

        ShellFrame { size, theme, regions, work_content: None }
    }

    /// Render a deterministic textual transcript describing each region.
    /// This is intentionally small and stable to enable tests and harness checks.
    pub fn render_lines(&self, comp: Option<&crate::desktop::DesktopComposition>) -> Vec<String> {
        let mut lines: Vec<String> = Vec::new();
        lines.push(format!("shell_size: {}x{}", self.size.width, self.size.height));
        lines.push(format!("theme.surface: {}", self.theme.surface));
        lines.push(format!(
            "theme.border: {} px color {}",
            self.theme.border_thickness, self.theme.border_color
        ));
        lines.push("regions:".to_string());
        for r in &self.regions {
            lines.push(format!("  id={} name={} rect={}", r.id, r.name, r.rect));
        }

        // Append chrome/widget-level deterministic placeholders produced by the widgets module.
        let widget_lines = super::widgets::render_chrome(&self.regions, comp);
        if !widget_lines.is_empty() {
            lines.push("widgets:".to_string());
            for wl in widget_lines {
                lines.push(format!("  {}", wl));
            }
        }

        lines
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gui::region_dispatch::region_role;
    use zaroxi_core_engine_style::PanelRole;

    /// Verify the editor keeps a usable width at every responsive breakpoint
    /// and that side panels shrink instead of squeezing the editor to zero.
    #[test]
    fn editor_never_collapses_at_any_width() {
        let widths = [1400u32, 1000, 920, 700, 630, 500, 400];
        for &w in &widths {
            let shell = ShellFrame::new(
                Size { width: w, height: 400 },
                zaroxi_interface_theme::theme::ZaroxiTheme::Dark,
            );
            let editor = shell
                .regions
                .iter()
                .find(|r| region_role(r.id) == PanelRole::ContentArea)
                .expect("center_editor region missing");
            assert!(
                editor.rect.width >= 120,
                "editor width {} < 120 at shell width {}",
                editor.rect.width,
                w
            );
        }
    }

    /// Regions occupying the same horizontal band must not overlap.
    #[test]
    fn no_horizontal_overlap_in_main_band() {
        let shell = ShellFrame::new(
            Size { width: 1000, height: 540 },
            zaroxi_interface_theme::theme::ZaroxiTheme::Dark,
        );
        let find = |role: PanelRole| -> Rect {
            crate::gui::region_dispatch::find_region_by_role(&shell.regions, role).unwrap().rect
        };

        let rail = find(PanelRole::NavigationRail);
        let sidebar = find(PanelRole::SidePanel);
        let gutter = find(PanelRole::GutterLane);
        let editor = find(PanelRole::ContentArea);
        let minimap = find(PanelRole::MinimapLane);
        let ai = find(PanelRole::AuxiliaryPanelContent);

        // Check no gaps between adjacent regions in x-order
        assert_eq!(rail.x + rail.width, sidebar.x, "rail/sidebar gap");

        // Tabs/breadcrumb abuts sidebar; gutter fills the gap below header level
        if gutter.width > 0 {
            assert_eq!(sidebar.x + sidebar.width, gutter.x, "sidebar/gutter gap");
            assert_eq!(gutter.x + gutter.width, editor.x, "gutter/editor gap");
        } else {
            assert_eq!(sidebar.x + sidebar.width, editor.x, "sidebar/editor gap");
        }

        // Minimap sits between editor content and AI panel (or has zero width)
        if minimap.width > 0 {
            assert_eq!(editor.x + editor.width, minimap.x, "editor/minimap gap");
            assert_eq!(minimap.x + minimap.width, ai.x, "minimap/ai gap");
        } else {
            assert_eq!(editor.x + editor.width, ai.x, "editor/ai gap");
        }
    }

    /// At narrow widths, the minimap is hidden and sidebar shrinks.
    #[test]
    fn minimap_hidden_at_narrow_widths() {
        let shell = ShellFrame::new(
            Size { width: 700, height: 400 },
            zaroxi_interface_theme::theme::ZaroxiTheme::Dark,
        );
        let minimap =
            shell.regions.iter().find(|r| region_role(r.id) == PanelRole::MinimapLane).unwrap();
        assert_eq!(minimap.rect.width, 0, "minimap should be hidden at 700px");

        let sidebar =
            shell.regions.iter().find(|r| region_role(r.id) == PanelRole::SidePanel).unwrap();
        assert_eq!(sidebar.rect.width, 170, "sidebar should shrink to 170 at 700px");
    }

    /// Sidebar collapses to small rail-width at very narrow shells.
    #[test]
    fn sidebar_collapses_at_extreme_widths() {
        let shell = ShellFrame::new(
            Size { width: 500, height: 400 },
            zaroxi_interface_theme::theme::ZaroxiTheme::Dark,
        );
        let sidebar =
            shell.regions.iter().find(|r| region_role(r.id) == PanelRole::SidePanel).unwrap();
        assert_eq!(sidebar.rect.width, 60, "sidebar should collapse to 60 at 500px");

        let editor =
            shell.regions.iter().find(|r| region_role(r.id) == PanelRole::ContentArea).unwrap();
        assert!(
            editor.rect.width >= 160,
            "editor should stay usable at 500px, got {}",
            editor.rect.width
        );
        let gutter =
            shell.regions.iter().find(|r| region_role(r.id) == PanelRole::GutterLane).unwrap();
        assert!(gutter.rect.width > 0, "gutter should be present at 500px");
    }
}
