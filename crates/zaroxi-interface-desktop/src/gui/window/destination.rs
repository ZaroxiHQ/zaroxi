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

    /// The activity-rail index that selects this destination (inverse of
    /// [`from_rail_index`]). Used to keep the rail highlight in sync with the
    /// active tab.
    pub fn rail_index(&self) -> usize {
        match self {
            Self::Explorer => 0,
            Self::Search => 1,
            Self::SourceControl => 2,
            Self::Debug => 3,
            Self::Extensions => 4,
            Self::Settings => 5,
            Self::Account => 6,
        }
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

/// Identity of a workbench tab. The single source of truth for what the center
/// content shows. `Editor` collapses *all* file tabs (the active file tab is the
/// active buffer); the other variants are first-class non-file workbench tabs.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WorkbenchTabId {
    /// The file editor (whichever buffer is active). Explorer destination.
    Editor,
    /// A destination landing/root tab (Search / Source Control / Debug /
    /// Extensions / Settings / Account). Explorer never gets a root tab — it is
    /// the editor.
    DestinationRoot(WorkbenchDestination),
    /// A specific Settings category page (index into the settings sections).
    SettingsSection(usize),
    /// A specific extension detail page (by extension id).
    ExtensionDetail(String),
}

impl WorkbenchTabId {
    /// The workbench area this tab belongs to — drives the sidebar + rail.
    pub fn destination(&self) -> WorkbenchDestination {
        match self {
            Self::Editor => WorkbenchDestination::Explorer,
            Self::DestinationRoot(d) => *d,
            Self::SettingsSection(_) => WorkbenchDestination::Settings,
            Self::ExtensionDetail(_) => WorkbenchDestination::Extensions,
        }
    }

    /// Whether this tab shows the file editor (vs a cockpit destination page).
    pub fn is_editor(&self) -> bool {
        matches!(self, Self::Editor | Self::DestinationRoot(WorkbenchDestination::Explorer))
    }
}

/// An open non-file workbench tab (file tabs are projected from the buffer
/// summary each frame and never stored here).
#[derive(Debug, Clone)]
pub struct WorkbenchTab {
    /// Stable identity (used for focus + dedup).
    pub id: WorkbenchTabId,
    /// Visible tab label, e.g. "Settings: General", "Zaroxi Formatter".
    pub title: String,
}

/// A rendered tab's hit region, set each frame from the cockpit tab-strip
/// layout so the host can route clicks (focus / close) without the widget tree.
#[derive(Debug, Clone)]
pub struct WorkbenchTabHit {
    /// Whole-tab hit rect `(x, y, w, h)`.
    pub rect: (f32, f32, f32, f32),
    /// Close-button hit rect, when the tab is closable.
    pub close_rect: Option<(f32, f32, f32, f32)>,
    /// `Some(buffer index)` for a file tab (routes to buffer switch); `None`
    /// for a non-file tab.
    pub file_index: Option<usize>,
    /// Tab identity (`Editor` for file tabs).
    pub id: WorkbenchTabId,
}

/// Title for a non-file tab id (used when opening a tab).
pub fn tab_title(id: &WorkbenchTabId) -> String {
    match id {
        WorkbenchTabId::Editor => "Editor".to_string(),
        WorkbenchTabId::DestinationRoot(d) => d.title().to_string(),
        WorkbenchTabId::SettingsSection(i) => {
            let sections = settings_sections("");
            let name = sections.get(*i).map(|s| s.label.as_str()).unwrap_or("Settings");
            format!("Settings: {name}")
        }
        WorkbenchTabId::ExtensionDetail(ext_id) => extension_entries()
            .into_iter()
            .find(|e| &e.id == ext_id)
            .map(|e| e.name)
            .unwrap_or_else(|| "Extension".to_string()),
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
    extensions_selected: Option<usize>,
    settings_selected: Option<usize>,
) -> Vec<DestSidebarRow> {
    match dest {
        WorkbenchDestination::Explorer => Vec::new(),
        WorkbenchDestination::Extensions => extension_entries()
            .iter()
            .enumerate()
            .map(|(i, e)| {
                let badge = if e.installed { "Installed" } else { "Available" };
                DestSidebarRow::new(&e.name, badge, Some(i) == extensions_selected, true)
            })
            .collect(),
        WorkbenchDestination::Settings => settings_sections("")
            .iter()
            .enumerate()
            .map(|(i, s)| DestSidebarRow::new(&s.label, "", Some(i) == settings_selected, true))
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

/// A tab in the unified tab strip (file tabs + non-file workbench tabs).
#[derive(Debug, Clone)]
pub struct UnifiedTab {
    /// Visible label.
    pub title: String,
    /// Whether this is the active tab.
    pub active: bool,
    /// Whether a close button is shown (non-file tabs are closable).
    pub closable: bool,
    /// `Some(buffer index)` for a file tab, `None` for a non-file tab.
    pub file_index: Option<usize>,
    /// Tab identity (`Editor` for file tabs).
    pub id: WorkbenchTabId,
}

/// Format a file path into a compact, filename-first tab label.
///
/// Rules (matches common IDE tab-label conventions):
/// - Single file: basename only (e.g. `AGENTS.md`)
/// - Unique basenames among all open files: basename only
/// - Duplicate basenames: add one parent-dir segment (e.g. `src/main.rs`,
///   `client/main.rs`)
/// - Paths that look like non-filesystem display names are returned as-is.
pub fn format_file_tab_label(path: &str, all_paths: &[&str]) -> String {
    // Guard: paths without any separator are likely display names, not
    // filesystem paths. Return them unchanged.
    if !path.contains('/') && !path.contains('\\') {
        return path.to_string();
    }

    let basename = std::path::Path::new(path).file_name().and_then(|n| n.to_str()).unwrap_or(path);

    // Collect all basenames from sibling paths.
    let same_basename_count = all_paths
        .iter()
        .filter(|p| {
            std::path::Path::new(p)
                .file_name()
                .and_then(|n| n.to_str())
                .map(|b| b == basename)
                .unwrap_or(false)
        })
        .count();

    // Unique — just the basename.
    if same_basename_count <= 1 {
        return basename.to_string();
    }

    // Duplicate: add one parent segment for disambiguation.
    let parent = std::path::Path::new(path)
        .parent()
        .and_then(|p| p.file_name())
        .and_then(|n| n.to_str())
        .unwrap_or("");

    if parent.is_empty() { format!("../{basename}") } else { format!("{parent}/{basename}") }
}

/// Project the unified tab strip from the file buffers (as `(label, is_active)`
/// pairs) followed by the open non-file workbench tabs. File tabs are only
/// highlighted when the editor itself is the active tab.
pub fn build_unified_tabs(
    file_tabs: &[(String, bool)],
    active_tab: &WorkbenchTabId,
    non_file_tabs: &[WorkbenchTab],
) -> Vec<UnifiedTab> {
    let mut out = Vec::new();
    let editor_active = active_tab.is_editor();
    let all_paths: Vec<&str> = file_tabs.iter().map(|(t, _)| t.as_str()).collect();
    for (i, (title, is_active)) in file_tabs.iter().enumerate() {
        let compact = format_file_tab_label(title, &all_paths);
        out.push(UnifiedTab {
            title: compact,
            active: editor_active && *is_active,
            closable: false,
            file_index: Some(i),
            id: WorkbenchTabId::Editor,
        });
    }
    for t in non_file_tabs {
        out.push(UnifiedTab {
            title: t.title.clone(),
            active: active_tab == &t.id,
            closable: true,
            file_index: None,
            id: t.id.clone(),
        });
    }
    out
}

/// Resolve the cockpit destination pages for the active tab: returns
/// `(settings_panel, extensions_panel, placeholder_panel)`. Exactly one (or
/// none, for the editor) is `Some`.
#[allow(clippy::type_complexity)]
pub fn cockpit_panels_for(
    active: &WorkbenchTabId,
    theme_label: &str,
) -> (
    Option<(Vec<SettingsSection>, usize)>,
    Option<(Vec<ExtensionEntry>, usize)>,
    Option<(String, String)>,
) {
    match active {
        WorkbenchTabId::SettingsSection(i) => {
            let sections = settings_sections(theme_label);
            let sel = (*i).min(sections.len().saturating_sub(1));
            (Some((sections, sel)), None, None)
        }
        WorkbenchTabId::DestinationRoot(WorkbenchDestination::Settings) => {
            (Some((settings_sections(theme_label), 0)), None, None)
        }
        WorkbenchTabId::ExtensionDetail(id) => {
            let entries = extension_entries();
            let sel = entries.iter().position(|e| &e.id == id).unwrap_or(0);
            (None, Some((entries, sel)), None)
        }
        WorkbenchTabId::DestinationRoot(WorkbenchDestination::Extensions) => {
            (None, Some((extension_entries(), 0)), None)
        }
        WorkbenchTabId::DestinationRoot(d) => (None, None, d.placeholder()),
        WorkbenchTabId::Editor => (None, None, None),
    }
}

/// Sidebar selection highlight `(extensions_selected, settings_selected)`
/// derived from the active tab — no separate selection state is stored.
pub fn sidebar_selection_for(active: &WorkbenchTabId) -> (Option<usize>, Option<usize>) {
    match active {
        WorkbenchTabId::ExtensionDetail(id) => {
            (extension_entries().iter().position(|e| &e.id == id), None)
        }
        WorkbenchTabId::SettingsSection(i) => (None, Some(*i)),
        _ => (None, None),
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
        let rows = sidebar_rows(WorkbenchDestination::Extensions, Some(1), None);
        assert!(rows[1].selected);
        assert!(rows[0].selectable);
        assert!(!rows[0].selected);
    }

    #[test]
    fn explorer_has_no_destination_rows() {
        assert!(sidebar_rows(WorkbenchDestination::Explorer, None, None).is_empty());
    }

    #[test]
    fn unified_tabs_orders_files_then_non_file_and_flags_active() {
        let files = vec![("src/main.rs".to_string(), true), ("src/lib.rs".to_string(), false)];
        let non_file = vec![
            WorkbenchTab {
                id: WorkbenchTabId::DestinationRoot(WorkbenchDestination::Settings),
                title: "Settings".to_string(),
            },
            WorkbenchTab {
                id: WorkbenchTabId::ExtensionDetail("zaroxi.git".to_string()),
                title: "Git Integration".to_string(),
            },
        ];

        // Editor active: the active file tab is highlighted, no non-file tab is.
        let tabs = build_unified_tabs(&files, &WorkbenchTabId::Editor, &non_file);
        assert_eq!(tabs.len(), 4);
        assert_eq!(tabs[0].title, "main.rs");
        assert!(tabs[0].active && tabs[0].file_index == Some(0) && !tabs[0].closable);
        assert!(!tabs[1].active); // lib.rs not the active buffer
        assert!(!tabs[2].active && tabs[2].closable); // Settings tab
        assert!(!tabs[3].active && tabs[3].closable); // extension tab

        // A non-file tab active: no file tab is highlighted, that tab is.
        let ext = WorkbenchTabId::ExtensionDetail("zaroxi.git".to_string());
        let tabs = build_unified_tabs(&files, &ext, &non_file);
        assert!(!tabs[0].active && !tabs[1].active);
        assert!(tabs[3].active);
    }

    #[test]
    fn format_file_tab_disambiguates_duplicates() {
        let paths: Vec<&str> = vec!["src/main.rs", "client/main.rs", "src/lib.rs"];
        assert_eq!(format_file_tab_label("src/main.rs", &paths), "src/main.rs");
        assert_eq!(format_file_tab_label("client/main.rs", &paths), "client/main.rs");
        assert_eq!(format_file_tab_label("src/lib.rs", &paths), "lib.rs");
    }

    #[test]
    fn format_file_tab_uses_basename_for_unique_names() {
        let paths: Vec<&str> = vec!["src/main.rs", "src/lib.rs"];
        assert_eq!(format_file_tab_label("src/main.rs", &paths), "main.rs");
        assert_eq!(format_file_tab_label("AGENTS.md", &paths), "AGENTS.md");
    }

    #[test]
    fn panels_follow_active_tab() {
        // Settings section -> settings panel at that index.
        let (s, e, p) = cockpit_panels_for(&WorkbenchTabId::SettingsSection(2), "Dark");
        assert!(s.is_some() && e.is_none() && p.is_none());
        assert_eq!(s.unwrap().1, 2);

        // Extension detail -> extensions panel selecting that id.
        let id = extension_entries()[2].id.clone();
        let (s, e, _p) = cockpit_panels_for(&WorkbenchTabId::ExtensionDetail(id), "Dark");
        assert!(s.is_none());
        assert_eq!(e.unwrap().1, 2);

        // A facet destination root -> placeholder; the editor -> nothing.
        let (_s, _e, p) = cockpit_panels_for(
            &WorkbenchTabId::DestinationRoot(WorkbenchDestination::Search),
            "Dark",
        );
        assert!(p.is_some());
        let (s, e, p) = cockpit_panels_for(&WorkbenchTabId::Editor, "Dark");
        assert!(s.is_none() && e.is_none() && p.is_none());
    }

    #[test]
    fn sidebar_selection_derives_from_active_tab() {
        let id = extension_entries()[1].id.clone();
        assert_eq!(sidebar_selection_for(&WorkbenchTabId::ExtensionDetail(id)), (Some(1), None));
        assert_eq!(sidebar_selection_for(&WorkbenchTabId::SettingsSection(3)), (None, Some(3)));
        assert_eq!(sidebar_selection_for(&WorkbenchTabId::Editor), (None, None));
    }

    #[test]
    fn tab_titles_are_human_readable() {
        assert_eq!(
            tab_title(&WorkbenchTabId::DestinationRoot(WorkbenchDestination::Extensions)),
            "Extensions"
        );
        assert!(tab_title(&WorkbenchTabId::SettingsSection(0)).starts_with("Settings: "));
        let id = extension_entries()[0].id.clone();
        assert_eq!(tab_title(&WorkbenchTabId::ExtensionDetail(id)), extension_entries()[0].name);
    }
}
