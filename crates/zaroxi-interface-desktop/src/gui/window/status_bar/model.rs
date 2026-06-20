//! Status bar presentation model.
//!
//! `StatusModel` is a small, typed view-model that the status bar panels read
//! from. It is derived once per frame from already-available app/editor state
//! (workspace, active file, cursor, buffer sample) so that panel rendering code
//! never has to reach into raw app internals. Keeping derivation here also makes
//! it the single place to extend with richer signals (git, diagnostics, tasks)
//! in later phases.

use zaroxi_core_engine_ui::ShellWorkContent;

/// Transient document/editor state surfaced on the left of the status bar.
///
/// Phase 1 distinguishes only "no file" from "ready"; richer transient states
/// (`Parsing`/`Saving`) arrive in the next phase once their live signals are
/// plumbed. The left panel already routes this field through a match, so adding
/// those variants is a localized change.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DocumentState {
    /// No file is open.
    NoFile,
    /// A file is open and idle.
    Ready,
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

/// Document line-ending convention.
///
/// `CrLf`/`Cr` are reserved for Phase 2, when raw line-ending detection is added;
/// the editor currently presents normalized `Lf`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LineEnding {
    /// `\n`
    Lf,
    /// `\r\n`
    #[allow(dead_code)]
    CrLf,
    /// `\r`
    #[allow(dead_code)]
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

/// Typed, render-ready view-model for the status bar.
#[derive(Clone, Debug)]
pub struct StatusModel {
    /// Workspace/project folder name, when a workspace is open.
    pub workspace: Option<String>,
    /// Transient document state (idle/parsing/saving).
    pub document_state: DocumentState,
    /// Caret line (0-based; displayed as 1-based).
    pub line: usize,
    /// Caret column (0-based; displayed as 1-based).
    pub column: usize,
    /// Indentation style of the active document.
    pub indent: IndentStyle,
    /// Text encoding label (UTF-8 only for now).
    pub encoding: &'static str,
    /// Line-ending convention.
    pub line_ending: LineEnding,
    /// Language / file-type label, when a file is open.
    pub language: Option<String>,
    /// Whether an editable file is currently active.
    pub has_file: bool,
}

impl Default for StatusModel {
    fn default() -> Self {
        Self {
            workspace: None,
            document_state: DocumentState::NoFile,
            line: 0,
            column: 0,
            indent: IndentStyle::Spaces(4),
            encoding: "UTF-8",
            line_ending: LineEnding::Lf,
            language: None,
            has_file: false,
        }
    }
}

impl StatusModel {
    /// Derive the model from the small set of state available at render time.
    ///
    /// * `work_content` — engine work DTO (provides the active file path).
    /// * `workspace_name` — workspace folder name (from the composition root).
    /// * `cursor_line` / `cursor_col` — 0-based caret position.
    /// * `indent_sample` — a small leading slice of the document used to detect
    ///   the indentation style; `None`/empty falls back to the editor default.
    pub fn from_sources(
        work_content: &Option<ShellWorkContent>,
        workspace_name: Option<&str>,
        cursor_line: usize,
        cursor_col: usize,
        indent_sample: Option<&str>,
    ) -> Self {
        let active_file = work_content.as_ref().and_then(|wc| wc.active_file.as_deref());
        let has_file = active_file.is_some();

        let language = active_file.map(language_label);
        let indent = indent_sample.map(detect_indent).unwrap_or(IndentStyle::Spaces(4));
        let document_state = if has_file { DocumentState::Ready } else { DocumentState::NoFile };

        Self {
            workspace: workspace_name.map(|s| s.to_string()),
            document_state,
            line: cursor_line,
            column: cursor_col,
            indent,
            encoding: "UTF-8",
            line_ending: LineEnding::Lf,
            language,
            has_file,
        }
    }
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

#[cfg(test)]
mod tests {
    use super::*;

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
    fn from_sources_without_file_is_quiet() {
        let model = StatusModel::from_sources(&None, None, 0, 0, None);
        assert!(!model.has_file);
        assert_eq!(model.document_state, DocumentState::NoFile);
        assert!(model.language.is_none());
        assert!(model.workspace.is_none());
    }

    #[test]
    fn from_sources_with_file_is_ready() {
        let wc = ShellWorkContent { active_file: Some("src/lib.rs".into()), ..Default::default() };
        let model = StatusModel::from_sources(&Some(wc), Some("zaroxi"), 4, 9, Some("\tindented"));
        assert!(model.has_file);
        assert_eq!(model.document_state, DocumentState::Ready);
        assert_eq!(model.language.as_deref(), Some("Rust"));
        assert_eq!(model.workspace.as_deref(), Some("zaroxi"));
        assert_eq!(model.indent, IndentStyle::Tabs(4));
        assert_eq!(model.line, 4);
        assert_eq!(model.column, 9);
    }
}
