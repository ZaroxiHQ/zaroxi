/*!
Theme adapter utilities for the window module.

Small helpers that interpret simple theme tokens (hex color strings) into
wgpu::Color values. Kept here so theme parsing is centralized and easy to
modify in later GUI phases.

Changes for GUI-5:
- Added a small `adjust_brightness` helper to derive subtle variations of
  the theme tokens (lighten/darken) so the shell regions can be visually
  distinguished while keeping the theme as the single source of truth.
*/

/// Helper: parse hex "#rrggbb" -> wgpu::Color (srgb approx).
pub fn parse_hex_color(s: &str) -> wgpu::Color {
    let s = s.trim_start_matches('#');
    if s.len() == 6 {
        if let Ok(v) = u32::from_str_radix(s, 16) {
            let r = ((v >> 16) & 0xFF) as f64 / 255.0;
            let g = ((v >> 8) & 0xFF) as f64 / 255.0;
            let b = (v & 0xFF) as f64 / 255.0;
            return wgpu::Color { r, g, b, a: 1.0 };
        }
    }
    // fallback: use a neutral, non-branded technical default to avoid
    // hardcoding product colors in window glue.
    eprintln!("GuiApp: parse_hex_color: invalid hex '{}', falling back to neutral black", s);
    wgpu::Color { r: 0.0, g: 0.0, b: 0.0, a: 1.0 }
}

/// Derive a subtle brightness-adjusted color from the theme hex token.
///
/// factor > 1.0 -> lighter, factor < 1.0 -> darker. Values are clamped to [0.0,1.0].
/// This is intentionally simple (linear scale) and lives in the interface layer
/// so we don't introduce a second theme system in the render backend.
pub fn adjust_brightness(s: &str, factor: f64) -> wgpu::Color {
    let s_trim = s.trim_start_matches('#');
    if s_trim.len() == 6 {
        if let Ok(v) = u32::from_str_radix(s_trim, 16) {
            let mut r = ((v >> 16) & 0xFF) as f64 / 255.0;
            let mut g = ((v >> 8) & 0xFF) as f64 / 255.0;
            let mut b = (v & 0xFF) as f64 / 255.0;
            r = (r * factor).clamp(0.0, 1.0);
            g = (g * factor).clamp(0.0, 1.0);
            b = (b * factor).clamp(0.0, 1.0);
            return wgpu::Color { r, g, b, a: 1.0 };
        }
    }
    eprintln!("GuiApp: adjust_brightness: invalid hex '{}', falling back to neutral black", s);
    wgpu::Color { r: 0.0, g: 0.0, b: 0.0, a: 1.0 }
}
