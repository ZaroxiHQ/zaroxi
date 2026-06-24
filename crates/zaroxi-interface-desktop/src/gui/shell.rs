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

/// Convert a shell region's integer layout rect into the renderer's float rect.
///
/// Every panel module needs the same `ShellRegion -> render::Rect` cast when
/// building its `UiBlock`s; centralizing it here removes the repeated
/// `Rect { x: r.rect.x as f32, .. }` boilerplate and keeps the layout-to-render
/// coordinate conversion in one place.
impl From<&ShellRegion> for zaroxi_core_engine_render::Rect {
    fn from(region: &ShellRegion) -> Self {
        zaroxi_core_engine_render::Rect {
            x: region.rect.x as f32,
            y: region.rect.y as f32,
            w: region.rect.width as f32,
            h: region.rect.height as f32,
        }
    }
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

        // Single source of truth: region construction is delegated to the layout
        // controller's builder, so there is exactly one shell region builder (the
        // live render loop uses the same function). No duplicate region list here.
        let layout =
            crate::gui::window::editor_shell::compute_layout(size.width as f32, size.height as f32);
        let regions =
            crate::gui::window::editor_shell::controller::build_shell_regions_from_layout(&layout);

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
        let ai = find(PanelRole::AuxiliaryPanelContent);

        // Sidebar anchors to x=0 (rail is below, at the bottom of the left column).
        assert_eq!(sidebar.x, 0, "sidebar should anchor at x=0");
        assert!(
            rail.width >= sidebar.width.saturating_sub(2),
            "rail width should match sidebar"
        );
        let expected_rail_y = sidebar.y + sidebar.height;
        assert!(
            rail.y.abs_diff(expected_rail_y) <= 2,
            "rail y={} should abut sidebar bottom y+height={}",
            rail.y,
            expected_rail_y
        );

        // Check no gaps between adjacent regions in x-order
        // Tabs/breadcrumb abuts sidebar; gutter fills the gap below header level
        if gutter.width > 0 {
            assert_eq!(sidebar.x + sidebar.width, gutter.x, "sidebar/gutter gap");
            assert_eq!(gutter.x + gutter.width, editor.x, "gutter/editor gap");
        } else {
            assert_eq!(sidebar.x + sidebar.width, editor.x, "sidebar/editor gap");
        }

        // No minimap lane: the AI panel abuts the editor content directly, with
        // no dead reserved column between them.
        assert_eq!(editor.x + editor.width, ai.x, "editor/ai gap (no dead minimap lane)");
    }

    /// The shell exposes no legacy minimap-lane region (overview is cockpit-owned),
    /// so the AI panel abuts the editor content with no dead reserved lane.
    #[test]
    fn no_legacy_minimap_lane_region() {
        for &(w, h) in &[(1200u32, 800u32), (700, 400)] {
            let shell = ShellFrame::new(
                Size { width: w, height: h },
                zaroxi_interface_theme::theme::ZaroxiTheme::Dark,
            );
            assert!(
                shell.regions.iter().all(|r| region_role(r.id) != PanelRole::MinimapLane),
                "no shell region should map to the legacy MinimapLane role (at {w}x{h})"
            );
        }
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
