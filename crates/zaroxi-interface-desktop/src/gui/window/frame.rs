/*!
Mapping helpers: convert ShellFrame regions into low-level DrawRect overlays
consumed by the render backend during the one-shot clear+present bootstrap.

GUI-5 visual improvements:
- Use small, brightness-adjusted variants of the theme tokens to make toolbar,
  content and status visually distinct while keeping the Theme as the single
  source of truth.
- Add thin separator rects (top / bottom / sides) to make region borders obvious.
- Preserve existing geometry so resize behavior remains deterministic.
*/

/// Build the small set of overlay rects used for the one-shot clear+present.
/// This mirrors the logic previously embedded in the big `window.rs` so the
/// behavior remains identical but the code is easier to navigate.
///
/// Extended for GUI-6: render the new subdivided editor regions with stronger
/// separators and clearer brightness deltas so the four inner IDE regions
/// (left rail, center editor, bottom panel, right AI pane) are unmistakable.
pub fn build_overlay_rects(
    shell: &crate::gui::ShellFrame,
) -> Vec<zaroxi_core_engine_render_backend::DrawRect> {
    use std::cmp;

    let mut rects: Vec<zaroxi_core_engine_render_backend::DrawRect> = Vec::new();
    let bt: u32 = shell.theme.border_thickness as u32;
    // Increase separator thickness to make major region boundaries obvious.
    let sep_h: u32 = cmp::max(2, bt);

    for r in &shell.regions {
        match r.id {
            // Top chrome / toolbar band: mostly unchanged but ensure a clear bottom separator.
            "toolbar" => {
                rects.push(zaroxi_core_engine_render_backend::DrawRect {
                    x: r.rect.x,
                    y: r.rect.y,
                    width: r.rect.width,
                    height: r.rect.height,
                    color: super::theme_adapter::parse_hex_color(shell.theme.border_color),
                });

                if r.rect.height > sep_h {
                    rects.push(zaroxi_core_engine_render_backend::DrawRect {
                        x: r.rect.x,
                        y: r.rect.y.saturating_add(r.rect.height.saturating_sub(sep_h)),
                        width: r.rect.width,
                        height: sep_h,
                        color: super::theme_adapter::adjust_brightness(shell.theme.border_color, 0.80),
                    });
                }
            }

            // App rail (far-left): distinct narrow rail with subtle darker fill and right divider.
            "app_rail" => {
                rects.push(zaroxi_core_engine_render_backend::DrawRect {
                    x: r.rect.x,
                    y: r.rect.y,
                    width: r.rect.width,
                    height: r.rect.height,
                    color: super::theme_adapter::adjust_brightness(shell.theme.surface, 0.92),
                });

                if r.rect.width > sep_h {
                    // right separator
                    rects.push(zaroxi_core_engine_render_backend::DrawRect {
                        x: r.rect.x.saturating_add(r.rect.width).saturating_sub(sep_h),
                        y: r.rect.y,
                        width: sep_h,
                        height: r.rect.height,
                        color: super::theme_adapter::parse_hex_color(shell.theme.border_color),
                    });
                }
            }

            // Outer sidebar (project rail): slightly lighter than app rail so the two rails read separately.
            "sidebar" => {
                rects.push(zaroxi_core_engine_render_backend::DrawRect {
                    x: r.rect.x,
                    y: r.rect.y,
                    width: r.rect.width,
                    height: r.rect.height,
                    color: super::theme_adapter::adjust_brightness(shell.theme.surface, 0.96),
                });

                // Right separator to visually separate the sidebar from editor column.
                if r.rect.width > sep_h {
                    rects.push(zaroxi_core_engine_render_backend::DrawRect {
                        x: r.rect.x.saturating_add(r.rect.width).saturating_sub(sep_h),
                        y: r.rect.y,
                        width: sep_h,
                        height: r.rect.height,
                        color: super::theme_adapter::adjust_brightness(shell.theme.border_color, 0.88),
                    });
                }
            }

            // Editor header: render as a shallow header bar that separates from the center_editor.
            "editor_header" => {
                rects.push(zaroxi_core_engine_render_backend::DrawRect {
                    x: r.rect.x,
                    y: r.rect.y,
                    width: r.rect.width,
                    height: r.rect.height,
                    color: super::theme_adapter::adjust_brightness(shell.theme.surface, 1.02),
                });
                // bottom separator (clear)
                if r.rect.height > sep_h {
                    rects.push(zaroxi_core_engine_render_backend::DrawRect {
                        x: r.rect.x,
                        y: r.rect.y.saturating_add(r.rect.height).saturating_sub(sep_h),
                        width: r.rect.width,
                        height: sep_h,
                        color: super::theme_adapter::parse_hex_color(shell.theme.border_color),
                    });
                }
            }

            // Left inner project rail (inside the editor column)
            "content_left_sidebar" => {
                // Darker fill to read as a distinct column.
                rects.push(zaroxi_core_engine_render_backend::DrawRect {
                    x: r.rect.x,
                    y: r.rect.y,
                    width: r.rect.width,
                    height: r.rect.height,
                    color: super::theme_adapter::adjust_brightness(shell.theme.surface, 0.90),
                });

                // Right separator between left rail and center editor (thicker).
                if r.rect.width > sep_h {
                    rects.push(zaroxi_core_engine_render_backend::DrawRect {
                        x: r.rect.x.saturating_add(r.rect.width).saturating_sub(sep_h),
                        y: r.rect.y,
                        width: sep_h,
                        height: r.rect.height,
                        color: super::theme_adapter::parse_hex_color(shell.theme.border_color),
                    });
                }
            }

            // Center editor canvas (primary content area)
            "center_editor" => {
                // Make the editor canvas slightly brighter than the shell.surface so it reads
                // as the primary, focused area.
                rects.push(zaroxi_core_engine_render_backend::DrawRect {
                    x: r.rect.x,
                    y: r.rect.y,
                    width: r.rect.width,
                    height: r.rect.height,
                    color: super::theme_adapter::adjust_brightness(shell.theme.surface, 1.10),
                });

                // Top separator (separates from editor header)
                if r.rect.height > sep_h {
                    rects.push(zaroxi_core_engine_render_backend::DrawRect {
                        x: r.rect.x,
                        y: r.rect.y,
                        width: r.rect.width,
                        height: cmp::min(sep_h, r.rect.height),
                        color: super::theme_adapter::parse_hex_color(shell.theme.border_color),
                    });
                }

                // Bottom soft divider is handled by the center_bottom_panel top separator.

                // Right separator (before minimap) to visually frame the center.
                if r.rect.width > sep_h {
                    rects.push(zaroxi_core_engine_render_backend::DrawRect {
                        x: r.rect.x.saturating_add(r.rect.width).saturating_sub(sep_h),
                        y: r.rect.y,
                        width: sep_h,
                        height: r.rect.height,
                        color: super::theme_adapter::adjust_brightness(shell.theme.border_color, 0.90),
                    });
                }
            }

            // Minimap lane: keep subtle but distinct from center editor.
            "minimap_lane" => {
                rects.push(zaroxi_core_engine_render_backend::DrawRect {
                    x: r.rect.x,
                    y: r.rect.y,
                    width: r.rect.width,
                    height: r.rect.height,
                    color: super::theme_adapter::adjust_brightness(shell.theme.surface, 0.97),
                });

                // left separator to separate from center editor
                if r.rect.width > sep_h {
                    rects.push(zaroxi_core_engine_render_backend::DrawRect {
                        x: r.rect.x,
                        y: r.rect.y,
                        width: sep_h,
                        height: r.rect.height,
                        color: super::theme_adapter::adjust_brightness(shell.theme.border_color, 0.92),
                    });
                }
            }

            // Bottom panel inside center (terminal / strip) - visually prominent
            "center_bottom_panel" => {
                rects.push(zaroxi_core_engine_render_backend::DrawRect {
                    x: r.rect.x,
                    y: r.rect.y,
                    width: r.rect.width,
                    height: r.rect.height,
                    color: super::theme_adapter::adjust_brightness(shell.theme.surface, 0.88),
                });

                // Strong top separator to clearly separate it from the center editor above.
                if r.rect.height > sep_h {
                    rects.push(zaroxi_core_engine_render_backend::DrawRect {
                        x: r.rect.x,
                        y: r.rect.y,
                        width: r.rect.width,
                        height: sep_h,
                        color: super::theme_adapter::parse_hex_color(shell.theme.border_color),
                    });
                }
            }

            // AI panel content: render as a distinct utility pane with slightly darker fill
            // and a left separator so it reads immediately as a separate major region.
            "ai_panel_content" => {
                rects.push(zaroxi_core_engine_render_backend::DrawRect {
                    x: r.rect.x,
                    y: r.rect.y,
                    width: r.rect.width,
                    height: r.rect.height,
                    color: super::theme_adapter::adjust_brightness(shell.theme.surface, 0.92),
                });

                // Left separator to separate AI pane from editor/minimap (stronger).
                if r.rect.width > sep_h {
                    rects.push(zaroxi_core_engine_render_backend::DrawRect {
                        x: r.rect.x,
                        y: r.rect.y,
                        width: sep_h,
                        height: r.rect.height,
                        color: super::theme_adapter::parse_hex_color(shell.theme.border_color),
                    });
                }
            }

            // Bottom dock (full-width panel above status): subtle elevated fill and top separator.
            "bottom_dock" => {
                rects.push(zaroxi_core_engine_render_backend::DrawRect {
                    x: r.rect.x,
                    y: r.rect.y,
                    width: r.rect.width,
                    height: r.rect.height,
                    color: super::theme_adapter::adjust_brightness(shell.theme.surface, 0.94),
                });

                if r.rect.height > sep_h {
                    rects.push(zaroxi_core_engine_render_backend::DrawRect {
                        x: r.rect.x,
                        y: r.rect.y,
                        width: r.rect.width,
                        height: sep_h,
                        color: super::theme_adapter::adjust_brightness(shell.theme.border_color, 0.9),
                    });
                }
            }

            // Bottom status bar: render as a distinct band using a slightly darker surface
            // so it anchors the frame visually. Emit a top separator to separate it from content.
            "status_bar" => {
                rects.push(zaroxi_core_engine_render_backend::DrawRect {
                    x: r.rect.x,
                    y: r.rect.y,
                    width: r.rect.width,
                    height: r.rect.height,
                    color: super::theme_adapter::adjust_brightness(shell.theme.surface, 0.90),
                });

                // Top separator (stronger contrast)
                if r.rect.height > sep_h {
                    rects.push(zaroxi_core_engine_render_backend::DrawRect {
                        x: r.rect.x,
                        y: r.rect.y,
                        width: r.rect.width,
                        height: sep_h,
                        color: super::theme_adapter::parse_hex_color(shell.theme.border_color),
                    });
                }
            }

            // Keep other regions off the one-shot overlay for now.
            _ => {}
        }
    }

    rects
}
