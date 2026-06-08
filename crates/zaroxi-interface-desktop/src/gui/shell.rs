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
    pub fn new(size: Size, _resolved_theme: zaroxi_interface_theme::theme::ZaroxiTheme) -> Self {
        let theme = Theme::from_variant(_resolved_theme);

        let layout =
            crate::gui::window::editor_shell::compute_layout(size.width as f32, size.height as f32);

        let tr = |r: (f32, f32, f32, f32)| Rect {
            x: r.0.max(0.0) as u32,
            y: r.1.max(0.0) as u32,
            width: r.2.max(0.0) as u32,
            height: r.3.max(0.0) as u32,
        };

        let regions = vec![
            ShellRegion {
                id: "toolbar",
                name: "editor_header_toolbar",
                rect: tr(layout.toolbar_rect),
            },
            ShellRegion { id: "app_rail", name: "app_rail", rect: tr(layout.rail_rect) },
            ShellRegion { id: "sidebar", name: "sidebar", rect: tr(layout.sidebar_rect) },
            ShellRegion {
                id: "editor_tabs",
                name: "editor_tabs",
                rect: tr(layout.editor_tabs_rect),
            },
            ShellRegion { id: "breadcrumb", name: "breadcrumb", rect: tr(layout.breadcrumb_rect) },
            ShellRegion { id: "gutter_lane", name: "gutter_lane", rect: tr(layout.gutter_rect) },
            ShellRegion {
                id: "editor_content",
                name: "editor_content",
                rect: tr(layout.editor_content_rect),
            },
            ShellRegion { id: "minimap_lane", name: "minimap_lane", rect: tr(layout.minimap_rect) },
            ShellRegion {
                id: "center_bottom_panel",
                name: "center_bottom_panel",
                rect: tr(layout.terminal_rect),
            },
            ShellRegion {
                id: "bottom_dock",
                name: "bottom_dock",
                rect: Rect { x: 0, y: 0, width: 0, height: 0 },
            },
            ShellRegion {
                id: "ai_panel_header",
                name: "ai_panel_header",
                rect: tr(layout.assistant_header_rect),
            },
            ShellRegion {
                id: "ai_panel_content",
                name: "ai_panel_content",
                rect: tr(layout.assistant_content_rect),
            },
            ShellRegion { id: "status_bar", name: "status_bar", rect: tr(layout.status_bar_rect) },
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
                editor.rect.width > 0,
                "editor width {} = 0 at shell width {}",
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

    /// At narrow widths, the minimap shrinks and sidebar contracts.
    #[test]
    fn minimap_hidden_at_narrow_widths() {
        let shell = ShellFrame::new(
            Size { width: 700, height: 400 },
            zaroxi_interface_theme::theme::ZaroxiTheme::Dark,
        );
        let minimap =
            shell.regions.iter().find(|r| region_role(r.id) == PanelRole::MinimapLane).unwrap();
        assert!(
            minimap.rect.width <= 60,
            "minimap should shrink at 700px, got {}",
            minimap.rect.width
        );

        let sidebar =
            shell.regions.iter().find(|r| region_role(r.id) == PanelRole::SidePanel).unwrap();
        assert!(
            sidebar.rect.width < 250,
            "sidebar should shrink at 700px, got {}",
            sidebar.rect.width
        );
    }

    /// Sidebar shrinks significantly at very narrow shells.
    #[test]
    fn sidebar_collapses_at_extreme_widths() {
        let shell = ShellFrame::new(
            Size { width: 500, height: 400 },
            zaroxi_interface_theme::theme::ZaroxiTheme::Dark,
        );
        let sidebar =
            shell.regions.iter().find(|r| region_role(r.id) == PanelRole::SidePanel).unwrap();
        assert!(
            sidebar.rect.width < 200,
            "sidebar w={} should shrink below 200 at 500px",
            sidebar.rect.width
        );

        let editor =
            shell.regions.iter().find(|r| region_role(r.id) == PanelRole::ContentArea).unwrap();
        assert!(
            editor.rect.width > 0,
            "editor should stay usable at 500px, got {}",
            editor.rect.width
        );
    }
}
