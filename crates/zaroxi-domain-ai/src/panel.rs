/// Domain-level content model for the AI assistant panel.
///
/// `AiPanelContent` is a pure data structure describing what the AI panel
/// should display — title, state, summary, body lines, and action hints —
/// without owning any geometry, colors, or rendering concerns.
///
/// Application crates populate this from AI proposal/request state;
/// Core composers convert it into engine-owned scene primitives.
#[derive(Debug, Clone)]
pub struct AiPanelContent {
    pub title: String,
    pub subtitle: String,
    pub kind: Option<String>,
    pub target_buffer: Option<String>,
    pub state: AiPanelState,
    pub summary: String,
    pub body_lines: Vec<String>,
    pub action_labels: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AiPanelState {
    Idle,
    Ready,
    Loading,
    Proposed,
    Applied,
    Failed,
}

impl AiPanelContent {
    /// Empty assistant panel shown when no AI session is active.
    pub fn idle() -> Self {
        Self {
            title: "Assistant".into(),
            subtitle: "No active AI session".into(),
            kind: None,
            target_buffer: None,
            state: AiPanelState::Idle,
            summary: String::new(),
            body_lines: vec!["Open a file and request an AI edit to get started.".into()],
            action_labels: vec![],
        }
    }
}
