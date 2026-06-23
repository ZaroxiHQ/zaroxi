//! Theme and design system for Zaroxi
//! This crate provides color themes, design tokens, and styling utilities

pub mod cockpit;
pub mod colors;
pub mod manager;
pub mod theme;

pub use cockpit::{CockpitTheme, CockpitTokens};
pub use colors::*;
pub use manager::{ThemeManager, ThemeSettings};
pub use theme::{DesignTokens, SemanticColors, ZaroxiTheme};
