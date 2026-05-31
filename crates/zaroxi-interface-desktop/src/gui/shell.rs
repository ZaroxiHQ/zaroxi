#![allow(dead_code)]
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

impl Default for Theme {
    fn default() -> Self {
        // Source canonical tokens from the workspace theme crate so the desktop
        // crate does not own or hardcode theme values. Use the resolved dark
        // variant for deterministic rendering in tests/harnesses.
        let sem = zaroxi_interface_theme::theme::ZaroxiTheme::Dark.colors(false);
        let tokens = zaroxi_interface_theme::theme::DesignTokens::default();

        // Convert colors to hex strings and leak them to 'static so existing
        // interfaces that expect &'static str continue to work without wide
        // changes across the crate. This is a narrow shim and avoids creating
        // a second theme representation inside the desktop crate.
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

/// ShellFrame is the top-level layout model for GUI-1. It computes the allocation
/// of the major regions for a given outer size and can render a deterministic
/// placeholder transcript describing the regions.
#[derive(Debug, Clone)]
pub struct ShellFrame {
    pub size: Size,
    pub theme: Theme,
    pub regions: Vec<ShellRegion>,
}

impl ShellFrame {
    /// Construct a new ShellFrame and compute a canonical IDE layout.
    ///
    /// Phase 2 layout: refined proportions, breadcrumb row, balanced panel widths
    /// matching the target reference image structure.
    pub fn new(size: Size) -> Self {
        let theme = Theme::default();

        // Use design tokens for spacing/metrics where reasonable (theme-driven sizes)
        let tokens = zaroxi_interface_theme::theme::DesignTokens::default();
        let outer_padding: u32 = tokens.spacing_xl as u32; // expected 24
        // Top toolbar height derived from token + small delta for visual comfort
        let top_toolbar_h: u32 = tokens.spacing_xxl as u32 + 12; // ~32 + 12 = 44
        let status_h: u32 = (tokens.spacing_md + tokens.font_size_sm + 2.0) as u32; // ~12+12+2=26
        let bottom_dock_h: u32 = 0; // no full-width bottom slab (terminal docked to center)

        let inner_x = outer_padding;
        let inner_y = outer_padding;
        let inner_w = size.width.saturating_sub(outer_padding * 2);
        let inner_h = size.height.saturating_sub(outer_padding * 2);

        // Left activity rail (compact)
        let app_rail_w: u32 = 48;
        // Left sidebar: target ~260px
        let mut left_sidebar_w: u32 = 260;
        left_sidebar_w = left_sidebar_w.clamp(180, inner_w.saturating_div(2));
        // Right AI panel: target ~320px
        let mut ai_panel_w: u32 = 320;
        ai_panel_w = ai_panel_w.clamp(220, inner_w.saturating_div(2));

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

        // Minimap lane inset on the right inside the editor column (narrow)
        let minimap_w: u32 =
            (zaroxi_interface_theme::theme::DesignTokens::default().spacing_lg * 4.0) as u32; // use token-derived width (~64)
        let editor_content_w = editor_w.saturating_sub(minimap_w);

        // Editor tiles region: tab strip + breadcrumb row at top of editor column
        let editor_tabs_h: u32 = 34;
        let breadcrumb_h: u32 = 24;
        let editor_top_h = editor_tabs_h + breadcrumb_h;

        // Available height for editor body + terminal panel
        let below_editor_top_y = columns_y + editor_top_h;
        let below_editor_top_h = columns_h.saturating_sub(editor_top_h);

        // Terminal panel (~28% of editor content height)
        let mut center_bottom_h = ((below_editor_top_h as f32) * 0.28) as u32;
        center_bottom_h = center_bottom_h.clamp(80, below_editor_top_h.saturating_sub(60));
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
        let ai_header_h: u32 = 36;
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

        ShellFrame { size, theme, regions }
    }

    /// Render a deterministic textual transcript describing each region.
    /// This is intentionally small and stable to enable tests and harness checks.
    pub fn render_lines(&self) -> Vec<String> {
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
        // Keeping widget rendering logic in `gui/widgets` preserves separation of concerns:
        // - `shell` remains the layout model and region source of truth
        // - `widgets` provides small, interface-facing chrome transcripts derived from regions
        let widget_lines = super::widgets::render_chrome(&self.regions);
        if !widget_lines.is_empty() {
            lines.push("widgets:".to_string());
            for wl in widget_lines {
                lines.push(format!("  {}", wl));
            }
        }

        lines
    }
}
