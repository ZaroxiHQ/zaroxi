use serde::{Deserialize, Serialize};

/// Simple assistant/AI panel state for v1.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssistantState {
    /// Messages exchanged with the assistant (placeholder strings).
    pub messages: Vec<String>,
    /// Current input text.
    pub input: String,
    /// Selected model (placeholder).
    pub selected_model: String,
    /// Whether the assistant panel is visible.
    pub visible: bool,
}

impl Default for AssistantState {
    fn default() -> Self {
        Self {
            messages: Vec::new(),
            input: String::new(),
            selected_model: "gpt-4".to_string(),
            visible: true,
        }
    }
}
