/*!
Theme adapter utilities for the window module.

Small helpers that interpret simple theme tokens (hex color strings) into
wgpu::Color values. Kept here so theme parsing is centralized and easy to
modify in later GUI phases.
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
