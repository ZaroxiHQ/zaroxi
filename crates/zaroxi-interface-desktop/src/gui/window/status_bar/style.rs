//! Status bar styling tokens.
//!
//! Centralises the (restrained) colour choices for the status bar so the view
//! layer never hard-codes theme colours. Per-segment span colours are produced
//! by the engine's `format_status_bar_spans` chrome primitive; this struct
//! provides the bar's background fill and the default text colour. Selected/
//! active/error styles are intentionally omitted in Phase 1 — the goal is a
//! clean, quiet baseline.

use zaroxi_core_engine_style::StyleTokens;

/// Resolved colours for the status bar, derived from the active theme tokens.
#[derive(Clone, Copy, Debug)]
pub struct StatusStyle {
    /// Bar background fill.
    pub background: [f32; 4],
    /// Default bar text colour.
    pub primary_text: [f32; 4],
}

impl StatusStyle {
    /// Derive status bar colours from theme tokens.
    pub fn from_tokens(tokens: &StyleTokens) -> Self {
        Self {
            background: tokens.status_bar_background.to_array(),
            primary_text: tokens.text_secondary.to_array(),
        }
    }
}
