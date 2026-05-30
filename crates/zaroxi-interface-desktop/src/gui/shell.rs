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
        Theme {
            surface: "#061025",      // dark blue-black surface
            border_color: "#1e6fb3", // thin luminous border
            border_thickness: 1,
            corner_radius: 10, // rounded outer shell
            // Use the workspace semantic values (dark theme defaults tuned for contrast).
            // These are intentionally chosen to read well against surface/backing colors.
            text_primary: "#E6EAF2",
            text_secondary: "#C8CDD6",
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
    /// Construct a new ShellFrame and compute a canonical layout.
    ///
    /// GUI-6 (follow-up): refine proportions and ensure visually-distinct
    /// editor subdivisions: left project rail, center editor, bottom panel,
    /// and the right AI pane. Geometry is explicit and uses simple proportional
    /// heuristics so resizing remains deterministic.
    pub fn new(size: Size) -> Self {
        let theme = Theme::default();

        // Compact desktop IDE spacing tokens (values chosen to match the visual direction)
        let outer_padding: u32 = 8;
        let top_toolbar_h: u32 = 40;
        let left_rail_w: u32 = 60;
        // Compute sidebars proportionally from available inner width so resizing
        // preserves relative balance rather than fixed absolute sizes.
        let inner_x = outer_padding;
        let inner_y = outer_padding;
        let inner_w = size.width.saturating_sub(outer_padding * 2);
        let inner_h = size.height.saturating_sub(outer_padding * 2);

        // Proportional outer sidebar (~20% of inner width), clamped to sane bounds.
        let mut left_sidebar_w = (inner_w.saturating_mul(20)) / 100;
        left_sidebar_w = left_sidebar_w.clamp(140, inner_w.saturating_div(2));

        // Right AI pane (~26% of inner width), clamped.
        let mut ai_panel_w = (inner_w.saturating_mul(26)) / 100;
        ai_panel_w = ai_panel_w.clamp(200, inner_w.saturating_div(2));

        // Keep a stable bottom dock and status height
        let bottom_dock_h: u32 = 120;
        let status_h: u32 = 24;
        let editor_header_h: u32 = 28;
        let minimap_w: u32 = 80;

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

        // App rail (far-left)
        let app_rail = Rect { x: inner_x, y: columns_y, width: left_rail_w, height: columns_h };

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

        // Editor header (top strip inside the editor column)
        let editor_header =
            Rect { x: editor_x, y: columns_y, width: editor_w, height: editor_header_h };

        // Area below the editor header (available for left project rail, center editor, bottom panel)
        let below_header_y = editor_header.y + editor_header.height;
        let below_header_h = columns_h.saturating_sub(editor_header.height);

        // Subdivision heuristics (refined for clearer separations):
        // - left project rail: ~22% of editor column width (clamped)
        // - right-side minimap lane preserved (minimap_w)
        // - center editor: remaining width after left rail & minimap
        // - bottom panel: ~26% of the center editor height (visually prominent)
        let left_inner_pct: f32 = 0.22;
        let right_minimap = minimap_w;
        let mut left_inner_w = ((editor_w as f32) * left_inner_pct) as u32;
        left_inner_w = left_inner_w.clamp(120, 480);

        let center_editor_w = editor_w.saturating_sub(left_inner_w).saturating_sub(right_minimap);

        // Center heights (make the bottom panel a bit taller so it's visually obvious)
        let mut bottom_panel_h = ((below_header_h as f32) * 0.26) as u32;
        bottom_panel_h = bottom_panel_h.clamp(56, below_header_h.saturating_sub(24));
        let center_editor_h = below_header_h.saturating_sub(bottom_panel_h);

        // Left project rail INSIDE the editor column (distinct from outer sidebar)
        let content_left_sidebar =
            Rect { x: editor_x, y: below_header_y, width: left_inner_w, height: below_header_h };

        // Center editor canvas above the bottom panel
        let center_editor = Rect {
            x: content_left_sidebar.x + content_left_sidebar.width,
            y: below_header_y,
            width: center_editor_w,
            height: center_editor_h,
        };

        // Bottom panel occupying the lower strip of the center editor area
        let center_bottom_panel = Rect {
            x: center_editor.x,
            y: center_editor.y.saturating_add(center_editor.height),
            width: center_editor.width,
            height: bottom_panel_h,
        };

        // Minimap lane to the right of center editor (preserve previous visual hint)
        let minimap_lane = Rect {
            x: center_editor.x + center_editor.width,
            y: below_header_y,
            width: right_minimap,
            height: below_header_h,
        };

        // AI panel header and content split (unchanged)
        let ai_header_h: u32 = 36;
        let ai_panel_header =
            Rect { x: ai_panel.x, y: ai_panel.y, width: ai_panel.width, height: ai_header_h };
        let ai_panel_content = Rect {
            x: ai_panel.x,
            y: ai_panel_header.y + ai_panel_header.height,
            width: ai_panel.width,
            height: ai_panel.height.saturating_sub(ai_header_h),
        };

        // Collect regions with stable ids and presentable names.
        // GUI-6 follow-up: emphasize center and bottom subdivisions visually.
        let regions = vec![
            ShellRegion { id: "toolbar", name: "editor_header_toolbar", rect: toolbar },
            ShellRegion { id: "app_rail", name: "app_rail", rect: app_rail },
            ShellRegion { id: "sidebar", name: "sidebar", rect: sidebar },
            ShellRegion { id: "editor_header", name: "editor_header", rect: editor_header },
            ShellRegion {
                id: "content_left_sidebar",
                name: "content_left_sidebar",
                rect: content_left_sidebar,
            },
            // Backwards-compatibility: provide an aggregated editor content region id expected by older tests.
            // This mirrors the center_editor rect so callers that expect "editor_content" continue to pass.
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
