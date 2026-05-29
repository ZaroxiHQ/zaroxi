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
pub fn build_overlay_rects(
    shell: &crate::gui::ShellFrame,
) -> Vec<zaroxi_core_engine_render_backend::DrawRect> {
    use std::cmp;

    let mut rects: Vec<zaroxi_core_engine_render_backend::DrawRect> = Vec::new();
    let bt: u32 = shell.theme.border_thickness as u32;
    let sep_h: u32 = cmp::max(1, bt); // ensure visible separators even when border_thickness is 0/1

    for r in &shell.regions {
        match r.id {
            // Top chrome / toolbar band: render a filled band using the accent (border_color).
            // Then emit a stronger bottom separator so the toolbar reads as chrome.
            "toolbar" => {
                // Accent-filled toolbar
                rects.push(zaroxi_core_engine_render_backend::DrawRect {
                    x: r.rect.x,
                    y: r.rect.y,
                    width: r.rect.width,
                    height: r.rect.height,
                    color: super::theme_adapter::parse_hex_color(shell.theme.border_color),
                });

                // Bottom separator (slightly darker accent)
                if r.rect.height > sep_h {
                    rects.push(zaroxi_core_engine_render_backend::DrawRect {
                        x: r.rect.x,
                        y: r.rect.y.saturating_add(r.rect.height.saturating_sub(sep_h)),
                        width: r.rect.width,
                        height: sep_h,
                        color: super::theme_adapter::adjust_brightness(shell.theme.border_color, 0.85),
                    });
                }
            }

            // Main editor content: give it a subtle variance from the shell.surface so it
            // reads as a distinct central pane. Add a thin border on the left/right/top
            // to separate it from the adjacent rails and toolbar.
            "editor_content" => {
                // Slightly lighter surface for the content pane so it reads as the primary canvas.
                rects.push(zaroxi_core_engine_render_backend::DrawRect {
                    x: r.rect.x,
                    y: r.rect.y,
                    width: r.rect.width,
                    height: r.rect.height,
                    color: super::theme_adapter::adjust_brightness(shell.theme.surface, 1.06),
                });

                // Thin top separator (separates from toolbar)
                if r.rect.height > sep_h {
                    rects.push(zaroxi_core_engine_render_backend::DrawRect {
                        x: r.rect.x,
                        y: r.rect.y,
                        width: r.rect.width,
                        height: cmp::min(sep_h, r.rect.height),
                        color: super::theme_adapter::adjust_brightness(shell.theme.border_color, 0.9),
                    });
                }

                // Left and right separators to visually frame the editor pane.
                if r.rect.width > sep_h {
                    // left
                    rects.push(zaroxi_core_engine_render_backend::DrawRect {
                        x: r.rect.x,
                        y: r.rect.y,
                        width: sep_h,
                        height: r.rect.height,
                        color: super::theme_adapter::adjust_brightness(shell.theme.border_color, 0.92),
                    });
                    // right
                    rects.push(zaroxi_core_engine_render_backend::DrawRect {
                        x: r.rect.x.saturating_add(r.rect.width.saturating_sub(sep_h)),
                        y: r.rect.y,
                        width: sep_h,
                        height: r.rect.height,
                        color: super::theme_adapter::adjust_brightness(shell.theme.border_color, 0.92),
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
                    color: super::theme_adapter::adjust_brightness(shell.theme.surface, 0.92),
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
