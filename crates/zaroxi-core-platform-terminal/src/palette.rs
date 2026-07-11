//! Terminal color palette and `vt100::Color` → linear RGBA resolution.
//!
//! The palette carries the surface foreground/background/cursor colors (wired
//! from the editor theme tokens) plus the 16 ANSI base colors. Indexed 256 and
//! truecolor values are computed on demand. Kept independent of any renderer so
//! it is deterministic and unit-testable.

/// RGBA color, matching the renderer's `[f32; 4]` convention (0.0..=1.0).
pub type Rgba = [f32; 4];

/// A resolved terminal color palette.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TerminalPalette {
    /// Color used for text with the default foreground.
    pub foreground: Rgba,
    /// Color used behind cells with the default background.
    pub background: Rgba,
    /// Block-cursor color.
    pub cursor: Rgba,
    /// The 16 ANSI base colors (0..7 normal, 8..15 bright).
    pub ansi: [Rgba; 16],
}

impl Default for TerminalPalette {
    fn default() -> Self {
        Self {
            foreground: [0.85, 0.85, 0.87, 1.0],
            background: [0.09, 0.09, 0.11, 1.0],
            cursor: [0.90, 0.90, 0.92, 1.0],
            ansi: DEFAULT_ANSI,
        }
    }
}

impl TerminalPalette {
    /// Build a palette from surface tokens (foreground/background/cursor),
    /// keeping the standard ANSI 16-color set. This is what the desktop layer
    /// uses to make the terminal feel native to the current theme.
    pub fn from_surface(foreground: Rgba, background: Rgba, cursor: Rgba) -> Self {
        Self { foreground, background, cursor, ansi: DEFAULT_ANSI }
    }

    /// Resolve a `vt100::Color` to RGBA.
    ///
    /// `is_fg` selects the default color when the cell uses the terminal
    /// default; `bold` brightens the low 8 ANSI indices (a common convention).
    pub fn resolve(&self, color: vt100::Color, is_fg: bool, bold: bool) -> Rgba {
        match color {
            vt100::Color::Default => {
                if is_fg {
                    self.foreground
                } else {
                    self.background
                }
            }
            vt100::Color::Idx(i) => self.resolve_indexed(i, bold),
            vt100::Color::Rgb(r, g, b) => rgb8(r, g, b),
        }
    }

    fn resolve_indexed(&self, idx: u8, bold: bool) -> Rgba {
        match idx {
            0..=7 => {
                let i = if bold { idx as usize + 8 } else { idx as usize };
                self.ansi[i.min(15)]
            }
            8..=15 => self.ansi[idx as usize],
            16..=231 => cube_color(idx),
            232..=255 => grayscale_color(idx),
        }
    }
}

fn rgb8(r: u8, g: u8, b: u8) -> Rgba {
    [r as f32 / 255.0, g as f32 / 255.0, b as f32 / 255.0, 1.0]
}

/// xterm 6×6×6 color cube for indices 16..=231.
fn cube_color(idx: u8) -> Rgba {
    const LEVELS: [u8; 6] = [0, 95, 135, 175, 215, 255];
    let v = (idx - 16) as usize;
    let r = LEVELS[(v / 36) % 6];
    let g = LEVELS[(v / 6) % 6];
    let b = LEVELS[v % 6];
    rgb8(r, g, b)
}

/// xterm 24-step grayscale ramp for indices 232..=255.
fn grayscale_color(idx: u8) -> Rgba {
    let level = 8u16 + (idx as u16 - 232) * 10;
    let l = level.min(255) as u8;
    rgb8(l, l, l)
}

/// Default ANSI 16-color set (a balanced dark-theme palette).
const DEFAULT_ANSI: [Rgba; 16] = [
    [0.157, 0.165, 0.180, 1.0], // 0 black
    [0.878, 0.298, 0.376, 1.0], // 1 red
    [0.427, 0.741, 0.412, 1.0], // 2 green
    [0.898, 0.753, 0.408, 1.0], // 3 yellow
    [0.388, 0.612, 0.918, 1.0], // 4 blue
    [0.769, 0.494, 0.867, 1.0], // 5 magenta
    [0.361, 0.741, 0.796, 1.0], // 6 cyan
    [0.831, 0.847, 0.867, 1.0], // 7 white
    [0.396, 0.420, 0.451, 1.0], // 8 bright black (gray)
    [0.937, 0.427, 0.494, 1.0], // 9 bright red
    [0.545, 0.831, 0.529, 1.0], // 10 bright green
    [0.945, 0.831, 0.529, 1.0], // 11 bright yellow
    [0.514, 0.706, 0.953, 1.0], // 12 bright blue
    [0.851, 0.612, 0.937, 1.0], // 13 bright magenta
    [0.478, 0.831, 0.878, 1.0], // 14 bright cyan
    [0.953, 0.957, 0.965, 1.0], // 15 bright white
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_and_rgb_colors() {
        let p = TerminalPalette::default();
        assert_eq!(p.resolve(vt100::Color::Default, true, false), p.foreground);
        assert_eq!(p.resolve(vt100::Color::Default, false, false), p.background);
        assert_eq!(p.resolve(vt100::Color::Rgb(255, 0, 0), true, false), [1.0, 0.0, 0.0, 1.0]);
    }

    #[test]
    fn ansi_indices_and_bold_brightening() {
        let p = TerminalPalette::default();
        assert_eq!(p.resolve(vt100::Color::Idx(1), true, false), p.ansi[1]);
        // Bold brightens the low 8 into the bright range.
        assert_eq!(p.resolve(vt100::Color::Idx(1), true, true), p.ansi[9]);
        // Explicit bright index is unchanged by bold.
        assert_eq!(p.resolve(vt100::Color::Idx(9), true, true), p.ansi[9]);
    }

    #[test]
    fn cube_and_grayscale() {
        // 16 is the cube origin (black), 231 is the cube max (white).
        assert_eq!(cube_color(16), [0.0, 0.0, 0.0, 1.0]);
        assert_eq!(cube_color(231), [1.0, 1.0, 1.0, 1.0]);
        // Grayscale endpoints.
        assert_eq!(grayscale_color(232), rgb8(8, 8, 8));
        assert_eq!(grayscale_color(255), rgb8(238, 238, 238));
    }

    #[test]
    fn from_surface_keeps_ansi() {
        let fg = [0.1, 0.2, 0.3, 1.0];
        let bg = [0.0, 0.0, 0.0, 1.0];
        let cur = [1.0, 1.0, 1.0, 1.0];
        let p = TerminalPalette::from_surface(fg, bg, cur);
        assert_eq!(p.foreground, fg);
        assert_eq!(p.background, bg);
        assert_eq!(p.cursor, cur);
        assert_eq!(p.ansi, DEFAULT_ANSI);
    }
}
