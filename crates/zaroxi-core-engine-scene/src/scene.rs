/// Tiny, semantic scene-description model used by the engine.
///
/// This model is intentionally minimal and semantic-only:
/// - carries text lines and viewport facts,
/// - preserves cursor/selection presence as a semantic signal,
/// - carries small semantic text blocks (status, chrome, last command),
/// - carries boolean flags for presence of AI indicators.
/// It does NOT contain any layout, pixel coordinates, colors, fonts, or GPU
/// resources. It is explicitly convertible from
/// `zaroxi_core_engine_view::EngineShellViewInput`.
use zaroxi_core_engine_view::EngineShellViewInput;
mod chrome;
pub use self::chrome::{ShellChrome, Tab};

/// Semantic, read-only scene model for the engine shell.
///
/// Keep this tiny and stable: it's a descriptive hand-off to later rendering
/// phases without any visual/layout concerns.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ShellSceneModel {
    /// Visible text lines (cloned strings).
    pub text_lines: Vec<String>,

    /// Top-most visible line index (1-based).
    pub viewport_top_line: u32,

    /// Total number of lines in the document.
    pub viewport_total_lines: u32,

    /// Optional compact viewport summary (opaque string).
    pub viewport_summary: Option<String>,

    /// Optional cursor line (1-based) when present.
    pub cursor_line: Option<u32>,

    /// Optional cursor column when present.
    pub cursor_column: Option<u32>,

    /// Whether a selection is present (semantic flag).
    pub selection_present: bool,

    /// Optional status bar text (semantic).
    pub status_text: Option<String>,

    /// Optional shell chrome/header text (semantic).
    pub chrome_text: Option<String>,

    /// Optional last command string (semantic).
    pub last_command: Option<String>,

    /// Whether an AI/status indicator is present (semantic flag).
    pub ai_status_present: bool,
}

impl From<EngineShellViewInput> for ShellSceneModel {
    fn from(src: EngineShellViewInput) -> Self {
        // Minimal semantic mapping from the view model into a scene description.
        // We purposefully do NOT attempt to compute layout or pixel metrics here.
        Self {
            text_lines: src.lines.clone(),
            viewport_top_line: src.top_line,
            viewport_total_lines: src.total_lines,
            viewport_summary: src.viewport_summary.clone(),
            cursor_line: src.cursor_line,
            cursor_column: src.cursor_column,
            // Selection presence is a semantic signal: either an explicit selection
            // or at least a cursor line present.
            selection_present: src.selection.is_some() || src.cursor_line.is_some(),
            status_text: src.status_text.clone(),
            chrome_text: src.shell_chrome.clone(),
            last_command: src.last_command.clone(),
            ai_status_present: src.ai_present,
        }
    }
}
