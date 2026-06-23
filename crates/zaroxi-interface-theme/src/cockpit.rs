//! Cockpit design themes (Void / Ash / Dusk / Light).
//!
//! These are the four named themes for the Zaroxi "cockpit" UI. They live in the
//! theme crate because themes are this crate's responsibility — consumers (e.g.
//! `zaroxi-interface-widgets`) read [`CockpitTokens`] but never define colors.
//!
//! [`CockpitTokens`] is a flat, strongly-typed struct (no `HashMap`), so a widget
//! that references a token the theme forgot to set is a compile error.

use crate::colors::Color;
use serde::{Deserialize, Serialize};

/// The four named cockpit themes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum CockpitTheme {
    /// Near-black background, electric-indigo accents, amber AI highlights.
    Void,
    /// Dark slate background, soft-cyan accents, rose AI highlights.
    Ash,
    /// Deep-navy background, gold accents, teal AI highlights.
    Dusk,
    /// Warm-white background, deep-indigo accents, amber AI highlights.
    Light,
}

impl CockpitTheme {
    /// Stable lowercase identifier.
    pub fn as_str(&self) -> &'static str {
        match self {
            CockpitTheme::Void => "zaroxi-void",
            CockpitTheme::Ash => "zaroxi-ash",
            CockpitTheme::Dusk => "zaroxi-dusk",
            CockpitTheme::Light => "zaroxi-light",
        }
    }

    /// All themes, in menu order.
    pub fn all() -> [CockpitTheme; 4] {
        [CockpitTheme::Void, CockpitTheme::Ash, CockpitTheme::Dusk, CockpitTheme::Light]
    }

    /// Resolve this theme's token set.
    pub fn tokens(&self) -> CockpitTokens {
        match self {
            CockpitTheme::Void => CockpitTokens::void(),
            CockpitTheme::Ash => CockpitTokens::ash(),
            CockpitTheme::Dusk => CockpitTokens::dusk(),
            CockpitTheme::Light => CockpitTokens::light(),
        }
    }
}

impl Default for CockpitTheme {
    fn default() -> Self {
        CockpitTheme::Void
    }
}

/// A complete, compiler-checked cockpit design-token set. Every field is a typed
/// [`Color`]; there is intentionally no dynamic lookup.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct CockpitTokens {
    /// Which named theme produced these tokens.
    pub theme: CockpitTheme,
    /// Whether this is a dark theme (drives motion/contrast heuristics).
    pub is_dark: bool,

    // ── Surfaces ──
    /// Window / editor base background.
    pub bg: Color,
    /// Slightly raised surface (panels, gutters).
    pub bg_elevated: Color,
    /// Card / floating-surface fill.
    pub surface: Color,
    /// Overlay surface (palette, tooltips) before alpha.
    pub surface_overlay: Color,

    // ── Text ──
    /// Primary foreground text.
    pub text_primary: Color,
    /// Secondary / label text.
    pub text_secondary: Color,
    /// Muted / placeholder text.
    pub text_muted: Color,
    /// Text drawn on top of an accent fill.
    pub text_on_accent: Color,

    // ── Accent ──
    /// Primary accent.
    pub accent: Color,
    /// Translucent accent wash.
    pub accent_soft: Color,

    // ── AI ──
    /// AI highlight (modified regions, AI affordances).
    pub ai_highlight: Color,
    /// AI pulse (high-confidence suggestion animation).
    pub ai_pulse: Color,
    /// AI-prediction heatmap base (low probability).
    pub ai_prediction_base: Color,
    /// AI-prediction heatmap warm end (high probability).
    pub ai_prediction_warm: Color,

    // ── Status / LSP ──
    /// Healthy status.
    pub status_healthy: Color,
    /// Slow / degraded status.
    pub status_slow: Color,
    /// Error status.
    pub status_error: Color,
    /// Informational status.
    pub status_info: Color,

    // ── Diff layer ──
    /// Added-line tint.
    pub diff_added_bg: Color,
    /// Added-line left border / glow.
    pub diff_added_border: Color,
    /// Removed-line tint.
    pub diff_removed_bg: Color,
    /// Removed-line strike-through color.
    pub diff_removed_strike: Color,

    // ── Semantic minimap ──
    /// Minimap background.
    pub minimap_bg: Color,
    /// Function symbol block.
    pub sym_function: Color,
    /// Type symbol diamond.
    pub sym_type: Color,
    /// Import line.
    pub sym_import: Color,
    /// AI-modified region highlight in the minimap (amber).
    pub minimap_ai_region: Color,

    // ── Left gutter ──
    /// Gutter background.
    pub gutter_bg: Color,
    /// Line-number text.
    pub gutter_linenum: Color,
    /// Git-blame heat, cold (older) end.
    pub gutter_blame_cold: Color,
    /// Git-blame heat, hot (recent) end.
    pub gutter_blame_hot: Color,
    /// LSP error dot.
    pub gutter_error: Color,
    /// LSP warning dot.
    pub gutter_warning: Color,
    /// AI "edit-next" prediction marker.
    pub gutter_ai_marker: Color,

    // ── Context canvas ──
    /// Floating file-panel fill.
    pub canvas_panel_bg: Color,
    /// Import connection line.
    pub canvas_connection: Color,
    /// Animated flow particle.
    pub canvas_particle: Color,
    /// Connection / panel glow.
    pub canvas_glow: Color,

    // ── Command palette ──
    /// Frosted-glass palette fill (pre-alpha).
    pub palette_bg: Color,
    /// Palette border.
    pub palette_border: Color,
    /// Fuzzy-match character highlight.
    pub palette_match: Color,

    // ── Chrome ──
    /// Divider / hairline.
    pub divider: Color,
    /// Focus ring.
    pub focus_ring: Color,
    /// Text selection background.
    pub selection: Color,
    /// Caret color.
    pub cursor: Color,
    /// Drop-shadow color.
    pub shadow: Color,
}

#[inline]
fn c(hex: &str) -> Color {
    Color::from_hex(hex)
}

#[inline]
fn ca(hex: &str, a: f32) -> Color {
    Color::from_hex(hex).with_alpha(a)
}

impl CockpitTokens {
    /// Resolve a named theme's tokens.
    pub fn for_theme(theme: CockpitTheme) -> CockpitTokens {
        theme.tokens()
    }

    /// Zaroxi Void — near-black, electric-indigo, amber AI.
    pub fn void() -> CockpitTokens {
        CockpitTokens {
            theme: CockpitTheme::Void,
            is_dark: true,
            bg: c("#0A0A0F"),
            bg_elevated: c("#12121A"),
            surface: c("#16161F"),
            surface_overlay: c("#1E1E2A"),
            text_primary: c("#E6E6F0"),
            text_secondary: c("#A8A8C0"),
            text_muted: c("#6C6C84"),
            text_on_accent: c("#FFFFFF"),
            accent: c("#6E40C9"),
            accent_soft: ca("#6E40C9", 0.18),
            ai_highlight: c("#F59E0B"),
            ai_pulse: c("#FBBF24"),
            ai_prediction_base: c("#0E7490"),
            ai_prediction_warm: c("#F59E0B"),
            status_healthy: c("#34D399"),
            status_slow: c("#F59E0B"),
            status_error: c("#F87171"),
            status_info: c("#60A5FA"),
            diff_added_bg: ca("#34D399", 0.10),
            diff_added_border: c("#34D399"),
            diff_removed_bg: ca("#F87171", 0.10),
            diff_removed_strike: c("#F87171"),
            minimap_bg: c("#0E0E16"),
            sym_function: c("#6E40C9"),
            sym_type: c("#7DCFFF"),
            sym_import: c("#4B5563"),
            minimap_ai_region: ca("#F59E0B", 0.55),
            gutter_bg: c("#0A0A0F"),
            gutter_linenum: c("#5A5A72"),
            gutter_blame_cold: c("#1E293B"),
            gutter_blame_hot: c("#F59E0B"),
            gutter_error: c("#F87171"),
            gutter_warning: c("#FBBF24"),
            gutter_ai_marker: c("#6E40C9"),
            canvas_panel_bg: c("#12121A"),
            canvas_connection: ca("#6E40C9", 0.6),
            canvas_particle: c("#F59E0B"),
            canvas_glow: ca("#6E40C9", 0.35),
            palette_bg: ca("#12121A", 0.85),
            palette_border: ca("#6E40C9", 0.40),
            palette_match: c("#F59E0B"),
            divider: c("#232334"),
            focus_ring: c("#6E40C9"),
            selection: ca("#6E40C9", 0.30),
            cursor: c("#F59E0B"),
            shadow: ca("#000000", 0.60),
        }
    }

    /// Zaroxi Ash — dark slate, soft cyan, rose AI.
    pub fn ash() -> CockpitTokens {
        CockpitTokens {
            theme: CockpitTheme::Ash,
            is_dark: true,
            bg: c("#1A1B26"),
            bg_elevated: c("#1F2030"),
            surface: c("#24283B"),
            surface_overlay: c("#2A2E42"),
            text_primary: c("#C0CAF5"),
            text_secondary: c("#9AA5CE"),
            text_muted: c("#565F89"),
            text_on_accent: c("#1A1B26"),
            accent: c("#7DCFFF"),
            accent_soft: ca("#7DCFFF", 0.18),
            ai_highlight: c("#FF79C6"),
            ai_pulse: c("#FF9CDB"),
            ai_prediction_base: c("#1ABC9C"),
            ai_prediction_warm: c("#FF79C6"),
            status_healthy: c("#9ECE6A"),
            status_slow: c("#E0AF68"),
            status_error: c("#F7768E"),
            status_info: c("#7DCFFF"),
            diff_added_bg: ca("#9ECE6A", 0.10),
            diff_added_border: c("#9ECE6A"),
            diff_removed_bg: ca("#F7768E", 0.10),
            diff_removed_strike: c("#F7768E"),
            minimap_bg: c("#16171F"),
            sym_function: c("#7DCFFF"),
            sym_type: c("#BB9AF7"),
            sym_import: c("#565F89"),
            minimap_ai_region: ca("#FF79C6", 0.55),
            gutter_bg: c("#1A1B26"),
            gutter_linenum: c("#565F89"),
            gutter_blame_cold: c("#24283B"),
            gutter_blame_hot: c("#FF79C6"),
            gutter_error: c("#F7768E"),
            gutter_warning: c("#E0AF68"),
            gutter_ai_marker: c("#7DCFFF"),
            canvas_panel_bg: c("#1F2030"),
            canvas_connection: ca("#7DCFFF", 0.6),
            canvas_particle: c("#FF79C6"),
            canvas_glow: ca("#7DCFFF", 0.35),
            palette_bg: ca("#1F2030", 0.85),
            palette_border: ca("#7DCFFF", 0.40),
            palette_match: c("#FF79C6"),
            divider: c("#2A2E42"),
            focus_ring: c("#7DCFFF"),
            selection: ca("#7DCFFF", 0.28),
            cursor: c("#FF79C6"),
            shadow: ca("#000000", 0.55),
        }
    }

    /// Zaroxi Dusk — deep navy, gold, teal AI.
    pub fn dusk() -> CockpitTokens {
        CockpitTokens {
            theme: CockpitTheme::Dusk,
            is_dark: true,
            bg: c("#0D1117"),
            bg_elevated: c("#161B22"),
            surface: c("#1C2128"),
            surface_overlay: c("#22272E"),
            text_primary: c("#E6EDF3"),
            text_secondary: c("#ADBAC7"),
            text_muted: c("#768390"),
            text_on_accent: c("#0D1117"),
            accent: c("#E6B450"),
            accent_soft: ca("#E6B450", 0.18),
            ai_highlight: c("#3DDBD9"),
            ai_pulse: c("#6FE8E6"),
            ai_prediction_base: c("#2D7D7B"),
            ai_prediction_warm: c("#3DDBD9"),
            status_healthy: c("#57AB5A"),
            status_slow: c("#E6B450"),
            status_error: c("#E5534B"),
            status_info: c("#539BF5"),
            diff_added_bg: ca("#57AB5A", 0.10),
            diff_added_border: c("#57AB5A"),
            diff_removed_bg: ca("#E5534B", 0.10),
            diff_removed_strike: c("#E5534B"),
            minimap_bg: c("#0B0F14"),
            sym_function: c("#E6B450"),
            sym_type: c("#3DDBD9"),
            sym_import: c("#545D68"),
            minimap_ai_region: ca("#3DDBD9", 0.55),
            gutter_bg: c("#0D1117"),
            gutter_linenum: c("#545D68"),
            gutter_blame_cold: c("#1C2128"),
            gutter_blame_hot: c("#E6B450"),
            gutter_error: c("#E5534B"),
            gutter_warning: c("#E6B450"),
            gutter_ai_marker: c("#3DDBD9"),
            canvas_panel_bg: c("#161B22"),
            canvas_connection: ca("#E6B450", 0.6),
            canvas_particle: c("#3DDBD9"),
            canvas_glow: ca("#3DDBD9", 0.35),
            palette_bg: ca("#161B22", 0.85),
            palette_border: ca("#E6B450", 0.40),
            palette_match: c("#3DDBD9"),
            divider: c("#22272E"),
            focus_ring: c("#E6B450"),
            selection: ca("#539BF5", 0.28),
            cursor: c("#3DDBD9"),
            shadow: ca("#000000", 0.55),
        }
    }

    /// Zaroxi Light — warm white, deep indigo, amber AI.
    pub fn light() -> CockpitTokens {
        CockpitTokens {
            theme: CockpitTheme::Light,
            is_dark: false,
            bg: c("#F8F5F0"),
            bg_elevated: c("#FFFFFF"),
            surface: c("#FFFFFF"),
            surface_overlay: c("#FFFFFF"),
            text_primary: c("#1F2328"),
            text_secondary: c("#4B5158"),
            text_muted: c("#8C939B"),
            text_on_accent: c("#FFFFFF"),
            accent: c("#3730A3"),
            accent_soft: ca("#3730A3", 0.12),
            ai_highlight: c("#D97706"),
            ai_pulse: c("#F59E0B"),
            ai_prediction_base: c("#0E7490"),
            ai_prediction_warm: c("#D97706"),
            status_healthy: c("#15803D"),
            status_slow: c("#B45309"),
            status_error: c("#B91C1C"),
            status_info: c("#1D4ED8"),
            diff_added_bg: ca("#15803D", 0.10),
            diff_added_border: c("#15803D"),
            diff_removed_bg: ca("#B91C1C", 0.10),
            diff_removed_strike: c("#B91C1C"),
            minimap_bg: c("#EFEAE2"),
            sym_function: c("#3730A3"),
            sym_type: c("#0E7490"),
            sym_import: c("#9CA3AF"),
            minimap_ai_region: ca("#D97706", 0.45),
            gutter_bg: c("#F1EDE6"),
            gutter_linenum: c("#9CA3AF"),
            gutter_blame_cold: c("#E2DDD3"),
            gutter_blame_hot: c("#D97706"),
            gutter_error: c("#B91C1C"),
            gutter_warning: c("#B45309"),
            gutter_ai_marker: c("#3730A3"),
            canvas_panel_bg: c("#FFFFFF"),
            canvas_connection: ca("#3730A3", 0.5),
            canvas_particle: c("#D97706"),
            canvas_glow: ca("#3730A3", 0.25),
            palette_bg: ca("#FFFFFF", 0.85),
            palette_border: ca("#3730A3", 0.30),
            palette_match: c("#D97706"),
            divider: c("#E2DDD3"),
            focus_ring: c("#3730A3"),
            selection: ca("#3730A3", 0.18),
            cursor: c("#D97706"),
            shadow: ca("#000000", 0.20),
        }
    }
}

impl Default for CockpitTokens {
    fn default() -> Self {
        CockpitTokens::void()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn anchor_colors_match_spec() {
        let v = CockpitTokens::void();
        assert_eq!(v.bg, Color::from_hex("#0A0A0F"));
        assert_eq!(v.accent, Color::from_hex("#6E40C9"));
        assert_eq!(v.ai_highlight, Color::from_hex("#F59E0B"));

        assert_eq!(CockpitTokens::ash().accent, Color::from_hex("#7DCFFF"));
        assert_eq!(CockpitTokens::dusk().accent, Color::from_hex("#E6B450"));
        assert_eq!(CockpitTokens::light().accent, Color::from_hex("#3730A3"));
    }

    #[test]
    fn only_light_is_light() {
        assert!(CockpitTokens::void().is_dark);
        assert!(CockpitTokens::ash().is_dark);
        assert!(CockpitTokens::dusk().is_dark);
        assert!(!CockpitTokens::light().is_dark);
    }

    #[test]
    fn theme_dispatch_roundtrips() {
        for t in CockpitTheme::all() {
            assert_eq!(t.tokens().theme, t);
            assert_eq!(CockpitTokens::for_theme(t).theme, t);
        }
    }

    #[test]
    fn translucent_tokens_have_alpha() {
        assert!(CockpitTokens::void().selection.a < 1.0);
        assert!(CockpitTokens::void().palette_bg.a < 1.0);
        assert!(CockpitTokens::void().diff_added_bg.a < 1.0);
    }
}
