use serde::{Deserialize, Serialize};

/// Bottom panel identifiers.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum BottomPanel {
    Terminal,
    Problems,
    Output,
}

impl Default for BottomPanel {
    fn default() -> Self {
        BottomPanel::Terminal
    }
}

/// State for the bottom panel area (tabbed).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BottomPanelState {
    /// Which bottom panel is active.
    pub active: BottomPanel,
    /// Whether the bottom area is visible.
    pub visible: bool,
}

impl Default for BottomPanelState {
    fn default() -> Self {
        Self {
            active: BottomPanel::default(),
            visible: true,
        }
    }
}
