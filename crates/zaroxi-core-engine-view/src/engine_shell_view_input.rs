/// Engine-facing, non-visual semantic input derived from the interface shell frame.
///
/// This type intentionally carries only semantic, non-visual data:
/// - visible text lines,
/// - cursor/selection presence,
/// - viewport/status strings,
/// - optional decoration text.
///
/// Fields that were IDE-specific (last_command, ai_present) have been removed
/// in Phase 38 to keep the engine contract app-neutral. IDE-specific data
/// stays in the interface/application layers and is mapped into generic
/// engine primitives through adapters.
///
/// It MUST NOT contain any geometry, fonts, color, layout, rendering or GPU resources.
#[derive(Debug, Clone, PartialEq)]
pub struct EngineSelection {
    /// 0-based line index for selection start.
    pub start_line: u32,
    /// 0-based column index for selection start.
    pub start_column: u32,
    /// 0-based line index for selection end.
    pub end_line: u32,
    /// 0-based column index for selection end.
    pub end_column: u32,
}

#[derive(Debug, Clone, PartialEq)]
pub struct EngineShellViewInput {
    /// Top-most visible line index (1-based to match TextView semantics).
    pub top_line: u32,
    /// Total number of lines in the document.
    pub total_lines: u32,
    /// Visible text lines contained in the view window (cloned strings).
    pub lines: Vec<String>,

    /// Optional cursor line (1-based) when present.
    pub cursor_line: Option<u32>,
    /// Optional cursor column when present.
    pub cursor_column: Option<u32>,

    /// Optional selection bounds when present.
    pub selection: Option<EngineSelection>,

    /// Compact semantic viewport summary (opaque string).
    pub viewport_summary: Option<String>,

    /// Optional status text (generic; populated by adapters).
    pub status_text: Option<String>,

    /// Optional decoration text (generic; populated by adapters).
    pub decoration_text: Option<String>,
}

impl EngineShellViewInput {
    /// Construct an absent (empty) input. Useful when no active text view is present.
    pub fn absent() -> Self {
        Self {
            top_line: 0,
            total_lines: 0,
            lines: Vec::new(),
            cursor_line: None,
            cursor_column: None,
            selection: None,
            viewport_summary: None,
            status_text: None,
            decoration_text: None,
        }
    }
}
