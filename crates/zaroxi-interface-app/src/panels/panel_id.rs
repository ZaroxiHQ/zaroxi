/// Stable panel identifiers used by the application.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PanelId {
    TitleBar,
    Sidebar,
    Editor,
    RightPanel,
    BottomPanel,
    StatusBar,
}

impl PanelId {
    pub fn as_str(&self) -> &'static str {
        match self {
            PanelId::TitleBar => "titlebar",
            PanelId::Sidebar => "sidebar",
            PanelId::Editor => "editor",
            PanelId::RightPanel => "right_panel",
            PanelId::BottomPanel => "bottom_panel",
            PanelId::StatusBar => "status_bar",
        }
    }
}
