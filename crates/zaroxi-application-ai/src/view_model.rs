// Minimal AI panel view models owned by application-ai

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct AiCard {
    pub title: String,
    pub body: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct AiPanelState {
    pub header: String,
    pub cards: Vec<AiCard>,
    pub composer_text: String,
}
