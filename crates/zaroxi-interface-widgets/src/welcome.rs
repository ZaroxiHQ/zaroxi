use crate::{WidgetLayer, WidgetText, ZaroxiWidget, brush, color_arr};
use vello::Scene;
use vello::kurbo::{Affine, Line, Point, Stroke};
use zaroxi_interface_theme::SemanticColors;

/// Professional IDE-style Welcome home page.
///
/// Multi-section layout rendered cockpit-natively. No file-editor
/// visual language (line numbers, text buffer rows) is used.
pub struct WelcomePanel {
    pub title: String,
    pub tagline: String,
    pub hint_open: String,
    pub hint_switch: String,
    pub hint_settings: String,
    pub hint_ai: String,
    pub recent_placeholder: String,
}

/// Vertical spacing tokens (logical px, relative to panel origin `py`).
const ROW_HERO: f32 = 28.0;
const ROW_TAGLINE: f32 = 64.0;
const ROW_SEC_HEADING: f32 = 102.0;
const ROW_HINT0: f32 = 124.0;
const ROW_HINT1: f32 = 146.0;
const ROW_HINT2: f32 = 166.0;
const ROW_HINT3: f32 = 186.0;
const ROW_RECENT_HEADING: f32 = 220.0;
const ROW_RECENT: f32 = 242.0;
const INSET_X: f32 = 28.0;

impl ZaroxiWidget for WelcomePanel {
    fn layer(&self) -> WidgetLayer {
        WidgetLayer::ActivityRail
    }

    fn paint(&self, scene: &mut Scene, layout: &taffy::Layout, theme: &SemanticColors) {
        let cw = layout.size.width;
        if cw < 200.0 {
            return;
        }
        let panel = crate::layout_rect(layout);
        let accent_color = brush(theme.accent);
        let subtle = brush(theme.divider_subtle);

        // Accent rule under hero heading.
        let rule_w = 48.0f64.min(cw as f64 - INSET_X as f64 * 2.0);
        scene.stroke(
            &Stroke::new(2.0),
            Affine::IDENTITY,
            accent_color,
            None,
            &Line::new(
                Point::new(panel.x0 + INSET_X as f64, panel.y0 + 50.0),
                Point::new(panel.x0 + INSET_X as f64 + rule_w, panel.y0 + 50.0),
            ),
        );

        // Separator between hints section and recent section.
        if cw >= 400.0 {
            let sep_y = panel.y0 + 210.0;
            scene.stroke(
                &Stroke::new(1.0),
                Affine::IDENTITY,
                subtle,
                None,
                &Line::new(
                    Point::new(panel.x0 + INSET_X as f64, sep_y),
                    Point::new(panel.x0 + cw as f64 - INSET_X as f64, sep_y),
                ),
            );
        }
    }

    fn text_items(&self, layout: &taffy::Layout, theme: &SemanticColors) -> Vec<WidgetText> {
        let px = layout.location.x;
        let py = layout.location.y;
        let cw = layout.size.width;
        let ch = layout.size.height;
        let clip = (px, py, cw, ch);

        let fit = |s: &str, em: f32| -> String {
            let max_n = ((cw - INSET_X * 2.0) / em).max(4.0) as usize;
            if s.chars().count() > max_n {
                let mut out: String = s.chars().take(max_n.saturating_sub(1)).collect();
                out.push('\u{2026}');
                out
            } else {
                s.to_string()
            }
        };

        let t_primary = color_arr(theme.text_primary);
        let t_muted = color_arr(theme.text_muted);
        let t_disabled = color_arr(theme.text_disabled);
        let t_accent = color_arr(theme.accent);

        let mut items = vec![
            // Hero
            WidgetText::new(fit(&self.title, 13.0), px + INSET_X, py + ROW_HERO, 20.0, t_primary)
                .with_clip(clip),
            WidgetText::new(fit(&self.tagline, 7.8), px + INSET_X, py + ROW_TAGLINE, 12.5, t_muted)
                .with_clip(clip),
            // Getting started
            WidgetText::new(
                "GETTING STARTED".to_string(),
                px + INSET_X,
                py + ROW_SEC_HEADING,
                10.0,
                t_accent,
            )
            .with_clip(clip),
            WidgetText::new(fit(&self.hint_open, 7.0), px + INSET_X, py + ROW_HINT0, 11.0, t_muted)
                .with_clip(clip),
            WidgetText::new(
                fit(&self.hint_switch, 7.0),
                px + INSET_X,
                py + ROW_HINT1,
                11.0,
                t_muted,
            )
            .with_clip(clip),
            WidgetText::new(
                fit(&self.hint_settings, 7.0),
                px + INSET_X,
                py + ROW_HINT2,
                11.0,
                t_muted,
            )
            .with_clip(clip),
            WidgetText::new(
                fit(&self.hint_ai, 7.0),
                px + INSET_X,
                py + ROW_HINT3,
                10.5,
                t_disabled,
            )
            .with_clip(clip),
        ];

        // Recent section (wider layouts only)
        if cw >= 400.0 {
            items.push(
                WidgetText::new(
                    "RECENT".to_string(),
                    px + INSET_X,
                    py + ROW_RECENT_HEADING,
                    10.0,
                    t_accent,
                )
                .with_clip(clip),
            );
            items.push(
                WidgetText::new(
                    fit(&self.recent_placeholder, 7.0),
                    px + INSET_X,
                    py + ROW_RECENT,
                    10.5,
                    t_disabled,
                )
                .with_clip(clip),
            );
        }

        items
    }
}
