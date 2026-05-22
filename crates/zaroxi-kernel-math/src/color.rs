#![allow(dead_code)]
/// Simple RGBA color with u8 components.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Color {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub a: u8,
}

impl Color {
    /// Create an RGBA color.
    pub const fn rgba(r: u8, g: u8, b: u8, a: u8) -> Self {
        Self { r, g, b, a }
    }

    /// Create an opaque RGB color.
    pub const fn rgb(r: u8, g: u8, b: u8) -> Self {
        Self { r, g, b, a: 255 }
    }

    /// Return a copy with given alpha.
    pub fn with_alpha(self, a: u8) -> Self {
        Self { a, ..self }
    }

    /// Convert to normalized f32 components in 0.0..=1.0 order [r,g,b,a].
    pub fn to_f32(self) -> [f32; 4] {
        [
            (self.r as f32) / 255.0,
            (self.g as f32) / 255.0,
            (self.b as f32) / 255.0,
            (self.a as f32) / 255.0,
        ]
    }

    /// Convert to raw u8 components [r,g,b,a].
    pub fn to_u8(self) -> [u8; 4] {
        [self.r, self.g, self.b, self.a]
    }

    /// Parse from hex string. Accepts "rrggbb", "#rrggbb", "rrggbbaa", "#rrggbbaa".
    pub fn from_hex(hex: &str) -> Result<Self, &'static str> {
        let s = hex.strip_prefix('#').unwrap_or(hex);
        match s.len() {
            6 => {
                let r = u8::from_str_radix(&s[0..2], 16).map_err(|_| "invalid hex")?;
                let g = u8::from_str_radix(&s[2..4], 16).map_err(|_| "invalid hex")?;
                let b = u8::from_str_radix(&s[4..6], 16).map_err(|_| "invalid hex")?;
                Ok(Self::rgb(r, g, b))
            }
            8 => {
                let r = u8::from_str_radix(&s[0..2], 16).map_err(|_| "invalid hex")?;
                let g = u8::from_str_radix(&s[2..4], 16).map_err(|_| "invalid hex")?;
                let b = u8::from_str_radix(&s[4..6], 16).map_err(|_| "invalid hex")?;
                let a = u8::from_str_radix(&s[6..8], 16).map_err(|_| "invalid hex")?;
                Ok(Self::rgba(r, g, b, a))
            }
            _ => Err("hex must be 6 or 8 hex digits"),
        }
    }
}

// Palette constants
pub const BG: Color = Color::rgba(13, 14, 17, 255);
pub const SURFACE: Color = Color::rgba(22, 24, 32, 255);
pub const SURFACE2: Color = Color::rgba(30, 33, 48, 255);
pub const BORDER: Color = Color::rgba(37, 40, 48, 255);
pub const ACCENT: Color = Color::rgba(59, 91, 219, 255);
pub const ACCENT_DIM: Color = Color::rgba(91, 138, 245, 255);
pub const TEXT: Color = Color::rgba(226, 228, 233, 255);
pub const TEXT_DIM: Color = Color::rgba(136, 145, 164, 255);
pub const TEXT_GHOST: Color = Color::rgba(99, 104, 120, 255);
pub const SYN_ORANGE: Color = Color::rgba(229, 131, 74, 255);
pub const SYN_CYAN: Color = Color::rgba(86, 182, 194, 255);
pub const SYN_PURPLE: Color = Color::rgba(198, 120, 221, 255);
pub const SYN_GREEN: Color = Color::rgba(152, 195, 121, 255);
pub const SYN_TEAL: Color = Color::rgba(78, 201, 176, 255);
pub const TERMINAL_GREEN: Color = Color::rgba(35, 209, 139, 255);
pub const GIT_GREEN: Color = Color::rgba(35, 209, 139, 255);
