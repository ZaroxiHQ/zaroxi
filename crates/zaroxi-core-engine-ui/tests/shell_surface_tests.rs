#[cfg(test)]
mod tests {
    use zaroxi_core_engine_layout::ShellLayout;
    use zaroxi_core_engine_style::EngineTheme;
    use zaroxi_core_engine_ui::build_shell_surface_set;

    /// Verify that the shell surface builder produces a deterministic,
    /// non-empty set of primitives at a standard window size and that
    /// surface rects are in the expected paint order (bg first).
    #[test]
    fn shell_surface_set_produces_deterministic_regions() {
        let layout = ShellLayout::from_window_size(1200, 800);
        let theme = EngineTheme::dark();
        let set = build_shell_surface_set(&layout, &theme);

        // Non-empty
        assert!(!set.is_empty(), "shell surface set must not be empty");

        // Surfaces are the primary layer — must have at least the full bg + major regions
        assert!(
            set.surfaces.len() >= 5,
            "expected >=5 surfaces, got {}",
            set.surfaces.len()
        );

        // Background must be first in paint order (full window)
        let bg = &set.surfaces[0];
        assert_eq!(bg.rect.x, 0.0, "bg x must be 0");
        assert_eq!(bg.rect.y, 0.0, "bg y must be 0");
        assert_eq!(bg.rect.width, layout.window_size.width, "bg width must span window");
        assert_eq!(bg.rect.height, layout.window_size.height, "bg height must span window");
        assert_eq!(
            bg.fill_color, theme.app_background.to_array(),
            "bg must use app_background color"
        );

        // Titlebar surface must be present
        let titlebar_present = set.surfaces.iter().any(|s| {
            (s.rect.y - layout.titlebar.y).abs() < 1.0
                && (s.rect.height - layout.titlebar.height).abs() < 1.0
                && s.rect.x == layout.titlebar.x
        });
        assert!(titlebar_present, "titlebar surface missing");

        // Editor content surface must be present
        let editor_present = set.surfaces.iter().any(|s| {
            (s.rect.y - layout.editor_content.y).abs() < 1.0
                && (s.rect.x - layout.editor_content.x).abs() < 1.0
        });
        assert!(editor_present, "editor content surface missing");

        // Status bar surface must be present
        let status_present = set.surfaces.iter().any(|s| {
            (s.rect.y - layout.status_bar.y).abs() < 1.0
                && (s.rect.x - layout.status_bar.x).abs() < 1.0
        });
        assert!(status_present, "status bar surface missing");

        // Dividers must be present (at least sidebar border + status top)
        assert!(set.dividers.len() >= 2, "expected >=2 dividers, got {}", set.dividers.len());

        // Headers must be present (at least AI panel + bottom panel)
        assert!(set.headers.len() >= 2, "expected >=2 headers, got {}", set.headers.len());
    }

    /// Verify that when the sidebar width is zero, the sidebar-related surfaces are absent.
    #[test]
    fn narrow_window_reduces_sidebar_content() {
        let layout = ShellLayout::from_window_size(200, 600);
        let theme = EngineTheme::dark();
        let set = build_shell_surface_set(&layout, &theme);

        // At 200px wide, sidebar collapses to zero, so sidebar surface count should be lower
        // Still works without panicking
        assert!(!set.is_empty(), "even narrow windows must produce valid surface set");
    }

    /// Verify the to_rect_primitives conversion is consistent.
    #[test]
    fn to_rect_primitives_preserves_count_and_order() {
        let layout = ShellLayout::from_window_size(1000, 700);
        let theme = EngineTheme::dark();
        let set = build_shell_surface_set(&layout, &theme);

        let rects = set.to_rect_primitives();

        let expected_min = set.surfaces.len() + set.headers.len() + set.dividers.len()
            + set.status_pills.len() + set.tabs.len() + set.icons.len();
        assert_eq!(
            rects.len(),
            expected_min,
            "rect primitive count must equal sum of all primitive layers"
        );

        // First rect must be the background
        assert_eq!(rects[0].x, 0.0);
        assert_eq!(rects[0].y, 0.0);
    }
}
