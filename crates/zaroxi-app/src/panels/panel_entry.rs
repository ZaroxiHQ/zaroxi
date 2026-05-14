/// Panel metadata owned by the application layer.
///
/// This struct lives in its own module so the app orchestration code is
/// focused and the panel model can evolve independently of the root app file.
#[derive(Debug, Clone)]
pub struct PanelEntry {
    /// Stable panel identifier (used by registry/lookup).
    pub id: &'static str,
    /// Human facing title shown in the panel header.
    pub title: String,
    /// Whether the panel is visible.
    pub visible: bool,
    /// Small content placeholder (used by the initial v1 UI).
    pub content: String,
}

impl PanelEntry {
    pub fn new(id: &'static str, title: impl Into<String>, content: impl Into<String>, visible: bool) -> Self {
        Self {
            id,
            title: title.into(),
            visible,
            content: content.into(),
        }
    }
}
