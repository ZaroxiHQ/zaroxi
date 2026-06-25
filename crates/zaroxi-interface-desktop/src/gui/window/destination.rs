/*!
Workbench destinations — the single routing concept that drives the rendered
shell when an activity-rail item is selected.

A `WorkbenchDestination` is derived from `GuiApp::rail_selected_index` once per
frame and then drives, coherently and visibly:

- the **sidebar** content ([`sidebar_rows`] + [`WorkbenchDestination::sidebar_title`]),
- the **editor region** (file editor for `Explorer`, otherwise a cockpit page),
- the **tab strip / breadcrumb** label ([`WorkbenchDestination::title`]),
- the **cockpit panels** (Settings / Extensions / placeholder), which read the
  same mock data exposed here so the sidebar and the main pane never disagree.

The Extensions list and Settings categories are backed by a small local mock
model ([`extension_entries`], [`settings_sections`]). A future theme/extension
system can replace these with real providers without changing the render
contract.
*/

use zaroxi_interface_widgets::{ExtensionEntry, SettingsRow, SettingsRowKind, SettingsSection};

/// A first-class destination selected from the activity rail.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WorkbenchDestination {
    /// File tree + editor cockpit (the default).
    Explorer,
    /// Full-text search across the workspace.
    Search,
    /// Source-control (git) changes.
    SourceControl,
    /// Run & debug.
    Debug,
    /// Extensions marketplace / manager.
    Extensions,
    /// Application settings.
    Settings,
    /// Account / sign-in.
    Account,
}

impl WorkbenchDestination {
    /// Map a rail index to a destination. Out-of-range falls back to `Explorer`.
    pub fn from_rail_index(index: usize) -> Self {
        match index {
            0 => Self::Explorer,
            1 => Self::Search,
            2 => Self::SourceControl,
            3 => Self::Debug,
            4 => Self::Extensions,
            5 => Self::Settings,
            6 => Self::Account,
            _ => Self::Explorer,
        }
    }

    /// Whether this destination is the file-editing Explorer (the only one that
    /// renders the file editor + explorer tree).
    pub fn is_explorer(&self) -> bool {
        matches!(self, Self::Explorer)
    }

    /// Human title used for the tab strip / breadcrumb / main heading.
    pub fn title(&self) -> &'static str {
        match self {
            Self::Explorer => "Explorer",
            Self::Search => "Search",
            Self::SourceControl => "Source Control",
            Self::Debug => "Run & Debug",
            Self::Extensions => "Extensions",
            Self::Settings => "Settings",
            Self::Account => "Account",
        }
    }

    /// Title shown above the destination's sidebar panel.
    pub fn sidebar_title(&self) -> &'static str {
        match self {
            Self::Explorer => "Explorer",
            Self::Search => "Search",
            Self::SourceControl => "Source Control",
            Self::Debug => "Run & Debug",
            Self::Extensions => "Extensions",
            Self::Settings => "Settings",
            Self::Account => "Account",
        }
    }

    /// `(title, subtitle)` for destinations rendered as a titled placeholder in
    /// the editor content region (no bespoke page yet). `None` for destinations
    /// with their own page (Explorer/Settings/Extensions).
    pub fn placeholder(&self) -> Option<(String, String)> {
        let (title, subtitle) = match self {
            Self::Search => ("Search", "Find across files in the workspace."),
            Self::SourceControl => ("Source Control", "Review and stage changes."),
            Self::Debug => ("Run & Debug", "Launch and inspect debug sessions."),
            Self::Account => ("Account", "Sign in to sync settings and extensions."),
            _ => return None,
        };
        Some((title.to_string(), subtitle.to_string()))
    }
}

/// One row in a destination's sidebar list.
#[derive(Debug, Clone)]
pub struct DestSidebarRow {
    /// Primary label (extension name, settings category, etc.).
    pub label: String,
    /// Trailing badge / hint (e.g. "Installed", "Available", or empty).
    pub secondary: String,
    /// Whether this row is the active selection (drives the highlight + detail).
    pub selected: bool,
    /// Whether clicking this row changes the active selection (Extensions /
    /// Settings). Decorative rows for other destinations are not selectable.
    pub selectable: bool,
}

impl DestSidebarRow {
    fn new(label: &str, secondary: &str, selected: bool, selectable: bool) -> Self {
        Self { label: label.to_string(), secondary: secondary.to_string(), selected, selectable }
    }
}

/// Mock extension catalogue. A future extension system replaces this provider.
pub fn extension_entries() -> Vec<ExtensionEntry> {
    vec![
        ExtensionEntry {
            id: "zaroxi.formatter".into(),
            name: "Zaroxi Formatter".into(),
            publisher: "Zaroxi Team".into(),
            description: "Code formatting for Rust, TOML, JSON, and more.".into(),
            installed: true,
        },
        ExtensionEntry {
            id: "zaroxi.lsp-client".into(),
            name: "LSP Client".into(),
            publisher: "Zaroxi Team".into(),
            description: "Language Server Protocol client for Rust, TypeScript, Python.".into(),
            installed: true,
        },
        ExtensionEntry {
            id: "community.themes".into(),
            name: "Community Themes".into(),
            publisher: "Community".into(),
            description: "A collection of popular color themes.".into(),
            installed: false,
        },
        ExtensionEntry {
            id: "zaroxi.git".into(),
            name: "Git Integration".into(),
            publisher: "Zaroxi Team".into(),
            description: "Inline blame, diff gutters, and commit tooling.".into(),
            installed: false,
        },
    ]
}

/// Mock settings sections. `theme_label` is the live theme mode so the General
/// section reflects real app state; the rest are clearly stubbed values.
pub fn settings_sections(theme_label: &str) -> Vec<SettingsSection> {
    vec![
        SettingsSection {
            label: "General".to_string(),
            items: vec![
                SettingsRow {
                    label: "Color Theme".to_string(),
                    description: "Active theme for the IDE".to_string(),
                    kind: SettingsRowKind::Label { value: theme_label.to_string() },
                },
                SettingsRow {
                    label: "Telemetry".to_string(),
                    description: "Send anonymous usage data".to_string(),
                    kind: SettingsRowKind::Toggle { on: false },
                },
            ],
        },
        SettingsSection {
            label: "Editor".to_string(),
            items: vec![
                SettingsRow {
                    label: "Tab Size".to_string(),
                    description: "Spaces per tab".to_string(),
                    kind: SettingsRowKind::Label { value: "4".to_string() },
                },
                SettingsRow {
                    label: "Word Wrap".to_string(),
                    description: "Wrap long lines to the viewport".to_string(),
                    kind: SettingsRowKind::Toggle { on: false },
                },
            ],
        },
        SettingsSection {
            label: "Appearance".to_string(),
            items: vec![
                SettingsRow {
                    label: "Font Size".to_string(),
                    description: "Default editor font size".to_string(),
                    kind: SettingsRowKind::Label { value: "13 px".to_string() },
                },
                SettingsRow {
                    label: "Minimap".to_string(),
                    description: "Show the semantic minimap".to_string(),
                    kind: SettingsRowKind::Toggle { on: true },
                },
            ],
        },
        SettingsSection {
            label: "Keybindings".to_string(),
            items: vec![SettingsRow {
                label: "Scheme".to_string(),
                description: "Keyboard shortcut scheme".to_string(),
                kind: SettingsRowKind::Select {
                    value: "Default".to_string(),
                    options: vec!["Default".to_string(), "Vim".to_string()],
                },
            }],
        },
        SettingsSection {
            label: "Extensions".to_string(),
            items: vec![SettingsRow {
                label: "Auto Update".to_string(),
                description: "Keep installed extensions up to date".to_string(),
                kind: SettingsRowKind::Toggle { on: true },
            }],
        },
    ]
}

/// Build the destination's sidebar rows. For Extensions/Settings these are the
/// selectable list driving the detail pane; for the placeholder destinations
/// they are decorative facets so the sidebar visibly changes per destination.
pub fn sidebar_rows(
    dest: WorkbenchDestination,
    extensions_selected: usize,
    settings_selected: usize,
) -> Vec<DestSidebarRow> {
    match dest {
        WorkbenchDestination::Explorer => Vec::new(),
        WorkbenchDestination::Extensions => extension_entries()
            .iter()
            .enumerate()
            .map(|(i, e)| {
                let badge = if e.installed { "Installed" } else { "Available" };
                DestSidebarRow::new(&e.name, badge, i == extensions_selected, true)
            })
            .collect(),
        WorkbenchDestination::Settings => settings_sections("")
            .iter()
            .enumerate()
            .map(|(i, s)| DestSidebarRow::new(&s.label, "", i == settings_selected, true))
            .collect(),
        WorkbenchDestination::Search => vec![
            DestSidebarRow::new("Match Case", "Aa", false, false),
            DestSidebarRow::new("Whole Word", "ab", false, false),
            DestSidebarRow::new("Use Regex", ".*", false, false),
        ],
        WorkbenchDestination::SourceControl => vec![
            DestSidebarRow::new("Changes", "0", false, false),
            DestSidebarRow::new("Staged Changes", "0", false, false),
            DestSidebarRow::new("Commits", "", false, false),
        ],
        WorkbenchDestination::Debug => vec![
            DestSidebarRow::new("Run and Debug", "", false, false),
            DestSidebarRow::new("Breakpoints", "0", false, false),
            DestSidebarRow::new("Watch", "", false, false),
        ],
        WorkbenchDestination::Account => vec![
            DestSidebarRow::new("Sign In", "", false, false),
            DestSidebarRow::new("Profile", "", false, false),
            DestSidebarRow::new("Sync Settings", "Off", false, false),
        ],
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rail_index_maps_to_destination() {
        assert_eq!(WorkbenchDestination::from_rail_index(0), WorkbenchDestination::Explorer);
        assert_eq!(WorkbenchDestination::from_rail_index(4), WorkbenchDestination::Extensions);
        assert_eq!(WorkbenchDestination::from_rail_index(5), WorkbenchDestination::Settings);
        // Out of range falls back to Explorer.
        assert_eq!(WorkbenchDestination::from_rail_index(99), WorkbenchDestination::Explorer);
    }

    #[test]
    fn only_explorer_is_explorer() {
        assert!(WorkbenchDestination::Explorer.is_explorer());
        assert!(!WorkbenchDestination::Settings.is_explorer());
        assert!(!WorkbenchDestination::Extensions.is_explorer());
    }

    #[test]
    fn placeholder_only_for_facet_destinations() {
        assert!(WorkbenchDestination::Search.placeholder().is_some());
        assert!(WorkbenchDestination::Account.placeholder().is_some());
        // Pages with bespoke views have no placeholder.
        assert!(WorkbenchDestination::Explorer.placeholder().is_none());
        assert!(WorkbenchDestination::Settings.placeholder().is_none());
        assert!(WorkbenchDestination::Extensions.placeholder().is_none());
    }

    #[test]
    fn extensions_sidebar_marks_selection() {
        let rows = sidebar_rows(WorkbenchDestination::Extensions, 1, 0);
        assert!(rows[1].selected);
        assert!(rows[0].selectable);
        assert!(!rows[0].selected);
    }

    #[test]
    fn explorer_has_no_destination_rows() {
        assert!(sidebar_rows(WorkbenchDestination::Explorer, 0, 0).is_empty());
    }
}
