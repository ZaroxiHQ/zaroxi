/*!
Mapping helpers: convert ShellFrame regions into low-level DrawRect overlays
consumed by the render backend during the one-shot clear+present bootstrap.
*/

/// Build the small set of overlay rects used for the one-shot clear+present.
/// This mirrors the logic previously embedded in the big `window.rs` so the
/// behavior remains identical but the code is easier to navigate.
pub fn build_overlay_rects(
    shell: &crate::gui::ShellFrame,
) -> Vec<zaroxi_core_engine_render_backend::DrawRect> {
    let mut rects: Vec<zaroxi_core_engine_render_backend::DrawRect> = Vec::new();
    let bt: u32 = shell.theme.border_thickness as u32;
    for r in &shell.regions {
        match r.id {
            // Top chrome / toolbar band: render a filled band (accent) so it's visually distinct.
            // We choose the shell's `border_color` token as the chrome accent to preserve
            // existing theme intent while keeping the implementation phase-scoped.
            "toolbar" => {
                rects.push(zaroxi_core_engine_render_backend::DrawRect {
                    x: r.rect.x,
                    y: r.rect.y,
                    width: r.rect.width,
                    height: r.rect.height,
                    color: super::theme_adapter::parse_hex_color(shell.theme.border_color),
                });

                // Emit a thin bottom border under the toolbar to separate it from content.
                if bt > 0 && r.rect.height > bt {
                    rects.push(zaroxi_core_engine_render_backend::DrawRect {
                        x: r.rect.x,
                        y: r.rect.y.saturating_add(r.rect.height.saturating_sub(bt)),
                        width: r.rect.width,
                        height: bt,
                        color: super::theme_adapter::parse_hex_color(shell.theme.border_color),
                    });
                }
            }

            // Main editor content: fill with the shell surface color so it reads as the primary canvas.
            "editor_content" => {
                rects.push(zaroxi_core_engine_render_backend::DrawRect {
                    x: r.rect.x,
                    y: r.rect.y,
                    width: r.rect.width,
                    height: r.rect.height,
                    color: super::theme_adapter::parse_hex_color(shell.theme.surface),
                });
            }

            // Bottom status bar: render as a filled band using the border token so it's visually anchored.
            "status_bar" => {
                rects.push(zaroxi_core_engine_render_backend::DrawRect {
                    x: r.rect.x,
                    y: r.rect.y,
                    width: r.rect.width,
                    height: r.rect.height,
                    color: super::theme_adapter::parse_hex_color(shell.theme.border_color),
                });

                // Emit a thin top border above the status bar to separate it from content.
                if bt > 0 {
                    rects.push(zaroxi_core_engine_render_backend::DrawRect {
                        x: r.rect.x,
                        y: r.rect.y,
                        width: r.rect.width,
                        height: bt,
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
