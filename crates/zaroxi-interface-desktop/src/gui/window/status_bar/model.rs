//! Status bar presentation model.
//!
//! `StatusModel` is a small, typed view-model that the status bar panels read
//! from. It is derived once per frame from already-available app/editor state
//! (workspace, active file, cursor, selection, buffer sample, parse + diagnostics
//! signals) so that panel rendering code never has to reach into raw app
//! internals.
//!
//! Phase 2 wires real live signals. Fields backed by a genuine source are
//! populated; fields without a source yet (e.g. `readonly`) keep a clean,
//! honest default and act as extension seams rather than fabricated state.

/// Transient document/editor state surfaced on the left of the status bar.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DocumentState {
    /// No file is open.
    NoFile,
    /// A file is open and idle.
    Ready,
    /// A background syntax parse is in flight for the current buffer.
    Parsing,
}

/// Indentation style of the active document.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum IndentStyle {
    /// Soft tabs: `size` spaces per indent level.
    Spaces(usize),
    /// Hard tabs rendered at `width` columns.
    Tabs(usize),
}

impl IndentStyle {
    /// Short, IDE-style label (e.g. `Spaces: 4`, `Tabs: 4`).
    pub fn label(&self) -> String {
        match self {
            IndentStyle::Spaces(n) => format!("Spaces: {n}"),
            IndentStyle::Tabs(n) => format!("Tabs: {n}"),
        }
    }
}

/// Document line-ending convention, detected from the raw buffer head.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LineEnding {
    /// `\n`
    Lf,
    /// `\r\n`
    CrLf,
    /// `\r`
    Cr,
}

impl LineEnding {
    /// Short label used in the status bar.
    pub fn label(&self) -> &'static str {
        match self {
            LineEnding::Lf => "LF",
            LineEnding::CrLf => "CRLF",
            LineEnding::Cr => "CR",
        }
    }
}

/// Compact selection summary (only present while a selection is active).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SelectionInfo {
    /// Number of selected characters.
    pub chars: usize,
    /// Number of lines the selection spans (>= 1).
    pub lines: usize,
}

/// Compact diagnostics counts for the active buffer (only present when a
/// diagnostics provider is actually ready).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct DiagnosticCounts {
    pub errors: u32,
    pub warnings: u32,
    pub infos: u32,
    pub hints: u32,
}

impl DiagnosticCounts {
    /// Whether anything worth showing is present.
    pub fn any(&self) -> bool {
        self.errors > 0 || self.warnings > 0 || self.infos > 0 || self.hints > 0
    }
}

/// Raw inputs gathered by the app each frame, from which a [`StatusModel`] is
/// derived. Grouping them as a struct keeps the app call site readable and the
/// derivation logic in one place.
pub struct StatusInputs<'a> {
    /// Active document label (path or display name) — `Some` whenever a document
    /// is open. Drives `has_file` and the language label. Derived by the app from
    /// the best available signal (active file / breadcrumb / editor title) so the
    /// bar reflects the document the editor is actually showing.
    pub file_label: Option<&'a str>,
    /// Workspace folder name (from the composition root), if any.
    pub workspace_name: Option<&'a str>,
    /// 0-based caret line/column.
    pub cursor_line: usize,
    pub cursor_col: usize,
    /// Small leading raw slice of the document (line endings preserved) used to
    /// detect indentation style and line endings.
    pub text_sample: Option<&'a str>,
    /// Whether the buffer has unsaved edits since it was loaded.
    pub modified: bool,
    /// Whether a background syntax parse is currently pending.
    pub parsing: bool,
    /// Whether the document is read-only (no source yet — reserved seam).
    pub readonly: bool,
    /// Active selection summary, when a selection is present.
    pub selection: Option<SelectionInfo>,
    /// Diagnostics counts, only when a provider is ready.
    pub diagnostics: Option<DiagnosticCounts>,
}

/// Typed, render-ready view-model for the status bar.
#[derive(Clone, Debug)]
pub struct StatusModel {
    /// Workspace/project folder name, when a workspace is open.
    pub workspace: Option<String>,
    /// Transient document state (idle/parsing).
    pub document_state: DocumentState,
    /// Whether the document has unsaved edits.
    pub modified: bool,
    /// Whether the document is read-only.
    pub readonly: bool,
    /// Caret line (0-based; displayed as 1-based).
    pub line: usize,
    /// Caret column (0-based; displayed as 1-based).
    pub column: usize,
    /// Active selection summary, if any.
    pub selection: Option<SelectionInfo>,
    /// Indentation style of the active document.
    pub indent: IndentStyle,
    /// Text encoding label (UTF-8 only for now).
    pub encoding: &'static str,
    /// Line-ending convention.
    pub line_ending: LineEnding,
    /// Language / file-type label, when a file is open.
    pub language: Option<String>,
    /// Diagnostics counts, when a provider is ready and reports any.
    pub diagnostics: Option<DiagnosticCounts>,
    /// Whether an editable file is currently active.
    pub has_file: bool,
    /// Active document's display name (basename), when a file is open. Drives the
    /// always-present document-identity segment so the bar is never blank-looking.
    pub file_name: Option<String>,
}

impl Default for StatusModel {
    fn default() -> Self {
        Self {
            workspace: None,
            document_state: DocumentState::NoFile,
            modified: false,
            readonly: false,
            line: 0,
            column: 0,
            selection: None,
            indent: IndentStyle::Spaces(4),
            encoding: "UTF-8",
            line_ending: LineEnding::Lf,
            language: None,
            diagnostics: None,
            has_file: false,
            file_name: None,
        }
    }
}

impl StatusModel {
    /// Derive the model from the gathered live inputs.
    pub fn from_inputs(inputs: &StatusInputs<'_>) -> Self {
        let has_file = inputs.file_label.is_some();

        let language = inputs.file_label.map(language_label);
        let indent = inputs.text_sample.map(detect_indent).unwrap_or(IndentStyle::Spaces(4));
        let line_ending = inputs.text_sample.map(detect_line_ending).unwrap_or(LineEnding::Lf);

        let document_state = if !has_file {
            DocumentState::NoFile
        } else if inputs.parsing {
            DocumentState::Parsing
        } else {
            DocumentState::Ready
        };

        // Diagnostics are only surfaced when a provider actually reports some.
        let diagnostics = inputs.diagnostics.filter(|d| d.any());

        // Treat a blank/whitespace workspace name as "no workspace" so the
        // status bar shows the clean "No Workspace" fallback rather than an
        // invisible empty segment (a root cause of the bar reading as blank).
        let workspace =
            inputs.workspace_name.map(str::trim).filter(|s| !s.is_empty()).map(|s| s.to_string());

        let file_name = inputs.file_label.map(file_basename);

        Self {
            workspace,
            document_state,
            modified: has_file && inputs.modified,
            readonly: has_file && inputs.readonly,
            line: inputs.cursor_line,
            column: inputs.cursor_col,
            selection: if has_file { inputs.selection } else { None },
            indent,
            encoding: "UTF-8",
            line_ending,
            language,
            diagnostics,
            has_file,
            file_name,
        }
    }
}

/// Extract the display basename from an active-file label (path or name).
fn file_basename(path: &str) -> String {
    path.rsplit(['/', '\\']).next().filter(|s| !s.is_empty()).unwrap_or(path).to_string()
}

/// Map an active file path to a short, human-friendly language/file-type label.
fn language_label(path: &str) -> String {
    let ext = match path.rsplit_once('.') {
        Some((_, ext)) if !ext.is_empty() => ext,
        _ => return "Plain Text".to_string(),
    };

    let label = match ext {
        "rs" => "Rust",
        "toml" => "TOML",
        "md" => "Markdown",
        "json" => "JSON",
        "py" => "Python",
        "js" => "JavaScript",
        "ts" => "TypeScript",
        "txt" => "Text",
        other => return other.to_string(),
    };
    label.to_string()
}

/// Detect the indentation style from the first indented line in `sample`.
///
/// Conservative by design: a leading tab → hard tabs, a leading space run → soft
/// tabs (2 if exactly two leading spaces, otherwise 4). Falls back to the editor
/// default (`Spaces(4)`) when nothing is indented.
fn detect_indent(sample: &str) -> IndentStyle {
    for line in sample.lines() {
        match line.chars().next() {
            Some('\t') => return IndentStyle::Tabs(4),
            Some(' ') => {
                let leading = line.chars().take_while(|c| *c == ' ').count();
                let unit = if leading == 2 { 2 } else { 4 };
                return IndentStyle::Spaces(unit);
            }
            _ => {}
        }
    }
    IndentStyle::Spaces(4)
}

/// Detect the line-ending convention from the first terminator in the raw
/// `sample` (which preserves original `\r`/`\n` bytes). Defaults to `Lf` for
/// single-line or terminator-free content.
fn detect_line_ending(sample: &str) -> LineEnding {
    for (i, c) in sample.char_indices() {
        match c {
            '\r' => {
                if sample[i + 1..].starts_with('\n') {
                    return LineEnding::CrLf;
                }
                return LineEnding::Cr;
            }
            '\n' => return LineEnding::Lf,
            _ => {}
        }
    }
    LineEnding::Lf
}

#[cfg(test)]
mod tests {
    use super::*;

    fn inputs<'a>(file_label: Option<&'a str>) -> StatusInputs<'a> {
        StatusInputs {
            file_label,
            workspace_name: None,
            cursor_line: 0,
            cursor_col: 0,
            text_sample: None,
            modified: false,
            parsing: false,
            readonly: false,
            selection: None,
            diagnostics: None,
        }
    }

    #[test]
    fn language_label_known_and_unknown() {
        assert_eq!(language_label("src/main.rs"), "Rust");
        assert_eq!(language_label("Cargo.toml"), "TOML");
        assert_eq!(language_label("data.csv"), "csv");
        assert_eq!(language_label("Makefile"), "Plain Text");
    }

    #[test]
    fn detect_indent_tabs_spaces_and_default() {
        assert_eq!(detect_indent("fn main() {\n\tlet x = 1;\n}"), IndentStyle::Tabs(4));
        assert_eq!(detect_indent("fn main() {\n    let x = 1;\n}"), IndentStyle::Spaces(4));
        assert_eq!(detect_indent("- a\n  - b"), IndentStyle::Spaces(2));
        assert_eq!(detect_indent("no indentation here"), IndentStyle::Spaces(4));
    }

    #[test]
    fn detect_line_ending_variants() {
        assert_eq!(detect_line_ending("a\r\nb\r\n"), LineEnding::CrLf);
        assert_eq!(detect_line_ending("a\nb\n"), LineEnding::Lf);
        assert_eq!(detect_line_ending("a\rb"), LineEnding::Cr);
        assert_eq!(detect_line_ending("single line no terminator"), LineEnding::Lf);
    }

    #[test]
    fn from_inputs_without_file_is_quiet() {
        let model = StatusModel::from_inputs(&inputs(None));
        assert!(!model.has_file);
        assert_eq!(model.document_state, DocumentState::NoFile);
        assert!(model.language.is_none());
        assert!(!model.modified && !model.readonly);
        assert!(model.selection.is_none());
        assert!(model.diagnostics.is_none());
    }

    #[test]
    fn from_inputs_with_file_wires_live_state() {
        let mut i = inputs(Some("src/lib.rs"));
        i.workspace_name = Some("zaroxi");
        i.cursor_line = 4;
        i.cursor_col = 9;
        i.text_sample = Some("fn main() {\r\n\tlet x = 1;\r\n}");
        i.modified = true;
        i.parsing = true;
        i.selection = Some(SelectionInfo { chars: 12, lines: 2 });
        i.diagnostics = Some(DiagnosticCounts { errors: 1, warnings: 2, ..Default::default() });

        let model = StatusModel::from_inputs(&i);
        assert!(model.has_file);
        assert_eq!(model.document_state, DocumentState::Parsing);
        assert!(model.modified);
        assert_eq!(model.language.as_deref(), Some("Rust"));
        assert_eq!(model.workspace.as_deref(), Some("zaroxi"));
        assert_eq!(model.indent, IndentStyle::Tabs(4));
        assert_eq!(model.line_ending, LineEnding::CrLf);
        assert_eq!(model.selection, Some(SelectionInfo { chars: 12, lines: 2 }));
        assert_eq!(model.diagnostics.map(|d| (d.errors, d.warnings)), Some((1, 2)));
    }

    #[test]
    fn empty_diagnostics_are_dropped() {
        let mut i = inputs(Some("a.rs"));
        i.diagnostics = Some(DiagnosticCounts::default());
        let model = StatusModel::from_inputs(&i);
        assert!(model.diagnostics.is_none(), "zero-count diagnostics should not show");
    }
}
