/// Status bar state.
#[derive(Debug, Clone, Default)]
pub struct StatusState {
    /// Short one-line status message.
    pub message: String,
    /// Placeholder cursor position (line, column).
    pub cursor: (usize, usize),
    /// Editor mode (placeholder).
    pub mode: String,
}

impl StatusState {
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            cursor: (1, 1),
            mode: "normal".to_string(),
        }
    }
}
