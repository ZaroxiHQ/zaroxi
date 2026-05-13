/// Status bar state.
#[derive(Debug, Clone)]
pub struct StatusState {
    /// Short one-line status message.
    pub message: String,
    /// Placeholder cursor position (line, column).
    pub cursor: (usize, usize),
    /// Editor mode (placeholder).
    pub mode: String,
}

impl Default for StatusState {
    fn default() -> Self {
        Self {
            message: "Ready".to_string(),
            cursor: (1, 1),
            mode: "normal".to_string(),
        }
    }
}
