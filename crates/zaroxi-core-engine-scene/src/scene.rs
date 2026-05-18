/// Tiny, semantic scene-description model used by the engine.
///
/// This model is intentionally minimal and semantic-only:
/// - carries text lines and viewport facts,
/// - preserves cursor/selection presence as a semantic signal,
/// - carries boolean flags for presence of status/chrome/AI indicators.
/// It does NOT contain any layout, pixel coordinates, colors, fonts, or GPU
/// resources. It is explicitly convertible from
/// `zaroxi_core_engine_view::EngineShellViewInput`.
use zaroxi_core_engine_view::EngineShellViewInput;

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

    /// Optional cursor line (1-based) when present.
    pub cursor_line: Option<u32>,

    /// Whether a selection is present. For this Phase we conservatively treat
    /// the presence of a cursor as a proxy for selection/cursor state.
    pub selection_present: bool,

    /// Whether a status block is present (semantic flag only).
    pub status_present: bool,

    /// Whether chrome/header text is present (semantic flag only).
    pub chrome_present: bool,

    /// Whether an AI/status indicator is present (semantic flag only).
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
            cursor_line: src.cursor_line,
            // Phase-50 conservative defaults:
            selection_present: src.cursor_line.is_some(),
            status_present: false,
            chrome_present: false,
            ai_status_present: false,
        }
    }
}
