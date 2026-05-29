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
            // Extended for GUI-7: render grouped placeholder rows that suggest a file/project list.
            "app_rail" => {
                rects.push(zaroxi_core_engine_render_backend::DrawRect {
                    x: r.rect.x,
                    y: r.rect.y,
                    width: r.rect.width,
                    height: r.rect.height,
                    color: super::theme_adapter::adjust_brightness(shell.theme.surface, 0.92),
                });

                // Render a small set of horizontal placeholder rows (grouped list hint).
                // Geometry: 4 rows with padding; each row slightly inset and alternates subtle contrast.
                let rows: u32 = 4;
                let padding: u32 = 8;
                if r.rect.height > padding && r.rect.width > padding {
                    let available_h = r.rect.height.saturating_sub(padding.saturating_mul(rows + 1));
                    let row_h = if available_h > 0 { available_h / rows } else { 0 };
                    let mut y_off = r.rect.y.saturating_add(padding);
                    for i in 0..rows {
                        let inset: u32 = if i == 0 { 6 } else { 10 };
                        let row_w = r.rect.width.saturating_sub(inset.saturating_add(padding));
                        // Slight brightness variation per row so the list reads as grouped lines.
                        let factor = (1.02_f64 - (i as f64) * 0.01).clamp(0.0, 2.0);
                        rects.push(zaroxi_core_engine_render_backend::DrawRect {
                            x: r.rect.x.saturating_add(inset),
                            y: y_off,
                            width: row_w,
                            height: row_h,
                            color: super::theme_adapter::adjust_brightness(shell.theme.surface, factor),
                        });

                        // thin separator below the row for subtle delineation
                        if row_h > sep_h {
                            rects.push(zaroxi_core_engine_render_backend::DrawRect {
                                x: r.rect.x,
                                y: y_off.saturating_add(row_h),
                                width: r.rect.width,
                                height: sep_h,
                                color: super::theme_adapter::adjust_brightness(shell.theme.border_color, 0.85),
                            });
                        }

                        y_off = y_off.saturating_add(row_h.saturating_add(padding));
                    }
                }

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
            // Extended for GUI-7: add a tab-strip placeholder along the top of the center editor.
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
                let top_sep_h = cmp::min(sep_h, r.rect.height);
                if r.rect.height > sep_h {
                    rects.push(zaroxi_core_engine_render_backend::DrawRect {
                        x: r.rect.x,
                        y: r.rect.y,
                        width: r.rect.width,
                        height: top_sep_h,
                        color: super::theme_adapter::parse_hex_color(shell.theme.border_color),
                    });
                }

                // Tab-strip placeholders
                // Place tabs just below the top separator so they remain visually attached to the chrome.
                let tab_strip_h: u32 = cmp::min(28, r.rect.height / 8);
                if tab_strip_h > 0 && r.rect.width > 16 {
                    let tabs: u32 = 5;
                    let tab_padding: u32 = 8;
                    let total_padding = tab_padding.saturating_mul(tabs + 1);
                    let tab_w = if r.rect.width > total_padding {
                        (r.rect.width.saturating_sub(total_padding)) / tabs
                    } else {
                        // fallback: evenly divide available width
                        r.rect.width / tabs
                    };
                    let mut tx = r.rect.x.saturating_add(tab_padding);
                    let tab_y = r.rect.y.saturating_add(top_sep_h);
                    for i in 0..tabs {
                        let is_active = i == 0;
                        rects.push(zaroxi_core_engine_render_backend::DrawRect {
                            x: tx,
                            y: tab_y,
                            width: tab_w,
                            height: tab_strip_h,
                            color: if is_active {
                                super::theme_adapter::adjust_brightness(shell.theme.surface, 1.16)
                            } else {
                                super::theme_adapter::adjust_brightness(shell.theme.surface, 1.06)
                            },
                        });
                        tx = tx.saturating_add(tab_w).saturating_add(tab_padding);
                    }

                    // underline separator below tabs for clarity
                    if tab_strip_h > top_sep_h {
                        rects.push(zaroxi_core_engine_render_backend::DrawRect {
                            x: r.rect.x,
                            y: tab_y.saturating_add(tab_strip_h),
                            width: r.rect.width,
                            height: top_sep_h,
                            color: super::theme_adapter::parse_hex_color(shell.theme.border_color),
                        });
                    }
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
            // Extended for GUI-7: add a header band with segment placeholders (terminal / problems / output style).
            "center_bottom_panel" => {
                rects.push(zaroxi_core_engine_render_backend::DrawRect {
                    x: r.rect.x,
                    y: r.rect.y,
                    width: r.rect.width,
                    height: r.rect.height,
                    color: super::theme_adapter::adjust_brightness(shell.theme.surface, 0.88),
                });

                // Header band with a few segment placeholders
                let header_h: u32 = cmp::min(28, r.rect.height / 4);
                if header_h > 0 && r.rect.width > 40 {
                    // header background (subtle elevation)
                    rects.push(zaroxi_core_engine_render_backend::DrawRect {
                        x: r.rect.x,
                        y: r.rect.y,
                        width: r.rect.width,
                        height: header_h,
                        color: super::theme_adapter::adjust_brightness(shell.theme.surface, 0.95),
                    });

                    // segmented placeholders within header
                    let segments: u32 = 3;
                    let seg_pad: u32 = 10;
                    let total_pad = seg_pad.saturating_mul(segments + 1);
                    let seg_w = if r.rect.width > total_pad {
                        (r.rect.width.saturating_sub(total_pad)) / segments
                    } else {
                        r.rect.width / segments
                    };
                    let mut sx = r.rect.x.saturating_add(seg_pad);
                    let seg_y = r.rect.y.saturating_add((header_h / 6).saturating_mul(1)); // slight vertical inset
                    let seg_h = header_h.saturating_sub((header_h / 6) * 2);
                    for i in 0..segments {
                        rects.push(zaroxi_core_engine_render_backend::DrawRect {
                            x: sx,
                            y: seg_y,
                            width: seg_w,
                            height: seg_h,
                            color: super::theme_adapter::adjust_brightness(shell.theme.border_color, 0.88 + (i as f64 * 0.02)),
                        });
                        sx = sx.saturating_add(seg_w).saturating_add(seg_pad);
                    }

                    // thin separator below header to separate from panel body
                    if r.rect.height > header_h.saturating_add(sep_h) {
                        rects.push(zaroxi_core_engine_render_backend::DrawRect {
                            x: r.rect.x,
                            y: r.rect.y.saturating_add(header_h),
                            width: r.rect.width,
                            height: sep_h,
                            color: super::theme_adapter::parse_hex_color(shell.theme.border_color),
                        });
                    }
                }

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
            // Extended for GUI-7: stack a few card-like placeholders to suggest assistant sections.
            "ai_panel_content" => {
                rects.push(zaroxi_core_engine_render_backend::DrawRect {
                    x: r.rect.x,
                    y: r.rect.y,
                    width: r.rect.width,
                    height: r.rect.height,
                    color: super::theme_adapter::adjust_brightness(shell.theme.surface, 0.92),
                });

                // stacked cards inside the AI pane
                let cards: u32 = 3;
                let pad: u32 = 10;
                if r.rect.height > pad.saturating_mul(cards + 1) && r.rect.width > pad * 2 {
                    let available_h = r.rect.height.saturating_sub(pad.saturating_mul(cards + 1));
                    let card_h = if available_h > 0 { available_h / cards } else { 0 };
                    let mut cy = r.rect.y.saturating_add(pad);
                    for i in 0..cards {
                        rects.push(zaroxi_core_engine_render_backend::DrawRect {
                            x: r.rect.x.saturating_add(pad),
                            y: cy,
                            width: r.rect.width.saturating_sub(pad * 2),
                            height: card_h,
                            color: super::theme_adapter::adjust_brightness(shell.theme.surface, 0.94 - (i as f64 * 0.02)),
                        });
                        // slight separator between cards
                        if card_h > sep_h {
                            rects.push(zaroxi_core_engine_render_backend::DrawRect {
                                x: r.rect.x,
                                y: cy.saturating_add(card_h),
                                width: r.rect.width,
                                height: sep_h,
                                color: super::theme_adapter::adjust_brightness(shell.theme.border_color, 0.86),
                            });
                        }
                        cy = cy.saturating_add(card_h.saturating_add(pad));
                    }
                }

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
