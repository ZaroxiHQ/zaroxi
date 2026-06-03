use std::fmt;

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
        let tokens = zaroxi_interface_theme::theme::DesignTokens::default();

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
    /// Phase 3 layout: compact chrome, editor-dominant, IDE-grade proportions.
    /// Removes outer padding (chrome regions self-inset), tightens header/separator
    /// heights, narrows the activity rail, and gives the center editor unambiguous
    /// visual dominance.
    pub fn new(size: Size, resolved_theme: zaroxi_interface_theme::theme::ZaroxiTheme) -> Self {
        let theme = Theme::from_variant(resolved_theme);

        // Chrome dimensions — responsive horizontal allocation.
        let outer_padding: u32 = 0;
        let top_toolbar_h: u32 = 30;
        let status_h: u32 = 28;
        let app_rail_w: u32 = 44;

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
        // Breakpoints:
        //   >= 1100  full layout: sidebar=260, ai=320, minimap=56
        //   >= 850   medium:      sidebar=200, ai=260, minimap=48
        //   >= 650   narrow:      sidebar=160, ai=200, minimap=0
        //   < 650    very narrow: sidebar=56,  ai=160, minimap=0
        let (left_sidebar_w, ai_panel_w, minimap_w) = if inner_w >= 1100 {
            (260u32, 320u32, 56u32)
        } else if inner_w >= 850 {
            (200, 260, 48)
        } else if inner_w >= 650 {
            (160, 200, 0)
        } else {
            (56, 160, 0)
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

        // Editor main column sits between sidebar and AI panel
        let editor_x = sidebar.x + sidebar.width;
        let editor_w = ai_panel.x.saturating_sub(editor_x);
        let editor_content_w = editor_w.saturating_sub(minimap_w);
        // Invariant: editor content width must be at least 120 px so tabs/breadcrumb
        // and code viewport remain functional. This is guaranteed by the responsive
        // sidebar/ai/minimap selection above.
        debug_assert!(
            editor_content_w >= 120 || editor_w == 0,
            "editor_content_w={} too narrow at inner_w={}",
            editor_content_w,
            inner_w
        );

        // Editor tiles region: slim tab strip + compact breadcrumb
        let editor_tabs_h: u32 = 28;
        let breadcrumb_h: u32 = 20;
        let editor_top_h = editor_tabs_h + breadcrumb_h;

        // Available height for editor body + terminal panel
        let below_editor_top_y = columns_y + editor_top_h;
        let below_editor_top_h = columns_h.saturating_sub(editor_top_h);

        // Terminal panel (~24% of editor content height, clamped conservatively)
        let mut center_bottom_h = ((below_editor_top_h as f32) * 0.24) as u32;
        center_bottom_h =
            center_bottom_h.saturating_sub(0).min(below_editor_top_h.saturating_sub(80)).max(0);
        let editor_body_h = below_editor_top_h.saturating_sub(center_bottom_h);

        // Editor tabs row (tab strip at top of editor column)
        let editor_tabs =
            Rect { x: editor_x, y: columns_y, width: editor_content_w, height: editor_tabs_h };

        // Breadcrumb / path row below tabs
        let breadcrumb = Rect {
            x: editor_x,
            y: columns_y + editor_tabs_h,
            width: editor_content_w,
            height: breadcrumb_h,
        };

        // Center editor canvas (code area)
        let center_editor = Rect {
            x: editor_x,
            y: below_editor_top_y,
            width: editor_content_w,
            height: editor_body_h,
        };

        // Terminal panel below editor body
        let center_bottom_panel = Rect {
            x: editor_x,
            y: below_editor_top_y + editor_body_h,
            width: editor_content_w,
            height: center_bottom_h,
        };

        // Minimap lane to the right of editor content (full column height)
        let minimap_lane = Rect {
            x: editor_x + editor_content_w,
            y: columns_y,
            width: minimap_w,
            height: columns_h,
        };

        // AI panel header and content split
        let ai_header_h: u32 = 28;
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
        let widths = [1354u32, 960, 850, 680, 650, 500, 400];
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
            Size { width: 960, height: 540 },
            zaroxi_interface_theme::theme::ZaroxiTheme::Dark,
        );
        let find = |role: PanelRole| -> Rect {
            crate::gui::region_dispatch::find_region_by_role(&shell.regions, role).unwrap().rect
        };

        let rail = find(PanelRole::NavigationRail);
        let sidebar = find(PanelRole::SidePanel);
        let editor = find(PanelRole::ContentArea);
        let minimap = find(PanelRole::MinimapLane);
        let ai = find(PanelRole::AuxiliaryPanelContent);

        // Check no gaps between adjacent regions
        assert_eq!(rail.x + rail.width, sidebar.x, "rail/sidebar gap");
        assert_eq!(sidebar.x + sidebar.width, editor.x, "sidebar/editor gap");

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
            Size { width: 680, height: 400 },
            zaroxi_interface_theme::theme::ZaroxiTheme::Dark,
        );
        let minimap =
            shell.regions.iter().find(|r| region_role(r.id) == PanelRole::MinimapLane).unwrap();
        assert_eq!(minimap.rect.width, 0, "minimap should be hidden at 680px");

        let sidebar =
            shell.regions.iter().find(|r| region_role(r.id) == PanelRole::SidePanel).unwrap();
        assert_eq!(sidebar.rect.width, 160, "sidebar should shrink to 160 at 680px");
    }

    /// Sidebar collapses to rail-width at very narrow shells.
    #[test]
    fn sidebar_collapses_at_extreme_widths() {
        let shell = ShellFrame::new(
            Size { width: 500, height: 400 },
            zaroxi_interface_theme::theme::ZaroxiTheme::Dark,
        );
        let sidebar =
            shell.regions.iter().find(|r| region_role(r.id) == PanelRole::SidePanel).unwrap();
        assert_eq!(sidebar.rect.width, 56, "sidebar should collapse to 56 at 500px");

        let editor =
            shell.regions.iter().find(|r| region_role(r.id) == PanelRole::ContentArea).unwrap();
        assert!(
            editor.rect.width >= 200,
            "editor should stay usable at 500px, got {}",
            editor.rect.width
        );
    }
}
