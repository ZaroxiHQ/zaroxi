//! Left-zone panel: diagnostics summary.
//!
//! Shows a compact error/warning count only when a diagnostics provider is
//! ready and actually reports something. Silent otherwise, so a clean buffer
//! adds no noise.

use super::super::model::StatusModel;

/// Build the diagnostics segment (e.g. `E 2  W 1`), or nothing when clean.
pub fn segments(model: &StatusModel) -> Vec<String> {
    let Some(counts) = &model.diagnostics else {
        return Vec::new();
    };

    let mut parts: Vec<String> = Vec::new();
    if counts.errors > 0 {
        parts.push(format!("E {}", counts.errors));
    }
    if counts.warnings > 0 {
        parts.push(format!("W {}", counts.warnings));
    }
    if parts.is_empty() {
        return Vec::new();
    }
    vec![parts.join(" ")]
}
