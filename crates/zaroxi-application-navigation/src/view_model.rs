// Simple app rail view model owned by application-navigation
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct AppRailState {
    pub icons: Vec<String>,
    pub active: Option<String>,
}

impl AppRailState {
    pub fn default_icons() -> Vec<String> {
        vec![
            "explorer".to_string(),
            "search".to_string(),
            "git".to_string(),
            "extensions".to_string(),
        ]
    }
}
