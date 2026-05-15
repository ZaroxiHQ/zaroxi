/// Simple panel content model (v1).
#[derive(Debug, Clone)]
pub enum PanelContent {
    /// Plain text placeholder content.
    Text(String),
}

impl From<String> for PanelContent {
    fn from(s: String) -> Self {
        PanelContent::Text(s)
    }
}

impl From<&str> for PanelContent {
    fn from(s: &str) -> Self {
        PanelContent::Text(s.to_string())
    }
}
