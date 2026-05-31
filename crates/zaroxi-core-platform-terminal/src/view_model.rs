// Minimal terminal panel view model

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct TerminalLine {
    pub text: String,
    pub level: u8,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct TerminalPanelState {
    pub tabs: Vec<String>,
    pub active_tab: usize,
    pub lines: Vec<TerminalLine>,
}
