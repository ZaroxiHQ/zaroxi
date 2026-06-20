//! Right-zone panel: file format details.
//!
//! Indentation, encoding, line endings, and language/file-type — the contextual
//! per-file information you expect on the right of an IDE status bar. Shown only
//! when a file is open so empty states stay clean.

use super::super::model::StatusModel;

/// Build the file-format segments (indent, encoding, line endings, language).
pub fn segments(model: &StatusModel) -> Vec<String> {
    if !model.has_file {
        return Vec::new();
    }

    let mut out = Vec::new();
    out.push(model.indent.label());
    out.push(model.encoding.to_string());
    out.push(model.line_ending.label().to_string());
    if let Some(language) = &model.language {
        out.push(language.clone());
    }
    out
}
